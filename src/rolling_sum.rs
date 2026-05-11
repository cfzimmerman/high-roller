//! # Rolling Sum
//!
//! [`RollingSum`] tracks the sum of values in a fixed-size window.
//! When the window is full, pushing a new value evicts the oldest.
//!
//! API guarantees:
//! - Add is O(1).
//! - Total is O(1).
//! - Overflow and underflow are detected and recoverable.
//! - Zero heap allocations.
//!
//! The value of this implementation is error recovery.
//! [`RollingSum`] returns [`None`] as long as the inner sum
//! overflows `T::MAX` or underflows `T::MIN`. And it resumes
//! yielding `Some(&T)` as soon as the inner sum recovers.
//!
//! ```
//! use high_roller::rolling_sum::RollingSum;
//!
//! let mut rsum: RollingSum<i8, 3> = RollingSum::default();
//!
//! // Add an initial value.
//! rsum.add(5);
//! assert_eq!(rsum.total().copied(), Some(5));
//!
//! // Stream of -1s takes over the window.
//! for _ in 0..100 {
//!     rsum.add(-1);
//! }
//! assert_eq!(rsum.total().copied(), Some(-3));
//!
//! // Underflow forces total to None.
//! rsum.add(i8::MIN);
//! assert_eq!(rsum.total(), None);
//!
//! // Balanced back to zero.
//! rsum.add(i8::MAX);
//! rsum.add(1);
//! assert_eq!(rsum.total().copied(), Some(0));
//!
//! // Evicting i8::MIN causes overflow.
//! rsum.add(0);
//! assert_eq!(rsum.total(), None);
//!
//! // Cause multiple overflows.
//! for _ in 0..100 {
//!     rsum.add(i8::MAX);
//! }
//! assert_eq!(rsum.total(), None);
//!
//! // And recover back into an expected range.
//! for _ in 0..100 {
//!     rsum.add(1);
//! }
//! assert_eq!(rsum.total().copied(), Some(3));
//! ```

use arraydeque::ArrayDeque;
use num_traits::CheckedAdd;
use num_traits::CheckedSub;
use num_traits::WrappingAdd;
use num_traits::WrappingSub;

// PERF: using a bitvec or double bitvec for RollingSum on
// applicable types could be a pretty nice space optimization.
// Future project :)

/// Tracks a rolling sum of at most `WINDOW` values.
///
/// This type is stack allocated. However a large window
/// might warrant boxing. Clustering multiple instances
/// in the same allocation might offer better cache
/// access patterns.
///
/// ```
/// use high_roller::rolling_sum::RollingSum;
///
/// struct Average {
///     total: RollingSum<u32, 6000>,
///     samples: RollingSum<u8, 6000>
/// }
///
/// // Probably want to box this.
/// const _: () = assert!(core::mem::size_of::<Average>() == 30064);
/// ```
#[derive(Debug)]
pub struct RollingSum<T, const WINDOW: usize> {
    deq: ArrayDeque<T, WINDOW>,
    total: T,
    zero: T,
    balance: isize,
}

impl<T, const W: usize> Default for RollingSum<T, W>
where
    T: Default,
{
    /// Constructs a new [`RollingSum`] with sum and zero
    /// values equal to `T::default()`.
    fn default() -> Self {
        Self::new(T::default(), T::default())
    }
}

