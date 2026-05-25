use crate::indicators::Indicator;
use crate::CircularBuffer;
use crate::IndicatorValue;

pub struct ChandeMomentumOscillator {
    buffer: CircularBuffer,
    previous_value: Option<IndicatorValue>,
    sum_up: IndicatorValue,
    sum_down: IndicatorValue,
}

impl ChandeMomentumOscillator {
    #[inline(always)]
    pub fn new(period: usize) -> Self {
        ChandeMomentumOscillator {
            buffer: CircularBuffer::new(period),
            previous_value: None,
            sum_up: IndicatorValue::from(0.0),
            sum_down: IndicatorValue::from(0.0),
        }
    }
}

impl Default for ChandeMomentumOscillator {
    #[inline(always)]
    fn default() -> Self {
        ChandeMomentumOscillator::new(14)
    }
}

impl Indicator for ChandeMomentumOscillator {
    type Input = IndicatorValue;
    type Output = Option<IndicatorValue>;

    #[inline(always)]
    fn next(&mut self, input: Self::Input) -> Self::Output {
        let difference = if let Some(prev) = self.previous_value {
            input - prev
        } else {
            self.previous_value = Some(input);
            IndicatorValue::from(0.0)
        };

        self.previous_value = Some(input);

        if difference > IndicatorValue::from(0.0) {
            self.sum_up += difference;
        } else if difference < IndicatorValue::from(0.0) {
            self.sum_down -= difference;
        }

        if self.buffer.is_full() {
            if let Some(removed_value) = self.buffer.push(difference) {
                if removed_value > IndicatorValue::from(0.0) {
                    self.sum_up -= removed_value;
                } else if removed_value < IndicatorValue::from(0.0) {
                    self.sum_down += removed_value;
                }
            }
        } else {
            self.buffer.push(difference);
        }

        if !self.buffer.is_full() {
            return None;
        }

        let total_sum = self.sum_up + self.sum_down;
        if total_sum == IndicatorValue::from(0.0) {
            Some(IndicatorValue::from(0.0))
        } else {
            Some((self.sum_up - self.sum_down) * IndicatorValue::from(100.0) / total_sum)
        }
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.buffer.clear();
        self.previous_value = None;
        self.sum_up = IndicatorValue::from(0.0);
        self.sum_down = IndicatorValue::from(0.0);
    }
}
