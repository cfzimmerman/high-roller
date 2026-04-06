use core::f32;
use core::fmt::Debug;
use core::ops::{Add, Sub};

use num_traits::{CheckedAdd, CheckedSub, WrappingAdd, WrappingSub};
use thiserror::Error;

// TODO(corzimmerman):
// Make this a macro?

pub type D1 = Decimal32<1>;
pub type D2 = Decimal32<2>;
pub type D3 = Decimal32<3>;
pub type D4 = Decimal32<4>;
pub type D5 = Decimal32<5>;
pub type D6 = Decimal32<6>;
pub type D7 = Decimal32<7>;
pub type D8 = Decimal32<8>;
pub type D9 = Decimal32<9>;
pub type D10 = Decimal32<10>;

// TODO(corzimmerman): do we need this error type?
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DecimalErr {
    #[error("The attempted operation would cause a loss of precision.")]
    Lossy,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Decimal32<const PRECISION: u32>(i32);

impl<const P: u32> Debug for Decimal32<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.get())
    }
}

const fn scalar(precision: u32) -> u32 {
    10u32.pow(precision)
}

impl<const PRECISION: u32> Decimal32<PRECISION> {
    pub const ZERO: Self = Self(0);

    /// The greatest positive value this type can contain.
    pub const MAX: Self = Self(i32::MAX);

    /// The most negative value this type can contain.
    pub const MIN: Self = Self(i32::MIN);

    /// The smallest positive value this type can contain.
    pub const MIN_UNIT: Self = Self(1);

    #[must_use]
    pub const fn cast(value: f32) -> Self {
        Self((value * self::scalar(PRECISION) as f32) as i32)
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0 as f32 / self::scalar(PRECISION) as f32
    }
}

impl<const P: u32> TryFrom<f32> for Decimal32<P> {
    type Error = DecimalErr;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        let dec = Self::cast(value);
        if dec.get() != value {
            return Err(DecimalErr::Lossy);
        }
        Ok(dec)
    }
}

impl<const P: u32> TryFrom<Decimal32<P>> for f32 {
    type Error = DecimalErr;

    fn try_from(value: Decimal32<P>) -> Result<Self, Self::Error> {
        let float = value.get();
        if Decimal32::<P>::cast(float) != value {
            return Err(DecimalErr::Lossy);
        }
        Ok(float)
    }
}

impl<const P: u32> Add for Decimal32<P> {
    type Output = Self;

    /// Uses wrapping addition.
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_add(rhs.0))
    }
}

impl<const P: u32> CheckedAdd for Decimal32<P> {
    fn checked_add(&self, v: &Self) -> Option<Self> {
        self.0.checked_add(v.0).map(Self)
    }
}

impl<const P: u32> WrappingAdd for Decimal32<P> {
    fn wrapping_add(&self, v: &Self) -> Self {
        Self(self.0.wrapping_add(v.0))
    }
}

impl<const P: u32> Default for Decimal32<P> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<const P: u32> Sub for Decimal32<P> {
    type Output = Self;

    /// Uses wrapping subtraction.
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl<const P: u32> CheckedSub for Decimal32<P> {
    fn checked_sub(&self, v: &Self) -> Option<Self> {
        self.0.checked_sub(v.0).map(Self)
    }
}

impl<const P: u32> WrappingSub for Decimal32<P> {
    #[inline]
    fn wrapping_sub(&self, v: &Self) -> Self {
        Self(self.0.wrapping_sub(v.0))
    }
}

#[allow(clippy::missing_panics_doc)]
#[allow(clippy::expect_used)]
#[cfg(test)]
pub mod decimal_tests {
    use num_traits::{CheckedAdd, CheckedSub, WrappingAdd, WrappingSub};

    use crate::decimal::{Decimal32, DecimalErr, D3};

    /// Checks equality within one unit of the given precision (i.e. tolerance = 10^-precision).
    pub fn assert_eq_f32(left: f32, right: f32, precision: u32) {
        let tolerance = 1.0 / super::scalar(precision) as f32;
        assert!(
            (left - right).abs() < tolerance,
            "equality failed: {left:?} != {right:?} (tolerance {tolerance})"
        );
    }

    // --- Conversion ---

    /// Values exactly representable at P=3 survive a cast_from / get round-trip.
    #[test]
    fn cast_from_exact() {
        assert_eq_f32(D3::cast(0.001).get(), 0.001, 3);
        assert_eq_f32(D3::cast(7.120).get(), 7.120, 3);
        assert_eq_f32(D3::cast(-3.500).get(), -3.500, 3);
    }

    /// cast truncates toward zero rather than rounding.
    #[test]
    fn cast_truncates() {
        // 0.9999 * 1 = 0.9999, cast to i32 truncates to 0
        let d = Decimal32::<0>::cast(0.9999_f32);
        assert_eq!(d.get(), 0.0_f32);
    }