impl<T, const WINDOW: usize> RollingSum<T, WINDOW>
where
    T: Default,
{
    /// Constructs a new [`RollingSum`].
    ///
    /// Prefer using [`RollingSum::default`] for a cleaner API.
    /// `init` is the sum's initial value. `zero` is value separating
    /// positive and negative for this type. While identifying zero is strange,
    /// passing as a parameter avoids requiring a `Zero` trait impl, which isn't
    /// fun for anybody. Zero is required for differentiating overflow
    /// and underflow.
    ///
    /// ```
    /// use high_roller::rolling_sum::RollingSum;
    ///
    /// let _sum: RollingSum::<usize, 10> = RollingSum::new(0, 0);
    /// ```
    ///
    /// # Compile errors
    ///
    /// Using `WINDOW == 0` is a compile-time error.
    ///
    /// ```compile_fail
    /// # use high_roller::rolling_sum::RollingSum;
    /// // This will fail to compile. A rolling sum of capacity 0 is nonsensical.
    /// let _sum: RollingSum::<usize, 0> = RollingSum::default();
    /// ```
    #[must_use]
    pub const fn new(init: T, zero: T) -> Self {
        const { assert!(WINDOW != 0, "RollingSum with WINDOW == 0 is not permitted") };
        Self {
            deq: ArrayDeque::new(),
            total: init,
            balance: 0,
            zero,
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
    /// calls to [`RollingSum::total`] will return [`None`] until window
    /// expirations cause underflow commensurate to the overflow.
    ///
    /// ```
    /// use high_roller::rolling_sum::RollingSum;
    ///
    /// let mut rsum: RollingSum<i32, 1000> = RollingSum::default();
    ///
    /// rsum.add(100);
    /// rsum.add(1);
    /// rsum.add(-2);
    ///
    /// assert_eq!(rsum.total().copied(), Some(99));
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics if the [`isize`] signed overflow balance
    /// counter itself overflows. Such a panic is impossible as long
    /// as `WINDOW <= isize::MAX`. And an array of size `isize::MAX`
    /// bytes is fantasy on all current hardware anyway.
    //
    // Clippy allow:
    // Explained inline. This is easily provable, will never occur,
    // and should not be exposed to the user.
    #[allow(clippy::expect_used)]
    #[allow(clippy::missing_panics_doc)]
    pub fn add(&mut self, val: T) {
        if self.deq.is_full() {
            // Construction has a const assertion that WINDOW is not zero.
            // So `is_full` guarantees there's something to pop.
            let popped = self.deq.pop_front().expect(
                "len is equal to capacity, and capacity is nonzero. So an element must exist.",
            );

            let changed = self.total.checked_sub(&popped).is_none();
            self.total = self.total.wrapping_sub(&popped);

            if changed {
                self.balance = self
                    .balance
                    .checked_add(if popped >= self.zero { -1 } else { 1 })
                    .expect("overflow count itself overflowed");
            }
        }

        let changed = self.total.checked_add(&val).is_none();
        self.total = self.total.wrapping_add(&val);

        if changed {
            self.balance = self
                .balance
                .checked_add(if val >= self.zero { 1 } else { -1 })
                .expect("overflow count itself overflowed");
        }

        // The `if` condition above guarantees the deque
        // is not full. So there's space to push a value.
        self.deq.push_back(val).expect("deq is not full");
    }

    /// Returns the accumulated total of all added values that
    /// fit within the rolling window's capacity.
    ///
    /// Returns [`None`] if the window has overflowed. [`RollingSum`]
    /// will resume returning `Some(&T)` when the last element
    /// causing overflow is pushed out of the window.
    ///
    /// ```
    /// use high_roller::rolling_sum::RollingSum;
    ///
    /// let mut rsum: RollingSum<i32, 100> = RollingSum::default();
    ///
    /// rsum.add(i32::MIN);
    /// assert_eq!(rsum.total().copied(), Some(i32::MIN));
    ///
    /// for _ in 0..1000 {
    ///     rsum.add(i32::MIN);
    ///     assert_eq!(rsum.total(), None);
    /// }
    ///
    /// for _ in 0..100 {
    ///     rsum.add(-1);
    /// }
    /// assert_eq!(rsum.total().copied(), Some(-100));
    /// ```
    #[must_use]
    pub fn total(&self) -> Option<&T> {
        (self.balance == 0).then_some(&self.total)
    }
}

#[cfg(test)]
pub mod for_tests {
    use arraydeque::ArrayDeque;
    use arraydeque::Wrapping;
    use num_bigint::BigInt;

    /// A simple implementation satisfying the same API as
    /// this crate's `RollingSum` type. This is used for both
    /// correctness and performance testing.
    ///
    /// See the note in total. This is not as robust against
    /// underflow as RollingSum.
    #[derive(Debug, Default)]
    pub struct NaiveRollingSum<T, const WINDOW: usize> {
        deq: ArrayDeque<T, WINDOW, Wrapping>,
        sum: BigInt,
    }

    impl<T, const WINDOW: usize> NaiveRollingSum<T, WINDOW>
    where
        T: Clone + Default + Into<BigInt> + for<'a> TryFrom<&'a BigInt>,
    {
        #[must_use]
        pub fn new(init: T) -> Self {
            const { assert!(WINDOW != 0, "RollingSum with WINDOW == 0 is not permitted") };
            Self {
                deq: ArrayDeque::new(),
                sum: init.into(),
            }
        }

        pub fn add(&mut self, val: T) {
            self.sum += Into::<BigInt>::into(val.clone());
            if let Some(replaced) = self.deq.push_back(val) {
                self.sum -= Into::<BigInt>::into(replaced);
            }
        }

        #[must_use]
        pub fn total(&self) -> Option<T> {
            (&self.sum).try_into().ok()
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::decimal::D1;
    use crate::decimal::D5;
    use crate::rolling_sum::for_tests::NaiveRollingSum;
    use core::fmt::Debug;
    use rand::distr::Uniform;
    use rand::rngs::SmallRng;
    use rand::RngExt;
    use rand::SeedableRng;

    /// Smoke test for RollingSum correctness.
    ///
    /// Accumulates a representative RollingMax and NaiveRollingMax
    /// to verify their outputs are identical.
    #[test]
    fn rng_with_naive() {
        const QLEN: usize = 1000;
        const STREAM_LEN: usize = 100_000;

        let sample = SmallRng::seed_from_u64(57).sample_iter(Uniform::new(-800f32, 800.).unwrap());
        let mut roller = RollingSum::<D5, QLEN>::default();
        let mut naive = NaiveRollingSum::<D5, QLEN>::default();

        for val in sample.take(STREAM_LEN) {
            let d4 = D5::cast(val.into());
            roller.add(d4);
            naive.add(d4);
            assert_eq!(roller.total(), naive.total().as_ref());
        }
    }

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
        const HALF: u64 = (u64::MAX as f64 / 2.) as u64 - 1;
        let mut rs = RollingSum::<_, 2>::default();
        rs.add(HALF);
        rs.add(HALF);
        assert_eq!(rs.total(), Some(&(HALF * 2))); // 2^63 - 1, no overflow
        rs.add(1);
        assert_eq!(rs.total(), Some(&(HALF + 1))); // evicted half → half + 1
    }

    #[test]
    fn overflow_negative() {
        let mut rs = RollingSum::<i32, 3>::default();

        rs.add(i32::MAX); // Total = MAX
        assert!(rs.total().is_some());

        rs.add(i32::MIN); // Total = MAX + MIN
        assert!(rs.total().is_some());

        rs.add(i32::MAX); // Total = MAX + MIN + MAX
        assert!(rs.total().is_some());

        rs.add(i32::MAX); // Total = MIN + MAX + MAX
        assert!(rs.total().is_some());

        rs.add(0); // Total = MAX + MAX + 0
        assert!(rs.total().is_none());

        rs.add(0); // Total = MAX + 0 + 0
        assert_eq!(rs.total(), Some(&i32::MAX));
    }

    #[test]
    fn underflow_negative() {
        let mut rs = RollingSum::<i32, 3>::default();

        rs.add(i32::MIN);
        assert!(rs.total().is_some());

        rs.add(-1);
        assert!(rs.total().is_none());

        rs.add(1);
        assert_eq!(rs.total(), Some(&i32::MIN));
    }

    #[test]
    fn decimal_overflow() {
        let mut rs = RollingSum::<D1, 4>::default();

        rs.add(D1::MAX);
        assert!(rs.total().is_some());

        for _ in 0..100 {
            rs.add(D1::MAX);
            assert!(rs.total().is_none());
        }

        for _ in 0..3 {
            rs.add(D1::ZERO);
        }

        rs.add(D1::MIN_UNIT);
        assert!(matches!(rs.total(), Some(&D1::MIN_UNIT)));
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
