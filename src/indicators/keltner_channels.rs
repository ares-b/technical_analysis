use crate::indicators::{AverageTrueRange, ExponentialMovingAverage, Indicator};
use crate::IndicatorValue;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeltnerChannelsOutput {
    pub middle_band: IndicatorValue,
    pub upper_band: IndicatorValue,
    pub lower_band: IndicatorValue,
}

pub struct KeltnerChannels {
    ema: ExponentialMovingAverage,
    atr: AverageTrueRange,
    multiplier: IndicatorValue,
}

impl KeltnerChannels {
    pub fn new(ema_period: usize, atr_period: usize, multiplier: f64) -> Self {
        KeltnerChannels {
            ema: ExponentialMovingAverage::new(ema_period),
            atr: AverageTrueRange::new(atr_period),
            multiplier: IndicatorValue::from(multiplier),
        }
    }
}

impl Default for KeltnerChannels {
    fn default() -> Self {
        KeltnerChannels::new(20, 10, 2.0)
    }
}

impl Indicator for KeltnerChannels {
    type Input = (IndicatorValue, IndicatorValue, IndicatorValue);
    type Output = Option<KeltnerChannelsOutput>;

    #[inline]
    fn next(&mut self, input: Self::Input) -> Self::Output {
        let (_, _, close) = input;

        let next_ema = self.ema.next(close);
        let next_atr = self.atr.next(input);

        if next_ema.is_none() || next_atr.is_none() {
            return None;
        };

        Some(KeltnerChannelsOutput {
            middle_band: next_ema.unwrap(),
            upper_band: next_ema.unwrap() + self.multiplier * next_atr.unwrap(),
            lower_band: next_ema.unwrap() - self.multiplier * next_atr.unwrap(),
        })
    }

    #[inline]
    fn reset(&mut self) {
        self.ema.reset();
        self.atr.reset();
    }
}
