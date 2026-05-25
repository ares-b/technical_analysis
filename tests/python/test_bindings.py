import math
import numpy as np
import pytest


def warmup(ind, n, *args):
    for _ in range(n):
        ind.next(*args)


def arr(*values):
    return np.array(values, dtype=np.float64)


class TestNonFiniteInputs:
    BAD = [float("nan"), float("inf"), float("-inf")]

    @pytest.mark.parametrize("bad", BAD)
    def test_single_input_rejects_non_finite(self, ta, bad):
        sma = ta.SimpleMovingAverage(3)
        with pytest.raises(ValueError):
            sma.next(bad)

    @pytest.mark.parametrize("bad", BAD)
    def test_multi_input_rejects_non_finite_in_any_slot(self, ta, bad):
        atr = ta.AverageTrueRange(3)
        with pytest.raises(ValueError):
            atr.next(bad, 1.0, 1.0)
        with pytest.raises(ValueError):
            atr.next(2.0, bad, 1.0)
        with pytest.raises(ValueError):
            atr.next(2.0, 1.0, bad)

    @pytest.mark.parametrize("bad", BAD)
    def test_next_chunk_rejects_non_finite(self, ta, bad):
        sma = ta.SimpleMovingAverage(3)
        with pytest.raises(ValueError):
            sma.next_chunk(arr(1.0, bad, 3.0))



class TestConstructorValidation:
    def test_period_zero_raises(self, ta):
        with pytest.raises(ValueError):
            ta.SimpleMovingAverage(0)

    def test_stdev_period_one_raises(self, ta):
        with pytest.raises(ValueError):
            ta.StandardDeviation(1)

    def test_stdev_period_two_accepted(self, ta):
        ta.StandardDeviation(2)  # must not raise

    def test_bb_multiplier_zero_raises(self, ta):
        with pytest.raises(ValueError):
            ta.BollingerBands(20, 0.0)

    def test_bb_multiplier_negative_raises(self, ta):
        with pytest.raises(ValueError):
            ta.BollingerBands(20, -1.0)

    def test_bb_fractional_multiplier_accepted(self, ta):
        ta.BollingerBands(20, 1.5)  # must not raise

    def test_keltner_multiplier_zero_raises(self, ta):
        with pytest.raises(ValueError):
            ta.KeltnerChannels(20, 10, 0.0)

    def test_keltner_fractional_multiplier_accepted(self, ta):
        ta.KeltnerChannels(20, 10, 2.5)  # must not raise

    def test_macd_inverted_periods_raises(self, ta):
        with pytest.raises(ValueError):
            ta.MovingAverageConvergenceDivergence(
                short_period=26, long_period=12, signal_period=9
            )

    def test_macd_equal_periods_raises(self, ta):
        with pytest.raises(ValueError):
            ta.MovingAverageConvergenceDivergence(
                short_period=12, long_period=12, signal_period=9
            )

    def test_ppo_inverted_periods_raises(self, ta):
        with pytest.raises(ValueError):
            ta.PercentagePriceOscillator(
                short_period=26, long_period=12, signal_period=9
            )

    def test_sar_max_lt_initial_raises(self, ta):
        with pytest.raises(ValueError):
            ta.ParabolicSAR(acceleration_factor=0.1, max_acceleration_factor=0.05)

    def test_sar_zero_acceleration_raises(self, ta):
        with pytest.raises(ValueError):
            ta.ParabolicSAR(acceleration_factor=0.0)

    def test_sar_non_finite_acceleration_raises(self, ta):
        with pytest.raises(ValueError):
            ta.ParabolicSAR(acceleration_factor=float("nan"))



class TestWarmup:
    def test_sma_returns_none_during_warmup(self, ta):
        sma = ta.SimpleMovingAverage(3)
        assert sma.next(1.0) is None
        assert sma.next(2.0) is None

    def test_sma_returns_value_after_warmup(self, ta):
        sma = ta.SimpleMovingAverage(3)
        sma.next(1.0)
        sma.next(2.0)
        result = sma.next(3.0)
        assert result is not None
        assert isinstance(result, float)

    def test_obv_never_returns_none(self, ta):
        obv = ta.OnBalanceVolume()
        result = obv.next(100.0, 1000.0)
        assert isinstance(result, float)

    def test_vwap_never_returns_none(self, ta):
        vwap = ta.VolumeWeightedAveragePrice()
        result = vwap.next(10.0, 9.0, 9.5, 500.0)
        assert isinstance(result, float)



