use std::iter::FusedIterator;
use crate::IndicatorValue;

#[derive(Debug, Clone)]
pub struct CircularBuffer {
    buffer: Vec<IndicatorValue>,
    index: usize,
    full: bool,
    capacity: usize,
    capacity_1: usize,
}

impl CircularBuffer {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        CircularBuffer {
            buffer: vec![IndicatorValue::from(0.0); capacity],
            index: 0,
            full: false,
            capacity,
            capacity_1: capacity.saturating_sub(1),
        }
    }

    #[inline]
    fn get_circular_index(&self, index: usize) -> usize {
        debug_assert!(index < self.len(), "index {index} out of bounds (len {})", self.len());

        let circular_index: usize = self.index + self.capacity_1 - index;

        // circular_index is at most 2*capacity - 2
        circular_index.wrapping_sub((circular_index >= self.capacity) as usize * self.capacity)
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&IndicatorValue> {
        if index >= self.len() {
            return None;
        }
        let buf_index = self.get_circular_index(index);
        self.buffer.get(buf_index)
    }

    #[inline]
    pub fn push(&mut self, value: IndicatorValue) -> Option<IndicatorValue> {
        let was_full = self.full;
        let old_value = std::mem::replace(&mut self.buffer[self.index], value);

        self.index += 1;
        if self.index == self.capacity {
            self.index = 0;
            self.full = true;
        }

        if was_full { Some(old_value) } else { None }
    }

    #[inline]
    pub fn len(&self) -> usize {
        if self.full { self.capacity } else { self.index }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.full
    }

    #[inline]
    pub fn clear(&mut self) {
        self.index = 0;
        self.full = false;
    }

    #[inline]
    pub fn iter(&self) -> CircularBufferIterator<'_> {
        CircularBufferIterator::new(self)
    }

    #[inline]
    pub fn iter_reversed(&self) -> ReversedCircularBufferIterator<'_> {
        ReversedCircularBufferIterator::new(self)
    }
}

#[derive(Copy, Clone)]
pub struct CircularBufferIterator<'a> {
    buffer: &'a CircularBuffer,
    index: usize,
    len: usize,
}

impl<'a> CircularBufferIterator<'a> {
    #[inline]
    pub fn new(buffer: &'a CircularBuffer) -> Self {
        CircularBufferIterator {
            buffer,
            index: 0,
            len: buffer.len(),
        }
    }
}

impl<'a> Iterator for CircularBufferIterator<'a> {
    type Item = &'a IndicatorValue;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }
        let value = self.buffer.get(self.index);
        self.index += 1;
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for CircularBufferIterator<'a> {}
impl<'a> FusedIterator for CircularBufferIterator<'a> {}

#[derive(Copy, Clone)]
pub struct ReversedCircularBufferIterator<'a> {
    buffer: &'a CircularBuffer,
    index: isize,
}

impl<'a> ReversedCircularBufferIterator<'a> {
    #[inline]
    pub fn new(buffer: &'a CircularBuffer) -> Self {
        ReversedCircularBufferIterator {
            buffer,
            index: (buffer.len() as isize) - 1,
        }
    }
}

impl<'a> Iterator for ReversedCircularBufferIterator<'a> {
    type Item = &'a IndicatorValue;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < 0 {
            return None;
        }
        let value = self.buffer.get(self.index as usize);
        self.index -= 1;
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.index + 1).max(0) as usize;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for ReversedCircularBufferIterator<'a> {}
impl<'a> FusedIterator for ReversedCircularBufferIterator<'a> {}
