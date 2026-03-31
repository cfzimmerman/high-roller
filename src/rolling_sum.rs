use num_traits::{CheckedAdd, CheckedSub, WrappingAdd, WrappingSub};
use std::{collections::VecDeque, num::NonZeroUsize};

// TODO docs:
// Also there's a bitvec optimization we could do
// when T = bool. But that's a project for later.

#[derive(Debug)]
pub struct RollingSum<T> {
    deq: VecDeque<T>,
    total: T,
    capacity: usize,
    wrap_ct: usize,
}

impl<T> RollingSum<T>
where
    T: WrappingAdd + WrappingSub + CheckedAdd + CheckedSub + PartialOrd + Copy + Default,
{
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            deq: VecDeque::with_capacity(capacity.into()),
            total: T::default(),
            capacity: capacity.into(),
            wrap_ct: 0,
        }
    }

    /// Adds `T` to the rolling sum, displacing the oldest
    /// member if the window is full to capacity.
    ///
    /// If adding `T` causes numerical overflow, subsequent
    /// calls to `total` will fail until the sum returns to an
    /// un-overflowed state.
    pub fn add(&mut self, val: T) -> bool {
        if self.deq.len() == self.capacity {
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
    use super::*;

    fn nz(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).unwrap()
    }

    /// Push every value and collect total() snapshots after each add.
    fn totals<T>(vals: &[T], cap: usize) -> Vec<T>
    where
        T: WrappingAdd + WrappingSub + CheckedAdd + CheckedSub + PartialOrd + Copy + Default,
    {
        let mut rs = RollingSum::new(nz(cap));
        vals.iter()
            .map(|&v| {
                rs.add(v);
                *rs.total().unwrap()
            })
            .collect()
    }

    /// Verifies that total() returns `init` before any values are added.
    #[test]
    fn total_before_any_add_is_init() {
        let rs: RollingSum<u32> = RollingSum::new(nz(3));
        assert_eq!(rs.total(), Some(&0u32));
    }

    /// A single add must accumulate into total without triggering eviction.
    #[test]
    fn single_add_below_capacity() {
        let mut rs: RollingSum<u32> = RollingSum::new(nz(3));
        rs.add(10);
        assert_eq!(rs.total(), Some(&10));
    }

    /// Filling exactly to capacity must sum all values with no eviction.
    #[test]
    fn fill_to_capacity_no_eviction() {
        assert_eq!(totals(&[1u32, 2, 3], 3), vec![1, 3, 6]);
    }

    /// The (capacity+1)th add must evict the oldest element.
    #[test]
    fn first_eviction_at_capacity_plus_one() {
        // Window = [2, 3, 4] after evicting 1.
        assert_eq!(totals(&[1u32, 2, 3, 4], 3), vec![1, 3, 6, 9]);
    }

    /// Step through a longer sequence to verify correct FIFO eviction ordering.
    #[test]
    fn sliding_window_trace() {
        // cap=3: [5]=5, [5,3]=8, [5,3,8]=16, [3,8,2]=13, [8,2,6]=16
        assert_eq!(totals(&[5u32, 3, 8, 2, 6], 3), vec![5, 8, 16, 13, 16]);
    }

    /// capacity=1: each add completely replaces the previous value.
    #[test]
    fn window_of_one() {
        assert_eq!(totals(&[5u32, 3, 9, 1], 1), vec![5, 3, 9, 1]);
    }

    /// Window larger than the input: no eviction ever occurs.
    #[test]
    fn window_larger_than_input() {
        assert_eq!(totals(&[1u32, 2, 3, 4, 5], 100), vec![1, 3, 6, 10, 15]);
    }

    /// Signed integers: negative values must be summed and evicted correctly.
    #[test]
    fn signed_integers() {
        // cap=2: [-3]=-3, [-3,5]=2, [5,-2]=3, [-2,4]=2
        assert_eq!(totals(&[-3i32, 5, -2, 4], 2), vec![-3, 2, 3, 2]);
    }

    /// Hitting u8::MAX exactly (no overflow) then recovering on eviction.
    #[test]
    fn u8_boundary_exact() {
        let mut rs = RollingSum::new(nz(3));
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
        let mut rs = RollingSum::<u8>::new(nz(2));
        rs.add(200);
        assert_eq!(rs.total(), Some(&200)); // single element, no overflow
        rs.add(100);                        // 200+100=300 > u8::MAX → wrap_ct=1
        assert_eq!(rs.total(), None);       // overflow detected
        rs.add(50);                         // evicts 200; window=[100,50], sum=150
        assert_eq!(rs.total(), Some(&150)); // overflow healed
    }

    /// Fills the window with three individually-overflowing values (wrap_ct reaches 2),
    /// then slides in small values to evict them one at a time, confirming that
    /// total() stays None while any overflow-causing element remains in the window
    /// and returns exact Some values as each one is expelled.
    #[test]
    fn double_overflow_and_full_recovery() {
        let mut rs = RollingSum::<u8>::new(nz(3));

        rs.add(200);
        assert_eq!(rs.total(), Some(&200)); // [200], true=200, wrap_ct=0
        rs.add(200);
        assert_eq!(rs.total(), None);       // [200,200], true=400, wrap_ct=1
        rs.add(200);
        assert_eq!(rs.total(), None);       // [200,200,200], true=600, wrap_ct=2

        rs.add(10);
        assert_eq!(rs.total(), None);       // evict 200 → wrap_ct=1; [200,200,10], true=410
        rs.add(10);
        assert_eq!(rs.total(), Some(&220)); // evict 200 → wrap_ct=0; [200,10,10], true=220
        rs.add(10);
        assert_eq!(rs.total(), Some(&30));  // evict 200, no wrap; [10,10,10], true=30
    }

    /// Large u64 values that stay within range: verifies no spurious overflow.
    #[test]
    fn u64_large_values_no_overflow() {
        let half = u64::MAX / 2;
        let mut rs = RollingSum::new(nz(2));
        rs.add(half);
        rs.add(half);
        assert_eq!(rs.total(), Some(&(half * 2))); // 2^63 - 1, no overflow
        rs.add(1);
        assert_eq!(rs.total(), Some(&(half + 1))); // evicted half → half + 1
    }
}
