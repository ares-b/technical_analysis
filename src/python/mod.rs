// pyo3's PyErr implements From<PyErr> (identity), which causes clippy to flag
// every `?` propagation in a PyResult-returning function as "useless_conversion".
// This is a known false positive when using pyo3.
#![allow(clippy::useless_conversion)]

// Python only exposes `float` (f64), so using `precision` (rust_decimal::Decimal)
// at the Python boundary throws away all precision while adding ~10× overhead.
#[cfg(feature = "precision")]
compile_error!(
    "The `precision` and `python` features cannot be combined: Python only handles f64, \
     so Decimal arithmetic is both lost at the boundary and significantly slower. \
     Build with `--features python` only."
);

use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::indicators::{
    Aroon, AverageTrueRange, BollingerBands, ChaikinMoneyFlow, ChandeMomentumOscillator,
    CommodityChannelIndex, DonchianChannels, ExponentialMovingAverage, High, HighLow, Indicator,
    KeltnerChannels, Low, MeanAbsDev, MedianAbsoluteStandardDeviation,
    MovingAverageConvergenceDivergence, OnBalanceVolume, ParabolicSAR, PercentagePriceOscillator,
    RateOfChange, RelativeStrengthIndex, SimpleMovingAverage, SimpleMovingMedian,
    StandardDeviation, StochasticOscillator, VolumeWeightedAveragePrice,
};
use crate::IndicatorValue;

#[inline]
fn f64_to_indicator_value(name: &str, value: f64) -> PyResult<IndicatorValue> {
    if !value.is_finite() {
        return Err(PyValueError::new_err(format!("{name} must be finite")));
    }
    Ok(IndicatorValue::from(value))
}

#[inline]
fn uniform_slice_len(lengths: &[usize]) -> PyResult<usize> {
    if lengths.windows(2).any(|w| w[0] != w[1]) {
        return Err(PyValueError::new_err("all input slices must have the same length"));
    }
    Ok(lengths.first().copied().unwrap_or(0))
}

macro_rules! validated_input {
    ($n:ident) => { f64_to_indicator_value(stringify!($n), $n)? };
    ($($n:ident),+ $(,)?) => { ($(f64_to_indicator_value(stringify!($n), $n)?),+) };
}

macro_rules! validated_input_at {
    ($idx:ident, $n:ident) => { f64_to_indicator_value(stringify!($n), $n[$idx])? };
    ($idx:ident, $($n:ident),+) => { ($(f64_to_indicator_value(stringify!($n), $n[$idx])?),+) };
}

macro_rules! validate_ctor_arg {
    (positive($n:ident)) => {
        if $n == 0 { return Err(PyValueError::new_err(format!("{} must be > 0", stringify!($n)))); }
    };
    (min($n:ident, $min:expr)) => {
        if $n < $min { return Err(PyValueError::new_err(format!("{} must be >= {}", stringify!($n), $min))); }
    };
    (lt($a:ident, $b:ident)) => {      // enforces $a < $b
        if $a >= $b { return Err(PyValueError::new_err(format!("{} must be < {}", stringify!($a), stringify!($b)))); }
    };
    (le($a:ident, $b:ident)) => {      // enforces $a <= $b
        if $a > $b { return Err(PyValueError::new_err(format!("{} must be <= {}", stringify!($a), stringify!($b)))); }
    };
    (positive_float($n:ident)) => {
        f64_to_indicator_value(stringify!($n), $n)?;  // validates finite + range
        if $n <= 0.0 { return Err(PyValueError::new_err(format!("{} must be > 0", stringify!($n)))); }
    };
}

macro_rules! format_repr {
    ($name:literal) => { concat!($name, "()").to_owned() };
    ($name:literal, $($k:ident = $v:expr),+ $(,)?) => {{
        let mut parts: Vec<String> = Vec::new();
        $(parts.push(format!("{}={}", stringify!($k), $v));)+
        format!("{}({})", $name, parts.join(", "))
    }};
}

// Must be in expression position - pyo3 bans item-position macros inside #[pymethods].
macro_rules! convert_indicator_output {
    (option, $expr:expr, $output:ident, $mapped:expr) => { Ok($expr.map(|$output| $mapped)) };
    (value,  $expr:expr, $output:ident, $mapped:expr) => {{ let $output = $expr; Ok($mapped) }};
}

// Infallible variant for use inside next_chunk after all inputs are validated.
macro_rules! map_indicator_output {
    (option, $expr:expr, $output:ident, $mapped:expr) => { $expr.map(|$output| $mapped) };
    (value,  $expr:expr, $output:ident, $mapped:expr) => {{ let $output = $expr; $mapped }};
}

