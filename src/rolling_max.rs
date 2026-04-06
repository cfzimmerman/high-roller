// TODO: DOCS

use arraydeque::ArrayDeque;

#[derive(Debug, Default)]
pub struct RollingMax<T, const WINDOW: usize> {
    deq: ArrayDeque<T, WINDOW>,
    expires: ArrayDeque<usize, WINDOW>,
    ct: usize,
}

impl<T, const W: usize> RollingMax<T, W>
where
    T: PartialOrd,
{
    #[must_use]
    pub const fn new() -> Self {
        Self {
            deq: ArrayDeque::new(),
            expires: ArrayDeque::new(),
            ct: 0,
        }
    }

    // Clippy allow:
    //
    // Expect is used in this function to guarantee invariants.
    // See the note within the function.
    //
    // It should never panic in user code. So exposing or documenting
    // the failure case makes the API unnecessarily leaky.
    #[allow(clippy::expect_used)]
    #[allow(clippy::missing_panics_doc)]
    pub fn push(&mut self, entry: T) {
        self.ct = self.ct.wrapping_add(1);

        while self
            .expires
            .front()
            .is_some_and(|&exp| self.ct.wrapping_sub(exp) <= W)
        {
            self.deq.pop_front();
            self.expires.pop_front();
        }

        while self.deq.back().is_some_and(|tail| tail <= &entry) {
            self.deq.pop_back();
            self.expires.pop_back();
        }

        // The first loop pops any entry whose expiration equals
        // or exceeds W. So every entry in the queue has a nonzero
        // expiration less than W. The queue has capacity W. So the
        // queue is guaranteed to have at least one spot available.
        // The calls to `expect` below check this invariant.
        self.deq
            .push_back(entry)
            .expect("expirations guarantee queue is never full at this point");
        self.expires
            .push_back(self.ct.wrapping_add(W))
            .expect("expirations guarantee queue is never full at this point");
    }

    #[must_use]
    pub fn max(&self) -> Option<&T> {
        self.deq.front()
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use super::*;

    /// Verifies the zero-state guarantee: max must be None before any push.
    #[test]
    fn max_on_empty_is_none() {
        let rm: RollingMax<i32, 3> = RollingMax::new();
        assert_eq!(rm.max(), None);
    }

    /// A single push must always yield Some, regardless of window size.
    #[test]
    fn single_push_yields_some() {
        let mut rm: RollingMax<i32, 5> = RollingMax::new();
        rm.push(42);
        assert_eq!(rm.max(), Some(&42));
    }

    /// Window=1: every element is its own maximum; exercises the path where
    /// the entire deque is evicted on every push.
    #[test]
    fn window_of_one() {
        expect_max::<i32, 1>([3, 1, 4, 1, 5].into_iter().zip([3, 1, 4, 1, 5]));
    }

    /// Window larger than the entire input: tracker never evicts, so the
    /// running max is monotonically non-decreasing.
    #[test]
    fn window_larger_than_input() {
        expect_max::<i32, 10>([2, 4, 1].into_iter().zip([2, 4, 4]));
    }

    /// Window exactly equal to input length: global max emerges only after
    /// the last push.
    #[test]
    fn window_equals_input_length() {
        expect_max::<i32, 5>([1, 3, 2, 5, 4].into_iter().zip([1, 3, 3, 5, 5]));
    }

    /// Core sliding-window case; this exact sequence caught the off-by-one
    /// expiry bug where element `3` incorrectly survived into window [1,2,0].
    #[test]
    fn sliding_window_canonical() {
        expect_max::<i32, 3>([1, 3, 1, 2, 0, 5].into_iter().zip([1, 3, 3, 3, 2, 5]));
    }

    /// Strictly increasing input: the monotone invariant discards every
    /// predecessor, so the deque always holds exactly one element.
    #[test]
    fn strictly_increasing() {
        expect_max::<i32, 3>([1, 2, 3, 4, 5].into_iter().zip([1, 2, 3, 4, 5]));
    }

    /// Strictly decreasing input: the oldest value leads the deque and must
    /// survive until it expires, then yield to the next oldest.
    #[test]
    fn strictly_decreasing() {
        expect_max::<i32, 3>([5, 4, 3, 2, 1].into_iter().zip([5, 5, 5, 4, 3]));
    }

    /// All-equal input: equal elements are pruned from the back (`<=`), so
    /// the deque stays bounded and does not grow without limit.
    #[test]
    fn all_equal() {
        expect_max::<i32, 3>([7i32; 6].into_iter().zip([7; 6]));
    }

    /// Negative values: ensures no implicit assumption about sign or zero.
    #[test]
    fn negative_values() {
        expect_max::<i32, 2>([-3, -1, -4, -1, -5].into_iter().zip([-3, -1, -1, -1, -1]));
    }

    /// Float input: exercises the PartialOrd bound on a non-Ord type.
    #[test]
    fn float_values() {
        expect_max::<f32, 2>(
            [1.0, 3.0, 2.0, 5.0, 4.0]
                .into_iter()
                .zip([1.0, 3.0, 3.0, 5.0, 5.0]),
        );
    }

    /// The maximum must survive exactly `cap` pushes and be gone on the next;
    /// guards against off-by-one errors at the expiry boundary.
    #[test]
    fn max_expires_at_exact_boundary() {
        let mut rm = RollingMax::<i32, 3>::new();
        rm.push(99);
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
        let mut rm: RollingMax<i32, 3> = RollingMax {
            deq: ArrayDeque::new(),
            expires: ArrayDeque::new(),
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

    /// Feeds inputs from an `(input, expected)` iterator into
    /// a RollingMax. Compares each max to `expected` and panics
    /// if they're not equal.
    #[allow(clippy::unwrap_used)]
    fn expect_max<T, const WINDOW: usize>(input_and_expected: impl Iterator<Item = (T, T)>)
    where
        T: PartialOrd + Copy + Debug + PartialEq,
    {
        let mut rm: RollingMax<T, WINDOW> = RollingMax::new();
        for (input, expected) in input_and_expected {
            rm.push(input);
            assert_eq!(*rm.max().unwrap(), expected);
        }
    }
}
