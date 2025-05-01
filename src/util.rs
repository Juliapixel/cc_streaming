use std::collections::VecDeque;

/// blud yields items only if it's able to do a number that's a multiple of 8
#[derive(Debug, Default, Clone)]
pub struct EightIter<T> {
    buffer: VecDeque<T>,
    yielded: usize,
}

impl<T> EightIter<T> {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            yielded: 0,
        }
    }

    pub fn feed_iter(&mut self, iter: impl IntoIterator<Item = T>) {
        self.buffer.extend(iter);
    }
}

impl<T> Iterator for EightIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.len() < 8 {
            if self.buffer.len() >= 8 - (self.yielded % 8) {
                match self.buffer.pop_front() {
                    Some(p) => {
                        self.yielded += 1;
                        Some(p)
                    }
                    None => None,
                }
            } else {
                None
            }
        } else {
            match self.buffer.pop_front() {
                Some(p) => {
                    self.yielded += 1;
                    Some(p)
                }
                None => None,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::EightIter;

    #[test]
    fn eightiter_inexact() {
        let mut eight = EightIter::new();
        eight.feed_iter([0; 7].as_slice().iter());

        assert_eq!(eight.next(), None);
        eight.feed_iter([1; 1].as_slice().iter());
        for _ in 0..7 {
            assert_eq!(eight.next(), Some(&0));
        }
        assert_eq!(eight.next(), Some(&1));

        eight.feed_iter([2; 9].as_slice().iter());
        for _ in 0..8 {
            assert_eq!(eight.next(), Some(&2));
        }
        assert_eq!(eight.next(), None);
    }

    #[test]
    fn eightiter_exact() {
        let mut eight = EightIter::new();
        eight.feed_iter([0; 8].iter());
        for _ in 0..8 {
            assert_eq!(eight.next(), Some(&0));
        }
        assert_eq!(eight.next(), None);
    }
}