// None -> NaN so warm-up slots are directly usable in numpy without .fillna().
trait IntoNumpyChunk: Sized {
    type Out;
    fn accumulate(iter: impl Iterator<Item = Self>, len: usize, py: Python<'_>) -> Self::Out;
}

impl IntoNumpyChunk for Option<f64> {
    type Out = Py<PyArray1<f64>>;
    fn accumulate(iter: impl Iterator<Item = Self>, len: usize, py: Python<'_>) -> Self::Out {
        let mut buf = Vec::with_capacity(len);
        for v in iter { buf.push(v.unwrap_or(f64::NAN)); }
        PyArray1::from_vec_bound(py, buf).unbind()
    }
}

impl IntoNumpyChunk for f64 {
    type Out = Py<PyArray1<f64>>;
    fn accumulate(iter: impl Iterator<Item = Self>, len: usize, py: Python<'_>) -> Self::Out {
        let mut buf = Vec::with_capacity(len);
        buf.extend(iter);
        PyArray1::from_vec_bound(py, buf).unbind()
    }
}

impl IntoNumpyChunk for Option<(f64, f64)> {
    type Out = (Py<PyArray1<f64>>, Py<PyArray1<f64>>);
    fn accumulate(iter: impl Iterator<Item = Self>, len: usize, py: Python<'_>) -> Self::Out {
        let mut a = Vec::with_capacity(len);
        let mut b = Vec::with_capacity(len);
        for v in iter {
            match v {
                Some((x, y)) => { a.push(x); b.push(y); }
                None         => { a.push(f64::NAN); b.push(f64::NAN); }
            }
        }
        (PyArray1::from_vec_bound(py, a).unbind(), PyArray1::from_vec_bound(py, b).unbind())
    }
}

impl IntoNumpyChunk for Option<(f64, f64, f64)> {
    type Out = (Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>);
    fn accumulate(iter: impl Iterator<Item = Self>, len: usize, py: Python<'_>) -> Self::Out {
        let mut a = Vec::with_capacity(len);
        let mut b = Vec::with_capacity(len);
        let mut c = Vec::with_capacity(len);
        for v in iter {
            match v {
                Some((x, y, z)) => { a.push(x); b.push(y); c.push(z); }
                None            => { a.push(f64::NAN); b.push(f64::NAN); c.push(f64::NAN); }
            }
        }
        (
            PyArray1::from_vec_bound(py, a).unbind(),
            PyArray1::from_vec_bound(py, b).unbind(),
            PyArray1::from_vec_bound(py, c).unbind(),
        )
    }
}

macro_rules! impl_py_indicator {
    ($W:ident, $N:literal, $I:ty,
     ctor($($a:ident : $T:ty = $d:expr),+ $(,)?) validate[$($v:ident($($va:tt)*)),* $(,)?];
     $kw:ident next($($inp:ident),+ $(,)?) -> $R:ty => |$output:ident| $mapped:expr;
     $($x:item)*
    ) => {
        #[pyclass(module = "technical_analysis", name = $N)]
        struct $W { inner: $I, repr: String }

        #[pymethods]
        impl $W {
            #[new]
            #[pyo3(signature = ($($a = $d),+))]
            fn new($($a: $T),+) -> PyResult<Self> {
                $(validate_ctor_arg!($v($($va)*));)*
                Ok(Self { inner: <$I>::new($($a),+), repr: format_repr!($N, $($a = $a),+) })
            }
            fn next(&mut self, $($inp: f64),+) -> PyResult<$R> {
                let input = validated_input!($($inp),+);
                convert_indicator_output!($kw, self.inner.next(input), $output, $mapped)
            }
            fn next_chunk(
                &mut self,
                py: Python<'_>,
                $($inp: PyReadonlyArray1<'_, f64>),+
            ) -> PyResult<<$R as IntoNumpyChunk>::Out> {
                // Zero-copy slice access; fails if the array is not contiguous.
                $(let $inp = $inp.as_slice().map_err(|_| PyValueError::new_err(
                    concat!("'", stringify!($inp), "' must be a contiguous array")
                ))?;)+
                let len = uniform_slice_len(&[$($inp.len()),+])?;

                // Validate all inputs before touching internal state (all-or-nothing).
                let validated: Vec<_> = (0..len)
                    .map(|idx| -> PyResult<_> { Ok(validated_input_at!(idx, $($inp),+)) })
                    .collect::<PyResult<_>>()?;

                let results = validated.into_iter()
                    .map(|input| map_indicator_output!($kw, self.inner.next(input), $output, $mapped));

                Ok(<$R as IntoNumpyChunk>::accumulate(results, len, py))
            }
            fn reset(&mut self) { self.inner.reset(); }
            fn __repr__(&self) -> &str { &self.repr }
            $($x)*
        }
    };

    ($W:ident, $N:literal, $I:ty,
     ctor();
     $kw:ident next($($inp:ident),+ $(,)?) -> $R:ty => |$output:ident| $mapped:expr;
     $($x:item)*
    ) => {
        #[pyclass(module = "technical_analysis", name = $N)]
        struct $W { inner: $I }

        #[pymethods]
        impl $W {
            #[new]
            fn new() -> Self { Self { inner: <$I>::new() } }
            fn next(&mut self, $($inp: f64),+) -> PyResult<$R> {
                let input = validated_input!($($inp),+);
                convert_indicator_output!($kw, self.inner.next(input), $output, $mapped)
            }
            fn next_chunk(
                &mut self,
                py: Python<'_>,
                $($inp: PyReadonlyArray1<'_, f64>),+
            ) -> PyResult<<$R as IntoNumpyChunk>::Out> {
                $(let $inp = $inp.as_slice().map_err(|_| PyValueError::new_err(
                    concat!("'", stringify!($inp), "' must be a contiguous array")
                ))?;)+
                let len = uniform_slice_len(&[$($inp.len()),+])?;

                let validated: Vec<_> = (0..len)
                    .map(|idx| -> PyResult<_> { Ok(validated_input_at!(idx, $($inp),+)) })
                    .collect::<PyResult<_>>()?;

                let results = validated.into_iter()
                    .map(|input| map_indicator_output!($kw, self.inner.next(input), $output, $mapped));

                Ok(<$R as IntoNumpyChunk>::accumulate(results, len, py))
            }
            fn reset(&mut self) { self.inner.reset(); }
            fn __repr__(&self) -> String { format_repr!($N) }
            $($x)*
        }
    };
}

impl_py_indicator!(
    PySimpleMovingAverage, "SimpleMovingAverage", SimpleMovingAverage,
    ctor(period: usize = 20) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyExponentialMovingAverage, "ExponentialMovingAverage", ExponentialMovingAverage,
    ctor(period: usize = 20) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.to_f64();

    fn set_ema(&mut self, value: f64) -> PyResult<()> {
        self.inner.set_ema(f64_to_indicator_value("value", value)?);
        Ok(())
    }
);

impl_py_indicator!(
    PySimpleMovingMedian, "SimpleMovingMedian", SimpleMovingMedian,
    ctor(period: usize = 20) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyRelativeStrengthIndex, "RelativeStrengthIndex", RelativeStrengthIndex,
    ctor(period: usize = 14) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyStandardDeviation, "StandardDeviation", StandardDeviation,
    ctor(period: usize = 20) validate[min(period, 2)];
    option next(value) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyMeanAbsDev, "MeanAbsDev", MeanAbsDev,
    ctor(period: usize = 20) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyMedianAbsoluteStandardDeviation, "MedianAbsoluteStandardDeviation",
    MedianAbsoluteStandardDeviation,
    ctor(period: usize = 20) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyRateOfChange, "RateOfChange", RateOfChange,
    ctor(period: usize = 12) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyChandeMomentumOscillator, "ChandeMomentumOscillator", ChandeMomentumOscillator,
    ctor(period: usize = 14) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyOnBalanceVolume, "OnBalanceVolume", OnBalanceVolume,
    ctor();
    value next(close, volume) -> f64 => |out| out.to_f64();
);

impl_py_indicator!(
    PyAverageTrueRange, "AverageTrueRange", AverageTrueRange,
    ctor(period: usize = 14) validate[positive(period)];
    option next(high, low, close) -> Option<f64> => |out| out.to_f64();
);

// multiplier is f64 to allow fractional widths (e.g. 1.5σ, 2.5σ).
impl_py_indicator!(
    PyBollingerBands, "BollingerBands", BollingerBands,
    ctor(period: usize = 20, multiplier: f64 = 2.0) validate[positive(period), positive_float(multiplier)];
    option next(value) -> Option<(f64, f64, f64)>
        => |out| (out.upper_band.to_f64(), out.middle_band.to_f64(), out.lower_band.to_f64());
);

impl_py_indicator!(
    PyMovingAverageConvergenceDivergence, "MovingAverageConvergenceDivergence",
    MovingAverageConvergenceDivergence,
    ctor(short_period: usize = 12, long_period: usize = 26, signal_period: usize = 9)
        validate[positive(short_period), positive(long_period), positive(signal_period), lt(short_period, long_period)];
    option next(value) -> Option<(f64, f64, f64)>
        => |out| (out.macd_value.to_f64(), out.signal_value.to_f64(), out.histogram_value.to_f64());
);

impl_py_indicator!(
    PyPercentagePriceOscillator, "PercentagePriceOscillator", PercentagePriceOscillator,
    ctor(short_period: usize = 12, long_period: usize = 26, signal_period: usize = 9)
        validate[positive(short_period), positive(long_period), positive(signal_period), lt(short_period, long_period)];
    option next(value) -> Option<(f64, f64, f64)>
        => |out| (out.ppo_value.to_f64(), out.signal_value.to_f64(), out.histogram_value.to_f64());
);

impl_py_indicator!(
    PyAroon, "Aroon", Aroon,
    ctor(period: usize = 14) validate[positive(period)];
    option next(high, low) -> Option<(f64, f64)>
        => |out| (out.aroon_up.to_f64(), out.aroon_down.to_f64());
);

impl_py_indicator!(
    PyParabolicSAR, "ParabolicSAR", ParabolicSAR,
    ctor(
        acceleration_factor: f64 = 0.02,
        max_acceleration_factor: f64 = 0.2,
        initialization_period: usize = 5,
    ) validate[
        positive_float(acceleration_factor),
        positive_float(max_acceleration_factor),
        positive(initialization_period),
        le(acceleration_factor, max_acceleration_factor)
    ];
    option next(high, low) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyStochasticOscillator, "StochasticOscillator", StochasticOscillator,
    ctor(period: usize = 14, d_period: usize = 3) validate[positive(period), positive(d_period)];
    option next(high, low, close) -> Option<(f64, f64)> => |out| (out.k.to_f64(), out.d.to_f64());
);

impl_py_indicator!(
    PyDonchianChannels, "DonchianChannels", DonchianChannels,
    ctor(period: usize = 20) validate[positive(period)];
    option next(high, low) -> Option<(f64, f64, f64)>
        => |out| (out.upper_band.to_f64(), out.middle_band.to_f64(), out.lower_band.to_f64());
);

impl_py_indicator!(
    PyKeltnerChannels, "KeltnerChannels", KeltnerChannels,
    ctor(ema_period: usize = 20, atr_period: usize = 10, multiplier: f64 = 2.0)
        validate[positive(ema_period), positive(atr_period), positive_float(multiplier)];
    option next(high, low, close) -> Option<(f64, f64, f64)>
        => |out| (out.upper_band.to_f64(), out.middle_band.to_f64(), out.lower_band.to_f64());
);

impl_py_indicator!(
    PyCommodityChannelIndex, "CommodityChannelIndex", CommodityChannelIndex,
    ctor(period: usize = 20) validate[positive(period)];
    option next(high, low, close) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyChaikinMoneyFlow, "ChaikinMoneyFlow", ChaikinMoneyFlow,
    ctor(period: usize = 20) validate[positive(period)];
    option next(high, low, close, volume) -> Option<f64> => |out| out.to_f64();
);

impl_py_indicator!(
    PyVolumeWeightedAveragePrice, "VolumeWeightedAveragePrice", VolumeWeightedAveragePrice,
    ctor();
    value next(high, low, close, volume) -> f64 => |out| out.to_f64();
);

// High/Low/HighLow only expose the value, not the internal index position.
impl_py_indicator!(
    PyHigh, "High", High,
    ctor(period: usize = 20) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.high_value.to_f64();
);

impl_py_indicator!(
    PyLow, "Low", Low,
    ctor(period: usize = 20) validate[positive(period)];
    option next(value) -> Option<f64> => |out| out.low_value.to_f64();
);

impl_py_indicator!(
    PyHighLow, "HighLow", HighLow,
    ctor(period: usize = 20) validate[positive(period)];
    option next(value) -> Option<(f64, f64)>
        => |out| (out.high_value.to_f64(), out.low_value.to_f64());
);

#[pymodule]
fn technical_analysis(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySimpleMovingAverage>()?;
    m.add_class::<PyExponentialMovingAverage>()?;
    m.add_class::<PySimpleMovingMedian>()?;
    m.add_class::<PyRelativeStrengthIndex>()?;
    m.add_class::<PyStandardDeviation>()?;
    m.add_class::<PyMeanAbsDev>()?;
    m.add_class::<PyMedianAbsoluteStandardDeviation>()?;
    m.add_class::<PyRateOfChange>()?;
    m.add_class::<PyChandeMomentumOscillator>()?;
    m.add_class::<PyOnBalanceVolume>()?;
    m.add_class::<PyAverageTrueRange>()?;
    m.add_class::<PyBollingerBands>()?;
    m.add_class::<PyMovingAverageConvergenceDivergence>()?;
    m.add_class::<PyPercentagePriceOscillator>()?;
    m.add_class::<PyAroon>()?;
    m.add_class::<PyParabolicSAR>()?;
    m.add_class::<PyStochasticOscillator>()?;
    m.add_class::<PyDonchianChannels>()?;
    m.add_class::<PyKeltnerChannels>()?;
    m.add_class::<PyCommodityChannelIndex>()?;
    m.add_class::<PyChaikinMoneyFlow>()?;
    m.add_class::<PyVolumeWeightedAveragePrice>()?;
    m.add_class::<PyHigh>()?;
    m.add_class::<PyLow>()?;
    m.add_class::<PyHighLow>()?;
    Ok(())
}
