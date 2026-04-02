use arraydeque::ArrayDeque;
use num_traits::{CheckedAdd, CheckedSub, WrappingAdd, WrappingSub};

// TODO docs:
// Also there's a bitvec optimization we could do
// when T = bool. But that's a project for later.

#[derive(Debug)]
pub struct RollingSum<T, const WINDOW: usize> {
    deq: ArrayDeque<T, WINDOW>,
    total: T,
    wrap_ct: usize,
}

impl<T, const W: usize> Default for RollingSum<T, W>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T, const WINDOW: usize> RollingSum<T, WINDOW>
where
    T: Default,
{
    #[must_use]
    pub fn new(init: T) -> Self {
        const { assert!(WINDOW != 0, "RollingSum with WINDOW == 0 is not permitted") };
        Self {
            deq: ArrayDeque::new(),
            total: init,
            wrap_ct: 0,
        }
    }
}

impl<T, const WINDOW: usize> RollingSum<T, WINDOW>
where
    T: WrappingAdd + WrappingSub + CheckedAdd + CheckedSub + PartialOrd + Copy + Default,
{
    /// Adds `T` to the rolling sum, displacing the oldest
    /// member if the window is full to capacity.
    ///
    /// If adding `T` causes numerical overflow, subsequent
    /// calls to `total` will fail until the sum returns to an
    /// un-overflowed state.
    pub fn add(&mut self, val: T) -> bool {
        if self.deq.is_full() {
            let popped = self.deq.pop_front().expect(
                "len is equal to capacity, and capacity is nonzero. So an element must exist.",
            );
            let before = self.total;
            self.total = self.total.wrapping_sub(&popped);
            if before.checked_sub(&popped).is_none() {
                self.wrap_ct -= 1;
            }
        }

        let before_add = self.total;
        self.total = self.total.wrapping_add(&val);
        self.wrap_ct += before_add.checked_add(&val).is_none() as usize;

        self.deq.push_back(val);
        true
    }

    /// Returns the accumulated total of all added
    /// values that fit within the rolling window's
    /// capacity.
    ///
    /// Returns None if the window has overflowed.
    /// In that case, it will return to Some(..) when
    /// the last element causing overflow is pushed out.
    #[must_use]
    pub fn total(&self) -> Option<&T> {
        (self.wrap_ct == 0).then_some(&self.total)
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Debug;

    use super::*;

    /// Verifies that total() returns `init` before any values are added.
    #[test]
    fn total_before_any_add_is_init() {
        let rs: RollingSum<u32, 3> = RollingSum::default();
        assert_eq!(rs.total(), Some(&0u32));
    }

    /// A single add must accumulate into total without triggering eviction.
    #[test]
    fn single_add_below_capacity() {
        let mut rs: RollingSum<u32, 3> = RollingSum::default();
        rs.add(10);
        assert_eq!(rs.total(), Some(&10));
    }

    /// Filling exactly to capacity must sum all values with no eviction.
    #[test]
    fn fill_to_capacity_no_eviction() {
        self::expect_total::<u32, 3>([1, 2, 3].into_iter().zip([1, 3, 6]));
    }

    /// The (capacity+1)th add must evict the oldest element.
    #[test]
    fn first_eviction_at_capacity_plus_one() {
        // Window = [2, 3, 4] after evicting 1.
        self::expect_total::<u32, 3>([1, 2, 3, 4].into_iter().zip([1, 3, 6, 9]));
    }

    /// Step through a longer sequence to verify correct FIFO eviction ordering.
    #[test]
    fn sliding_window_trace() {
        // cap=3: [5]=5, [5,3]=8, [5,3,8]=16, [3,8,2]=13, [8,2,6]=16
        self::expect_total::<u32, 3>([5, 3, 8, 2, 6].into_iter().zip([5, 8, 16, 13, 16]));
    }

    /// capacity=1: each add completely replaces the previous value.
    #[test]
    fn window_of_one() {
        self::expect_total::<u32, 1>([5, 3, 9, 1].into_iter().zip([5, 3, 9, 1]));
    }

    /// Window larger than the input: no eviction ever occurs.
    #[test]
    fn window_larger_than_input() {
        self::expect_total::<u32, 100>([1, 2, 3, 4, 5].into_iter().zip([1, 3, 6, 10, 15]));
    }

    /// Signed integers: negative values must be summed and evicted correctly.
    #[test]
    fn signed_integers() {
        // cap=2: [-3]=-3, [-3,5]=2, [5,-2]=3, [-2,4]=2
        self::expect_total::<i32, 2>([-3, 5, -2, 4].into_iter().zip([-3, 2, 3, 2]));
    }

    /// Hitting u8::MAX exactly (no overflow) then recovering on eviction.
    #[test]
    fn u8_boundary_exact() {
        let mut rs = RollingSum::<u8, 3>::default();
        rs.add(100);
        rs.add(100);
        rs.add(55);
        assert_eq!(rs.total(), Some(&255)); // 100+100+55 = u8::MAX exactly
        rs.add(0);
        assert_eq!(rs.total(), Some(&155)); // evicted 100 → 100+55+0
    }

    /// total() returns None while the window sum has overflowed, and recovers to Some
    /// once all overflowing elements have been evicted.
    #[test]
    fn overflow_detected_then_recovered() {
        let mut rs = RollingSum::<u8, 2>::default();
        rs.add(200);
        assert_eq!(rs.total(), Some(&200)); // single element, no overflow
        rs.add(100); // 200+100=300 > u8::MAX → wrap_ct=1
        assert_eq!(rs.total(), None); // overflow detected
        rs.add(50); // evicts 200; window=[100,50], sum=150
        assert_eq!(rs.total(), Some(&150)); // overflow healed
    }

    /// Fills the window with three individually-overflowing values (wrap_ct reaches 2),
    /// then slides in small values to evict them one at a time, confirming that
    /// total() stays None while any overflow-causing element remains in the window
    /// and returns exact Some values as each one is expelled.
    #[test]
    fn double_overflow_and_full_recovery() {
        let mut rs = RollingSum::<u8, 3>::default();

        rs.add(200);
        assert_eq!(rs.total(), Some(&200)); // [200], true=200, wrap_ct=0
        rs.add(200);
        assert_eq!(rs.total(), None); // [200,200], true=400, wrap_ct=1
        rs.add(200);
        assert_eq!(rs.total(), None); // [200,200,200], true=600, wrap_ct=2

        rs.add(10);
        assert_eq!(rs.total(), None); // evict 200 → wrap_ct=1; [200,200,10], true=410
        rs.add(10);
        assert_eq!(rs.total(), Some(&220)); // evict 200 → wrap_ct=0; [200,10,10], true=220
        rs.add(10);
        assert_eq!(rs.total(), Some(&30)); // evict 200, no wrap; [10,10,10], true=30
    }

    /// Large u64 values that stay within range: verifies no spurious overflow.
    #[test]
    fn u64_large_values_no_overflow() {
        let half = u64::MAX / 2;
        let mut rs = RollingSum::<_, 2>::default();
        rs.add(half);
        rs.add(half);
        assert_eq!(rs.total(), Some(&(half * 2))); // 2^63 - 1, no overflow
        rs.add(1);
        assert_eq!(rs.total(), Some(&(half + 1))); // evicted half → half + 1
    }

    /// Feeds inputs from an `(input, expected)` iterator into
    /// a RollingSum. Compares each total to `expected` and panics
    /// if they're not equal.
    fn expect_total<T, const WINDOW: usize>(input_and_expected: impl Iterator<Item = (T, T)>)
    where
        T: WrappingAdd
            + WrappingSub
            + CheckedAdd
            + CheckedSub
            + PartialOrd
            + Copy
            + Default
            + Debug,
    {
        let mut roll: RollingSum<T, WINDOW> = RollingSum::default();
        for (input, expected) in input_and_expected {
            roll.add(input);
            assert_eq!(*roll.total().unwrap(), expected);
        }
    }
}