class TestReset:
    def test_reset_restores_warmup(self, ta):
        sma = ta.SimpleMovingAverage(3)
        warmup(sma, 3, 1.0)
        assert sma.next(1.0) is not None

        sma.reset()

        assert sma.next(1.0) is None
        assert sma.next(1.0) is None
        assert sma.next(1.0) is not None

    def test_reset_clears_obv_accumulator(self, ta):
        obv = ta.OnBalanceVolume()
        obv.next(10.0, 1000.0)
        obv.next(11.0, 2000.0)
        v_before = obv.next(12.0, 500.0)

        obv.reset()
        v_after = obv.next(12.0, 500.0)

        assert v_before != v_after

    def test_reset_clears_vwap_accumulator(self, ta):
        vwap = ta.VolumeWeightedAveragePrice()
        vwap.next(10.0, 9.0, 9.5, 1000.0)
        v1 = vwap.next(20.0, 18.0, 19.0, 500.0)

        vwap.reset()
        v2 = vwap.next(20.0, 18.0, 19.0, 500.0)

        assert v1 != v2



class TestRepr:
    def test_sma_repr(self, ta):
        assert repr(ta.SimpleMovingAverage(5)) == "SimpleMovingAverage(period=5)"

    def test_sma_default_repr(self, ta):
        assert repr(ta.SimpleMovingAverage()) == "SimpleMovingAverage(period=20)"

    def test_ema_repr(self, ta):
        assert repr(ta.ExponentialMovingAverage(10)) == "ExponentialMovingAverage(period=10)"

    def test_bb_repr(self, ta):
        assert repr(ta.BollingerBands(20, 2.0)) == "BollingerBands(period=20, multiplier=2)"

    def test_bb_fractional_repr(self, ta):
        assert repr(ta.BollingerBands(20, 1.5)) == "BollingerBands(period=20, multiplier=1.5)"

    def test_macd_repr(self, ta):
        r = repr(ta.MovingAverageConvergenceDivergence(12, 26, 9))
        assert r == (
            "MovingAverageConvergenceDivergence("
            "short_period=12, long_period=26, signal_period=9)"
        )

    def test_obv_repr(self, ta):
        assert repr(ta.OnBalanceVolume()) == "OnBalanceVolume()"

    def test_vwap_repr(self, ta):
        assert repr(ta.VolumeWeightedAveragePrice()) == "VolumeWeightedAveragePrice()"

    def test_sar_repr(self, ta):
        r = repr(ta.ParabolicSAR(0.02, 0.2, 5))
        assert "ParabolicSAR(" in r
        assert "acceleration_factor=0.02" in r
        assert "max_acceleration_factor=0.2" in r
        assert "initialization_period=5" in r



class TestNextChunk:
    def test_chunk_length_matches_input(self, ta):
        sma = ta.SimpleMovingAverage(3)
        result = sma.next_chunk(arr(1.0, 2.0, 3.0, 4.0, 5.0))
        assert isinstance(result, np.ndarray)
        assert len(result) == 5

    def test_chunk_warmup_slots_are_nan(self, ta):
        sma = ta.SimpleMovingAverage(3)
        result = sma.next_chunk(arr(1.0, 2.0, 3.0, 4.0))
        assert math.isnan(result[0])
        assert math.isnan(result[1])
        assert not math.isnan(result[2])
        assert not math.isnan(result[3])

    def test_chunk_matches_sequential_next(self, ta):
        values = np.arange(1.0, 11.0)

        sma_seq = ta.SimpleMovingAverage(3)
        expected = [sma_seq.next(v) for v in values]

        sma_chunk = ta.SimpleMovingAverage(3)
        actual = sma_chunk.next_chunk(values)

        assert isinstance(actual, np.ndarray)
        assert len(actual) == len(expected)
        for got, exp in zip(actual, expected):
            if exp is None:
                assert math.isnan(got)
            else:
                assert got == pytest.approx(exp)

    def test_chunk_multi_input_matches_sequential(self, ta):
        highs  = np.array([10.0 + i for i in range(10)])
        lows   = np.array([ 9.0 + i for i in range(10)])
        closes = np.array([ 9.5 + i for i in range(10)])

        atr_seq = ta.AverageTrueRange(3)
        expected = [atr_seq.next(h, l, c) for h, l, c in zip(highs, lows, closes)]

        atr_chunk = ta.AverageTrueRange(3)
        actual = atr_chunk.next_chunk(highs, lows, closes)

        assert isinstance(actual, np.ndarray)
        for got, exp in zip(actual, expected):
            if exp is None:
                assert math.isnan(got)
            else:
                assert got == pytest.approx(exp)

    def test_chunk_length_mismatch_raises(self, ta):
        atr = ta.AverageTrueRange(3)
        with pytest.raises(ValueError):
            atr.next_chunk(arr(1.0, 2.0), arr(1.0), arr(1.0, 2.0))

    def test_chunk_obv_matches_sequential(self, ta):
        closes  = arr(10.0, 11.0, 10.5, 12.0, 11.0)
        volumes = arr(100.0, 200.0, 150.0, 300.0, 250.0)

        obv_seq = ta.OnBalanceVolume()
        expected = [obv_seq.next(c, v) for c, v in zip(closes, volumes)]

        obv_chunk = ta.OnBalanceVolume()
        actual = obv_chunk.next_chunk(closes, volumes)

        assert isinstance(actual, np.ndarray)
        np.testing.assert_array_almost_equal(actual, expected)

    def test_chunk_sar_length_mismatch_raises(self, ta):
        sar = ta.ParabolicSAR()
        with pytest.raises(ValueError):
            sar.next_chunk(arr(1.0, 2.0), arr(1.0))

    def test_chunk_bad_element_does_not_advance_state(self, ta):
        sma = ta.SimpleMovingAverage(3)
        warmup(sma, 3, 1.0)
        value_before = sma.next(1.0)

        sma2 = ta.SimpleMovingAverage(3)
        warmup(sma2, 3, 1.0)

        with pytest.raises(ValueError):
            sma2.next_chunk(arr(1.0, float("nan"), 1.0))

        assert sma2.next(1.0) == pytest.approx(value_before)



