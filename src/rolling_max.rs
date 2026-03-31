use std::{collections::VecDeque, num::NonZeroUsize};

// TODO: DOCS

#[derive(Debug)]
pub struct RollingMax<T> {
    // Invariant: These are 1:1. They are conceptually a `(usize, T)` tuple,
    // but split into two deques to avoid alignment padding when T is narrower
    // than usize (e.g. u8/u16 on 64-bit targets).
    deq: VecDeque<T>,
    expires: VecDeque<usize>,

    ct: usize,
    cap: usize,
}

impl<T> RollingMax<T>
where
    T: PartialOrd,
{
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        let cap: usize = capacity.into();
        Self {
            deq: VecDeque::with_capacity(cap),
            expires: VecDeque::with_capacity(cap),
            cap,
            ct: 0,
        }
    }

    pub fn push(&mut self, entry: T) {
        self.ct = self.ct.wrapping_add(1);

        while self
            .expires
            .front()
            .is_some_and(|&exp| self.ct.wrapping_sub(exp) <= self.cap)
        {
            self.deq.pop_front();
            self.expires.pop_front();
        }

        while self.deq.back().is_some_and(|tail| tail <= &entry) {
            self.deq.pop_back();
            self.expires.pop_back();
        }

        self.deq.push_back(entry);
        self.expires.push_back(self.ct.wrapping_add(self.cap));
    }

    #[must_use]
    pub fn max(&self) -> Option<&T> {
        self.deq.front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).unwrap()
    }

    /// Push every value and collect the rolling max after each push.
    fn maxes<T: PartialOrd + Copy>(vals: &[T], cap: usize) -> Vec<T> {
        let mut rm = RollingMax::new(nz(cap));
        vals.iter()
            .map(|&v| {
                rm.push(v);
                *rm.max().unwrap()
            })
            .collect()
    }

    /// Verifies the zero-state guarantee: max must be None before any push.
    #[test]
    fn max_on_empty_is_none() {
        let rm: RollingMax<i32> = RollingMax::new(nz(3));
        assert_eq!(rm.max(), None);
    }

    /// A single push must always yield Some, regardless of window size.
    #[test]
    fn single_push_yields_some() {
        let mut rm = RollingMax::new(nz(5));
        rm.push(42i32);
        assert_eq!(rm.max(), Some(&42));
    }

    /// Window=1: every element is its own maximum; exercises the path where
    /// the entire deque is evicted on every push.
    #[test]
    fn window_of_one() {
        assert_eq!(maxes(&[3, 1, 4, 1, 5], 1), vec![3, 1, 4, 1, 5]);
    }

    /// Window larger than the entire input: tracker never evicts, so the
    /// running max is monotonically non-decreasing.
    #[test]
    fn window_larger_than_input() {
        assert_eq!(maxes(&[2, 4, 1], 10), vec![2, 4, 4]);
    }

    /// Window exactly equal to input length: global max emerges only after
    /// the last push.
    #[test]
    fn window_equals_input_length() {
        assert_eq!(maxes(&[1, 3, 2, 5, 4], 5), vec![1, 3, 3, 5, 5]);
    }

    /// Core sliding-window case; this exact sequence caught the off-by-one
    /// expiry bug where element `3` incorrectly survived into window [1,2,0].
    #[test]
    fn sliding_window_canonical() {
        assert_eq!(maxes(&[1, 3, 1, 2, 0, 5], 3), vec![1, 3, 3, 3, 2, 5]);
    }

    /// Strictly increasing input: the monotone invariant discards every
    /// predecessor, so the deque always holds exactly one element.
    #[test]
    fn strictly_increasing() {
        assert_eq!(maxes(&[1, 2, 3, 4, 5], 3), vec![1, 2, 3, 4, 5]);
    }

    /// Strictly decreasing input: the oldest value leads the deque and must
    /// survive until it expires, then yield to the next oldest.
    #[test]
    fn strictly_decreasing() {
        assert_eq!(maxes(&[5, 4, 3, 2, 1], 3), vec![5, 5, 5, 4, 3]);
    }

    /// All-equal input: equal elements are pruned from the back (`<=`), so
    /// the deque stays bounded and does not grow without limit.
    #[test]
    fn all_equal() {
        assert_eq!(maxes(&[7i32; 6], 3), vec![7; 6]);
    }

    /// Negative values: ensures no implicit assumption about sign or zero.
    #[test]
    fn negative_values() {
        assert_eq!(maxes(&[-3, -1, -4, -1, -5], 2), vec![-3, -1, -1, -1, -1]);
    }

    /// Float input: exercises the PartialOrd bound on a non-Ord type.
    #[test]
    fn float_values() {
        assert_eq!(
            maxes(&[1.0f64, 3.0, 2.0, 5.0, 4.0], 2),
            vec![1.0, 3.0, 3.0, 5.0, 5.0]
        );
    }

    /// The maximum must survive exactly `cap` pushes and be gone on the next;
    /// guards against off-by-one errors at the expiry boundary.
    #[test]
    fn max_expires_at_exact_boundary() {
        let mut rm = RollingMax::new(nz(3));
        rm.push(99i32);
        rm.push(1);
        rm.push(1);
        assert_eq!(rm.max(), Some(&99)); // 99 still in [99, 1, 1]
        rm.push(1);
        assert_eq!(rm.max(), Some(&1)); // 99 evicted; window is now [1, 1, 1]
    }

    /// Exercises the `usize` counter wrap-around: pre-seeds `ct` so that
    /// expiry values cross the `usize::MAX → 0` boundary, verifying that the
    /// wrapping arithmetic correctly evicts and retains elements.
    #[test]
    fn expiry_counter_wrapping() {
        let cap = 3;
        let mut rm = RollingMax {
            deq: VecDeque::with_capacity(cap),
            expires: VecDeque::with_capacity(cap),
            cap: 3,
            ct: usize::MAX - 3,
        };

        rm.push(10); // ct = usize::MAX-2, exp = 0  (wraps)
        rm.push(5); // ct = usize::MAX-1, exp = 1  (wraps)
        rm.push(8); // ct = usize::MAX,   exp = 2  (wraps)
        assert_eq!(rm.max(), Some(&10)); // window = [10, 5, 8]

        rm.push(6); // ct = 0 (wrap). exp=0 matches ct → evicts 10. window=[5,8,6]
        assert_eq!(rm.max(), Some(&8));

        rm.push(7); // ct = 1. No expiry yet. Monotone pops 6. window=[8,6,7]
        assert_eq!(rm.max(), Some(&8));

        rm.push(9); // ct = 2. exp=2 matches ct → evicts 8. Monotone pops 7. window=[6,7,9]
        assert_eq!(rm.max(), Some(&9));
    }
}
