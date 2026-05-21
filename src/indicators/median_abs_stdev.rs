use crate::CircularBuffer;
use crate::indicators::Indicator;
use crate::IndicatorValue;

/// Median Absolute Deviation: median(|x_i − median(window)|) over a rolling window.
#[derive(Debug, Clone)]
pub struct MedianAbsoluteStandardDeviation {
    buffer: CircularBuffer,
    sorted: Vec<IndicatorValue>,
    is_even: bool,
    median_index: usize,
    median_index_1: usize,
}

impl MedianAbsoluteStandardDeviation {
    pub fn new(period: usize) -> Self {
        let median_index = period / 2;
        let is_even = period % 2 == 0;
        Self {
            buffer: CircularBuffer::new(period),
            sorted: Vec::with_capacity(period),
            is_even,
            median_index,
            median_index_1: median_index.saturating_sub(is_even as usize),
        }
    }

    #[inline]
    fn median_of(sorted: &[IndicatorValue], is_even: bool, mid: usize, mid_1: usize) -> IndicatorValue {
        if is_even {
            (sorted[mid_1] + sorted[mid]) / IndicatorValue::from(2.0)
        } else {
            sorted[mid]
        }
    }
}

impl Indicator for MedianAbsoluteStandardDeviation {
    type Output = Option<IndicatorValue>;
    type Input = IndicatorValue;

    fn next(&mut self, input: Self::Input) -> Self::Output {
        let old_value = self.buffer.push(input);

        if let Some(old) = old_value {
            if let Ok(pos) = self.sorted.binary_search(&old) {
                self.sorted.remove(pos);
            }
        }

        let pos = self.sorted.binary_search(&input).unwrap_or_else(|e| e);
        self.sorted.insert(pos, input);

        if !self.buffer.is_full() {
            return None;
        }

        let median = Self::median_of(&self.sorted, self.is_even, self.median_index, self.median_index_1);

        let mut abs_devs: Vec<IndicatorValue> = self.sorted.iter().map(|&x| (x - median).abs()).collect();
        abs_devs.sort_unstable();

        Some(Self::median_of(&abs_devs, self.is_even, self.median_index, self.median_index_1))
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.sorted.clear();
    }
}
