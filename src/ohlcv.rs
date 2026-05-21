use crate::IndicatorValue;

#[derive(Clone, Copy, Debug)]
pub struct OHLCV {
    pub ts: u64,
    pub open: IndicatorValue,
    pub high: IndicatorValue,
    pub low: IndicatorValue,
    pub close: IndicatorValue,
    pub volume: IndicatorValue,
}