    /// try_from succeeds for values the type can represent without precision loss.
    #[test]
    fn try_from_lossless() {
        assert!(D3::try_from(0.001_f32).is_ok());
        assert!(D3::try_from(7.120_f32).is_ok());
        assert!(D3::try_from(0.0_f32).is_ok());
    }

    /// try_from returns Err(Lossy) when the f32 value can't be represented exactly.
    #[test]
    fn try_from_lossy() {
        // 1/3 is not representable at any finite decimal precision
        assert!(matches!(
            D3::try_from(1.0_f32 / 3.0_f32),
            Err(DecimalErr::Lossy)
        ));

        // But cast accepts it
        assert_eq_f32(D3::cast(1. / 3.).get(), 0.333, 3);
    }

    /// TryFrom<Decimal32<P>> for f32 succeeds for values that round-trip cleanly.
    #[test]
    fn decimal_to_f32_ok() {
        let d = D3::cast(7.120);
        let f: f32 = f32::try_from(d).expect("should round-trip");
        assert_eq_f32(f, 7.120, 3);
    }

    // --- Arithmetic ---

    /// Basic addition produces the correct sum.
    #[test]
    fn add_basic() {
        let sum = D3::cast(0.001) + D3::cast(7.120);
        assert_eq_f32(sum.get(), 7.121, 3);
    }

    /// Adding a negative value crosses zero correctly.
    #[test]
    fn add_negative() {
        let sum = D3::cast(1.000) + D3::cast(-3.500);
        assert_eq_f32(sum.get(), -2.500, 3);
    }

    /// checked_add returns Some when the result fits in i32.
    #[test]
    fn checked_add_ok() {
        let a = D3::cast(1.000);
        let b = D3::cast(2.000);
        assert_eq!(a.checked_add(&b), Some(D3::cast(3.000)));
    }

    /// checked_add returns None when the internal i32 would overflow.
    #[test]
    fn checked_add_overflow() {
        let max = Decimal32::<0>(i32::MAX);
        let one = Decimal32::<0>(1);
        assert_eq!(max.checked_add(&one), None);
    }

    /// wrapping_add wraps the internal i32 on overflow.
    #[test]
    fn wrapping_add_overflow() {
        let max = Decimal32::<0>(i32::MAX);
        let one = Decimal32::<0>(1);
        let expected = Decimal32::<0>(i32::MIN);
        assert_eq!(max.wrapping_add(&one), expected);
    }

    /// Basic subtraction produces the correct difference.
    #[test]
    fn sub_basic() {
        let diff = D3::cast(7.121) - D3::cast(0.001);
        assert_eq_f32(diff.get(), 7.120, 3);
    }

    /// Subtraction can produce a negative result.
    #[test]
    fn sub_to_negative() {
        let diff = D3::cast(1.000) - D3::cast(3.500);
        assert_eq_f32(diff.get(), -2.500, 3);
    }

    /// checked_sub returns Some when the result fits in i32.
    #[test]
    fn checked_sub_ok() {
        let a = D3::cast(5.000);
        let b = D3::cast(2.000);
        assert_eq!(a.checked_sub(&b), Some(D3::cast(3.000)));
    }

    /// checked_sub returns None when the internal i32 would underflow.
    #[test]
    fn checked_sub_underflow() {
        let min = Decimal32::<0>(i32::MIN);
        let one = Decimal32::<0>(1);
        assert_eq!(min.checked_sub(&one), None);
    }

    /// wrapping_sub wraps the internal i32 on underflow.
    #[test]
    fn wrapping_sub_underflow() {
        let min = Decimal32::<0>(i32::MIN);
        let one = Decimal32::<0>(1);
        let expected = Decimal32::<0>(i32::MAX);
        assert_eq!(min.wrapping_sub(&one), expected);
    }

    // --- Identity / Semantic ---

    /// Adding ZERO leaves the value unchanged from both sides.
    #[test]
    fn zero_additive_identity() {
        let x = D3::cast(4.200);
        assert_eq!(x + D3::ZERO, x);
        assert_eq!(D3::ZERO + x, x);
    }

    /// Subtracting ZERO leaves the value unchanged.
    #[test]
    fn zero_subtractive_identity() {
        let x = D3::cast(4.200);
        assert_eq!(x - D3::ZERO, x);
    }

    /// PartialOrd is consistent across negative, zero, and positive values.
    #[test]
    fn ordering() {
        let neg = D3::cast(-1.000);
        let zero = D3::ZERO;
        let pos = D3::cast(1.000);
        assert!(neg < zero);
        assert!(zero < pos);
        assert!(neg < pos);
        assert_eq!(zero, D3::cast(0.000));
    }

    /// Default returns ZERO.
    #[test]
    fn default_is_zero() {
        assert_eq!(D3::default(), D3::ZERO);
    }
}
