use crate::indicators::Indicator;
use crate::CircularBuffer;
use crate::IndicatorValue;

pub struct ChaikinMoneyFlow {
    buffer_mfv: CircularBuffer,
    buffer_volume: CircularBuffer,
    running_sum_mfv: IndicatorValue,
    running_sum_volume: IndicatorValue,
}

impl ChaikinMoneyFlow {
    #[inline]
    pub fn new(period: usize) -> Self {
        Self {
            buffer_mfv: CircularBuffer::new(period),
            buffer_volume: CircularBuffer::new(period),
            running_sum_mfv: 0.0.into(),
            running_sum_volume: 0.0.into(),
        }
    }
}

impl Default for ChaikinMoneyFlow {
    #[inline]
    fn default() -> Self {
        Self::new(20)
    }
}

impl Indicator for ChaikinMoneyFlow {
    type Input = (
        IndicatorValue,
        IndicatorValue,
        IndicatorValue,
        IndicatorValue,
    );
    type Output = Option<IndicatorValue>;

    #[inline]
    fn next(&mut self, input: Self::Input) -> Self::Output {
        let (high, low, close, volume) = input;

        let mfv = if high == low {
            IndicatorValue::from(0.0)
        } else {
            ((close - low) - (high - close)) / (high - low) * volume
        };

        if let Some(old_mfv) = self.buffer_mfv.push(mfv) {
            self.running_sum_mfv += mfv - old_mfv;
        } else {
            self.running_sum_mfv += mfv;
        }

        if let Some(old_volume) = self.buffer_volume.push(volume) {
            self.running_sum_volume += volume - old_volume;
        } else {
            self.running_sum_volume += volume;
        }

        if !self.buffer_mfv.is_full() {
            return None;
        }

        if self.running_sum_volume == 0.0.into() {
            return None;
        }

        Some(self.running_sum_mfv / self.running_sum_volume)
    }

    #[inline]
    fn reset(&mut self) {
        self.buffer_mfv.clear();
        self.buffer_volume.clear();
        self.running_sum_mfv = 0.0.into();
        self.running_sum_volume = 0.0.into();
    }
}