class TestOutputShape:
    def test_bollinger_returns_3tuple(self, ta):
        bb = ta.BollingerBands(3, 2.0)
        warmup(bb, 2, 10.0)
        out = bb.next(10.0)
        assert out is not None
        upper, middle, lower = out
        assert isinstance(upper, float)
        assert isinstance(middle, float)
        assert isinstance(lower, float)
        assert upper >= middle >= lower

    def test_bollinger_chunk_returns_3_arrays(self, ta):
        bb = ta.BollingerBands(3, 2.0)
        upper, middle, lower = bb.next_chunk(arr(10.0, 11.0, 12.0, 13.0))
        assert isinstance(upper, np.ndarray)
        assert len(upper) == 4
        # first two are NaN (warm-up), last two are valid
        assert math.isnan(upper[0]) and math.isnan(upper[1])
        assert not math.isnan(upper[2])

    def test_macd_returns_3tuple(self, ta):
        macd = ta.MovingAverageConvergenceDivergence()
        out = None
        for v in range(1, 80):
            out = macd.next(float(v))
        assert out is not None
        assert len(out) == 3
        assert all(isinstance(x, float) for x in out)

    def test_aroon_returns_2tuple(self, ta):
        aroon = ta.Aroon(3)
        warmup(aroon, 3, 10.0, 9.0)
        out = aroon.next(10.0, 9.0)
        assert out is not None
        assert len(out) == 2

    def test_aroon_chunk_returns_2_arrays(self, ta):
        aroon = ta.Aroon(3)
        highs = np.arange(10.0, 18.0)
        lows  = highs - 1.0
        up, down = aroon.next_chunk(highs, lows)
        assert isinstance(up, np.ndarray) and isinstance(down, np.ndarray)
        assert len(up) == len(highs)

    def test_stochastic_returns_2tuple(self, ta):
        stoch = ta.StochasticOscillator(3, 3)
        warmup(stoch, 5, 10.0, 9.0, 9.5)
        out = stoch.next(10.0, 9.0, 9.5)
        assert out is not None
        assert len(out) == 2

    def test_donchian_returns_3tuple(self, ta):
        dc = ta.DonchianChannels(3)
        warmup(dc, 2, 10.0, 9.0)
        out = dc.next(10.0, 9.0)
        assert out is not None
        assert len(out) == 3

    def test_keltner_returns_3tuple(self, ta):
        kc = ta.KeltnerChannels(3, 3, 2.0)
        warmup(kc, 3, 10.0, 9.0, 9.5)
        out = kc.next(10.0, 9.0, 9.5)
        assert out is not None
        assert len(out) == 3

    def test_highlow_returns_2tuple_of_floats(self, ta):
        hl = ta.HighLow(3)
        warmup(hl, 2, 10.0)
        out = hl.next(10.0)
        assert out is not None
        high, low = out
        assert isinstance(high, float)
        assert isinstance(low, float)
        assert high >= low

    def test_high_returns_scalar_float(self, ta):
        h = ta.High(3)
        warmup(h, 2, 10.0)
        out = h.next(10.0)
        assert out is not None
        assert isinstance(out, float)

    def test_low_returns_scalar_float(self, ta):
        l = ta.Low(3)
        warmup(l, 2, 10.0)
        out = l.next(10.0)
        assert out is not None
        assert isinstance(out, float)

    def test_vwap_returns_float(self, ta):
        vwap = ta.VolumeWeightedAveragePrice()
        assert isinstance(vwap.next(10.0, 8.0, 9.0, 100.0), float)

    def test_obv_returns_float(self, ta):
        obv = ta.OnBalanceVolume()
        assert isinstance(obv.next(10.0, 1000.0), float)

    def test_chunk_returns_ndarray_dtype_float64(self, ta):
        sma = ta.SimpleMovingAverage(3)
        result = sma.next_chunk(np.arange(1.0, 6.0))
        assert result.dtype == np.float64
