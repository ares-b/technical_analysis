# technical_analysis

Technical indicators for Rust

## Usage

```toml
[dependencies]
technical_analysis = { path = "..." }

# decimal precision instead of f64:
technical_analysis = { path = "...", features = ["precision"] }
```

```rust
use technical_analysis::indicators::{Indicator, SimpleMovingAverage, RelativeStrengthIndex};
use technical_analysis::IndicatorValue;

let mut sma = SimpleMovingAverage::new(20);
let mut rsi = RelativeStrengthIndex::new(14);

for price in prices {
    let v = IndicatorValue::from(price);
    if let Some(s) = sma.next(v) {
        println!("SMA: {}", s.to_f64());
    }
    if let Some(r) = rsi.next(v) {
        println!("RSI: {}", r.to_f64());
    }
}
```

Indicators return `None` until warmed up, then `Some` on every call after that.

`next_chunk` feeds a whole slice and returns the output of the last bar. Call `reset()` to clear state without re-allocating.

```rust
let result = sma.next_chunk(&candles);
```

## Python bindings (PyO3)

This crate includes optional Python bindings behind the `python` feature.

Build/install in a virtualenv with `maturin`:

```bash
pip install maturin
maturin develop --features python
```

Then from Python:

```python
import technical_analysis as ta

sma = ta.SimpleMovingAverage(20)
print(sma.next(100.0))
```

## Indicators

| Indicator | Input | Output |
|---|---|---|
| `SimpleMovingAverage` | price | `Option<f>` |
| `ExponentialMovingAverage` | price | `Option<f>` |
| `SimpleMovingMedian` | price | `Option<f>` |
| `RelativeStrengthIndex` | price | `Option<f>` |
| `StandardDeviation` | price | `Option<f>` |
| `MeanAbsDev` | price | `Option<f>` |
| `MedianAbsoluteStandardDeviation` | price | `Option<f>` |
| `RateOfChange` | price | `Option<f>` |
| `ChandeMomentumOscillator` | price | `Option<f>` |
| `OnBalanceVolume` | (close, volume) | `f` |
| `AverageTrueRange` | (high, low, close) | `Option<f>` |
| `BollingerBands` | price | `Option<(upper, mid, lower)>` |
| `MovingAverageConvergenceDivergence` | price | `Option<(macd, signal, hist)>` |
| `PercentagePriceOscillator` | price | `Option<(ppo, signal, hist)>` |
| `Aroon` | (high, low) | `Option<(up, down)>` |
| `ParabolicSAR` | (high, low) | `Option<f>` |
| `StochasticOscillator` | (high, low, close) | `Option<(k, d)>` |
| `DonchianChannels` | (high, low) | `Option<(upper, mid, lower)>` |
| `KeltnerChannels` | (high, low, close) | `Option<(upper, mid, lower)>` |
| `CommodityChannelIndex` | (high, low, close) | `Option<f>` |
| `ChaikinMoneyFlow` | (high, low, close, volume) | `Option<f>` |
| `VolumeWeightedAveragePrice` | (high, low, close, volume) | `f` |
| `High` / `Low` | price | `Option<f>` |
| `HighLow` | price | `Option<(high, low)>` |

## The `Indicator` trait

```rust
pub trait Indicator {
    type Input: Copy;
    type Output;

    fn next(&mut self, input: Self::Input) -> Self::Output;
    fn next_chunk(&mut self, input: &[Self::Input]) -> Self::Output;
    fn reset(&mut self);
}
```

## Precision

`IndicatorValue` wraps `f64` by default. The `precision` feature swaps it for `rust_decimal::Decimal`, same API with exact decimal arithmetic. Useful if you're working with fixed tick sizes.

```toml
technical_analysis = { ..., features = ["precision"] }
```

## Performance

Benched on a Ryzen 7 5800X, single core, one `next()` call per iteration.

```
test bench_circular_buffer_get           ... bench:           0.22 ns/iter (+/- 0.03)
test bench_circular_buffer_iter          ... bench:           0.23 ns/iter (+/- 0.09)
test bench_circular_buffer_iter_reversed ... bench:           0.44 ns/iter (+/- 0.02)
test bench_circular_buffer_push          ... bench:           0.89 ns/iter (+/- 0.09)
test bench_obv                           ... bench:           0.92 ns/iter (+/- 0.13)
test bench_roc                           ... bench:           1.14 ns/iter (+/- 0.11)
test bench_smm                           ... bench:           1.15 ns/iter (+/- 0.21)
test bench_sma                           ... bench:           1.17 ns/iter (+/- 0.29)
test bench_vwap                          ... bench:           1.17 ns/iter (+/- 0.12)
test bench_ema                           ... bench:           1.31 ns/iter (+/- 0.13)
test bench_atr                           ... bench:           1.77 ns/iter (+/- 0.14)
test bench_macd                          ... bench:           2.23 ns/iter (+/- 0.68)
test bench_stdev                         ... bench:           2.32 ns/iter (+/- 0.25)
test bench_parabolic_sar                 ... bench:           2.63 ns/iter (+/- 0.28)
test bench_ppo                           ... bench:           2.82 ns/iter (+/- 0.43)
test bench_rsi                           ... bench:           2.93 ns/iter (+/- 0.61)
test bench_cmf                           ... bench:           3.01 ns/iter (+/- 0.31)
test high                                ... bench:           3.07 ns/iter (+/- 0.20)
test low                                 ... bench:           3.23 ns/iter (+/- 0.27)
test bench_keltner_channels              ... bench:           3.42 ns/iter (+/- 0.61)
test bench_cmo                           ... bench:           4.14 ns/iter (+/- 1.44)
test bench_bollinger_bands               ... bench:           4.24 ns/iter (+/- 1.14)
test bench_donchian_channels             ... bench:           5.97 ns/iter (+/- 0.31)
test high_low                            ... bench:           6.02 ns/iter (+/- 0.42)
test bench_aroon                         ... bench:           6.06 ns/iter (+/- 0.52)
test bench_mean_abs_dev                  ... bench:           9.50 ns/iter (+/- 1.73)
test bench_stochastic_oscillator         ... bench:          13.40 ns/iter (+/- 1.30)
test bench_woodies_cci                   ... bench:          13.74 ns/iter (+/- 5.03)
test bench_median_abs_dev                ... bench:          86.16 ns/iter (+/- 6.45)
```
