use core::f32;
use core::fmt::{Debug, Display};
use core::ops::{Add, Neg, Sub};

use num_traits::{CheckedAdd, CheckedSub, WrappingAdd, WrappingSub};
use thiserror::Error;

/// A `Decimal32` type with one significant figure
/// after the decimal point.
///
/// ```
/// use high_roller::decimal::D1;
///
/// assert_eq!(D1::MAX.get(), 214748364.7_f64);
/// assert_eq!(D1::MIN.get(), -214748364.8_f64);
/// assert_eq!(D1::MIN_UNIT.get(), 0.1_f64);
/// ```
pub type D1 = Decimal32<1>;

/// A `Decimal32` type with two significant figures
/// after the decimal point.
///
/// ```
/// use high_roller::decimal::D2;
///
/// assert_eq!(D2::MAX.get(), 21474836.47_f64);
/// assert_eq!(D2::MIN.get(), -21474836.48_f64);
/// assert_eq!(D2::MIN_UNIT.get(), 0.01_f64);
/// ```
pub type D2 = Decimal32<2>;

/// A `Decimal32` type with three significant figures
/// after the decimal point.
///
/// ```
/// use high_roller::decimal::D3;
///
/// assert_eq!(D3::MAX.get(), 2147483.647_f64);
/// assert_eq!(D3::MIN.get(), -2147483.648_f64);
/// assert_eq!(D3::MIN_UNIT.get(), 0.001_f64);
/// ```
pub type D3 = Decimal32<3>;

/// A `Decimal32` type with four significant figures
/// after the decimal point.
///
/// ```
/// use high_roller::decimal::D4;
///
/// assert_eq!(D4::MAX.get(), 214748.3647_f64);
/// assert_eq!(D4::MIN.get(), -214748.3648_f64);
/// assert_eq!(D4::MIN_UNIT.get(), 0.0001_f64);
/// ```
pub type D4 = Decimal32<4>;

/// A `Decimal32` type with five significant figures
/// after the decimal point.
///
/// ```
/// use high_roller::decimal::D5;
///
/// assert_eq!(D5::MAX.get(), 21474.83647_f64);
/// assert_eq!(D5::MIN.get(), -21474.83648_f64);
/// assert_eq!(D5::MIN_UNIT.get(), 0.00001_f64);
/// ```
pub type D5 = Decimal32<5>;

/// A `Decimal32` type with six significant figures
/// after the decimal point.
///
/// ```
/// use high_roller::decimal::D6;
///
/// assert_eq!(D6::MAX.get(), 2147.483647_f64);
/// assert_eq!(D6::MIN.get(), -2147.483648_f64);
/// assert_eq!(D6::MIN_UNIT.get(), 0.000001_f64);
/// ```
pub type D6 = Decimal32<6>;

/// A `Decimal32` type with seven significant figures
/// after the decimal point.
///
/// ```
/// use high_roller::decimal::D7;
///
/// assert_eq!(D7::MAX.get(), 214.7483647_f64);
/// assert_eq!(D7::MIN.get(), -214.7483648_f64);
/// assert_eq!(D7::MIN_UNIT.get(), 0.0000001_f64);
/// ```
pub type D7 = Decimal32<7>;

/// A `Decimal32` type with eight significant figures
/// after the decimal point.
///
/// ```
/// use high_roller::decimal::D8;
///
/// assert_eq!(D8::MAX.get(), 21.47483647_f64);
/// assert_eq!(D8::MIN.get(), -21.47483648_f64);
/// assert_eq!(D8::MIN_UNIT.get(), 0.00000001_f64);
/// ```
pub type D8 = Decimal32<8>;

/// A `Decimal32` type with nine significant figures
/// after the decimal point.
///
/// ```
/// use high_roller::decimal::D9;
///
/// assert_eq!(D9::MAX.get(), 2.147483647_f64);
/// assert_eq!(D9::MIN.get(), -2.147483648_f64);
/// assert_eq!(D9::MIN_UNIT.get(), 0.000000001_f64);
/// ```
pub type D9 = Decimal32<9>;

/// # Decimal32
///
/// This is a transparent wrapper over an i32.
/// A const generic declares the number of places
/// after the decimal point.
///
/// The motivation for such a type is providing lossless
/// arithmetic guarantees like in the example below.
///
/// ```
/// use high_roller::decimal::D9;
/// use num_traits::{CheckedAdd, WrappingAdd, WrappingSub};
///
/// const SMALL: f64 = 0.111000111;
/// const LARGE: f64 = 2.147483647;
///
/// const CHECKED_SMALL: D9 = D9::checked(SMALL).unwrap();
/// const CHECKED_LARGE: D9 = D9::checked(LARGE).unwrap();
///
/// // Parity with lossless operations
/// let sum = const { D9::checked(1.).unwrap() }.checked_add(&CHECKED_SMALL);
/// assert_eq!(sum.unwrap().get(), 1. + SMALL, "Result fits in f64");
///
/// // Checked operations prevent overflow
/// let lossy = CHECKED_LARGE.checked_add(&CHECKED_SMALL);
/// assert_eq!(lossy, None, "Result overflows i32");
/// assert_ne!(LARGE + SMALL - LARGE, SMALL);
///
/// // Wrapping operations enable loss recovery
/// let wrapped = CHECKED_LARGE.wrapping_add(&CHECKED_SMALL);
/// assert_eq!(wrapped.wrapping_sub(&CHECKED_LARGE), CHECKED_SMALL);
/// ```
///
/// # Design
///
/// There are different ways to represent a floating point
/// number with wrapping and saturating semantics.
/// This design basically takes the bounds of an i32 and
/// sticks a decimal point somewhere. So the type itself
/// serves primarily for self-documentation and convenience.
///
/// IEEE 754 floating point debauchery keeps the lossless
/// range of an f32 in signed 2^24.
/// So an equally valid design could lock the inner value within
/// that range and use that bound for wrapping, saturating, and
/// checked operations.
/// The benefit is that `get` could return an f32 without loss
/// of precision. The cost is hardware support for arithmetic.
/// Since wrapping arithmetic is the primary motivation for
/// Decimal32, this was not chosen.
///
/// A Decimal64 type is not (yet) exposed because at that point,
/// the [Decimal](https://crates.io/crates/rust_decimal) crate
/// might be a better fit for your use case.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Decimal32<const PRECISION: u32>(i32);

/// Enumerates the possible errors `Decimal` operations may return.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DecimalErr {
    #[error("The attempted operation would cause a loss of precision.")]
    Lossy,
}

impl<const PRECISION: u32> Decimal32<PRECISION> {
    const _PRECISION_CHECK: () = assert!(
        PRECISION <= 9,
        "PRECISION must be <= 9; 10ePRECISION would overflow u32"
    );

    pub const ZERO: Self = Self(0);

    /// The greatest positive value this type can contain.
    pub const MAX: Self = Self(i32::MAX);

    /// The most negative value this type can contain.
    pub const MIN: Self = Self(i32::MIN);

    /// The smallest positive value this type can contain.
    pub const MIN_UNIT: Self = Self(1);

    /// Constructor that accepts any input. Truncates toward zero when
    /// the input has more decimal places than `PRECISION`. Use [`Self::checked`]
    /// to detect when precision is lost.
    ///
    /// This function takes an `f64` to prevent sneaky loss of precision.
    /// `f32` only has a 24-bit mantissa, so values between 2^24 and
    /// and 2^31 require an f64 to be constructed.
    ///
    /// ```
    /// use high_roller::decimal::D2;
    ///
    /// let num = D2::cast(0.125_f64);
    /// assert_eq!(num.get(), 0.12_f64);
    /// ```
    ///
    /// If this `f64` situation is annoying for your use case,
    /// you can still escape it entirely at compile time.
    ///
    /// ```
    /// use high_roller::decimal::D3;
    ///
    /// const MY_F32: f32 = D3::cast(0.321_f64).get() as f32;
    /// assert_eq!(MY_F32, 0.321);
    /// ```
    #[must_use]
    #[inline]
    pub const fn cast(value: f64) -> Self {
        Self((value * self::scalar(PRECISION) as f64) as i32)
    }

    /// Const constructor that prevents loss of precision
    /// from the input value.
    ///
    /// ### Succeeds
    ///
    /// ```
    /// use high_roller::decimal::D5;
    ///
    /// const GOOD: D5 = D5::checked(-100.12345).unwrap();
    /// ```
    ///
    /// ### Fails
    ///
    /// ```compile_fail
    /// use high_roller::decimal::D9;
    ///
    /// const BAD: D9 = D9::checked(-100.)
    ///     .expect("There isn't space in 32 bits for 9 decimal places after -100");
    /// ```
    #[must_use]
    pub const fn checked(value: f64) -> Option<Self> {
        let dec = Self::cast(value);
        if dec.get() != value {
            return None;
        }
        Some(dec)
    }

    /// Returns the inner value as an f64. This conversion is lossless
    /// because f64's 53-bit mantissa can represent every i32 value exactly.
    ///
    /// For f32 output use [`f32::try_from`], which returns `Err(Lossy)` when
    /// the inner value exceeds f32's 24-bit mantissa.
    ///
    /// ```
    /// use high_roller::decimal::D1;
    ///
    /// assert_eq!(D1::cast(1.0_f64).get(), 1.0_f64);
    /// ```
    #[must_use]
    #[inline]
    pub const fn get(self) -> f64 {
        self.0 as f64 / self::scalar(PRECISION) as f64
    }
}

/// Helper function for the scaling constant on an
/// inner Decimal value.
#[inline]
const fn scalar(precision: u32) -> u32 {
    10u32.pow(precision)
}

impl<const P: u32> Debug for Decimal32<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Decimal32<{}>({})", P, self.get())
    }
}

impl<const P: u32> Display for Decimal32<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.get())
    }
}

impl<const P: u32> TryFrom<f32> for Decimal32<P> {
    type Error = DecimalErr;

    /// Constructs a `Decimal32` from an `f32` or returns `Err(DecimalErr::Lossy)`
    /// if the conversion would lose precision. This might occur if the input
    /// literal specifies more decimal places than the underlying`Decimal32` type.
    ///
    /// ```should_panic
    /// use high_roller::decimal::D2;
    ///
    /// D2::try_from(0.123).expect("resulting decimal is 0.12");
    /// ```
    ///
    /// But take care not to lose precision when constructing the
    /// f32 input into this function. Since f32 has a 24-bit mantissa,
    /// it cannot represent some values that Decimal32 can.
    ///
    /// In the example below, rustc abbreviates the float before this
    /// function even sees it. [`Decimal32::cast`] solves this case by
    /// using an f64 constructor.
    ///
    /// ```
    /// use high_roller::decimal::D9;
    ///
    /// const INPUT: f32 = 2.147483647;
    /// let expected = D9::try_from(INPUT).unwrap();
    ///
    /// assert_eq!(INPUT, f32::try_from(expected).unwrap());
    /// assert_eq!(INPUT, 2.1474836, "two places were dropped");
    /// ```
    ///
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        // Use f32 arithmetic for the scale step: the caller's value is already
        // an f32, and f32 multiplication may round (e.g. 7.12f32 * 1000 → 7120)
        // in ways that f64 arithmetic would not.
        let dec = Self((value * self::scalar(P) as f32) as i32);
        if dec.get() as f32 != value {
            return Err(DecimalErr::Lossy);
        }
        Ok(dec)
    }
}

impl<const P: u32> TryFrom<Decimal32<P>> for f32 {
    type Error = DecimalErr;

    /// Converts to f32. Returns `Err(Lossy)` when the inner value exceeds
    /// f32's 24-bit mantissa. Use [`Decimal32::get`] if unchecked lossless
    /// output is required.
    fn try_from(value: Decimal32<P>) -> Result<Self, Self::Error> {
        if value.0 as Self as i32 != value.0 {
            return Err(DecimalErr::Lossy);
        }
        Ok(value.get() as Self)
    }
}

impl<const P: u32> From<Decimal32<P>> for f64 {
    fn from(val: Decimal32<P>) -> Self {
        val.get()
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

impl<const P: u32> Neg for Decimal32<P> {
    type Output = Self;

    /// Uses wrapping negation.
    fn neg(self) -> Self::Output {
        Self(self.0.wrapping_neg())
    }
}

#[cfg(test)]
impl<const P: u32> From<Decimal32<P>> for num_bigint::BigInt {
    fn from(val: Decimal32<P>) -> Self {
        Self::from(val.0)
    }
}

#[cfg(test)]
impl<const P: u32> TryFrom<&num_bigint::BigInt> for Decimal32<P> {
    type Error = ();

    fn try_from(val: &num_bigint::BigInt) -> Result<Self, Self::Error> {
        let n: i32 = val.try_into().map_err(|_| ())?;
        Ok(Self(n))
    }
}

#[allow(clippy::missing_panics_doc)]
#[allow(clippy::expect_used)]
#[cfg(test)]
pub mod decimal_tests {
    use num_traits::{CheckedAdd, CheckedSub, WrappingAdd, WrappingSub};

    use crate::decimal::{Decimal32, DecimalErr, D3};

    /// Checks equality within one unit of the given precision (i.e. tolerance = 10^-precision).
    pub fn assert_eq_f64(left: f64, right: f64, precision: u32) {
        let tolerance = 1.0 / super::scalar(precision) as f64;
        assert!(
            (left - right).abs() < tolerance,
            "equality failed: {left:?} != {right:?} (tolerance {tolerance})"
        );
    }

    // --- Conversion ---

    /// Values exactly representable at P=3 survive a cast_from / get round-trip.
    #[test]
    fn cast_from_exact() {
        assert_eq_f64(D3::cast(0.001).get(), 0.001, 3);
        assert_eq_f64(D3::cast(7.120).get(), 7.120, 3);
        assert_eq_f64(D3::cast(-3.500).get(), -3.500, 3);
    }

    /// cast truncates toward zero rather than rounding.
    #[test]
    fn cast_truncates() {
        // 0.9999 * 1 = 0.9999, cast to i32 truncates to 0
        let d = Decimal32::<0>::cast(0.9999_f64);
        assert_eq!(d.get(), 0.0_f64);
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
        assert_eq_f64(D3::cast(1. / 3.).get(), 0.333, 3);
    }

    /// TryFrom<Decimal32<P>> for f32 succeeds for values that round-trip cleanly.
    #[test]
    fn decimal_to_f32_ok() {
        let d = D3::cast(7.120);
        let f: f32 = f32::try_from(d).expect("should round-trip");
        assert_eq_f64(f as f64, 7.120, 3);
    }

    // --- Arithmetic ---

    /// Basic addition produces the correct sum.
    #[test]
    fn add_basic() {
        let sum = D3::cast(0.001) + D3::cast(7.120);
        assert_eq_f64(sum.get(), 7.121, 3);
    }

    /// Adding a negative value crosses zero correctly.
    #[test]
    fn add_negative() {
        let sum = D3::cast(1.000) + D3::cast(-3.500);
        assert_eq_f64(sum.get(), -2.500, 3);
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
        assert_eq_f64(diff.get(), 7.120, 3);
    }

    /// Subtraction can produce a negative result.
    #[test]
    fn sub_to_negative() {
        let diff = D3::cast(1.000) - D3::cast(3.500);
        assert_eq_f64(diff.get(), -2.500, 3);
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

    // --- Neg ---

    /// Negating a positive value gives the corresponding negative.
    #[test]
    fn neg_basic() {
        let x = D3::cast(4.200);
        assert_eq!(-x, D3::cast(-4.200));
        assert_eq!(-(-x), x);
    }

    /// Negating i32::MIN wraps to i32::MIN (wrapping_neg behaviour).
    #[test]
    fn neg_min_wraps() {
        let min = Decimal32::<0>(i32::MIN);
        assert_eq!(-min, min);
    }

    // --- TryFrom<Decimal32<P>> for f32 ---

    /// TryFrom<Decimal32> for f32 returns Err when the inner i32 exceeds f32's
    /// 24-bit mantissa (~16.7 million), causing the get() → cast() round-trip to
    /// land on a different inner value.
    #[test]
    fn decimal_to_f32_lossy() {
        // 2^24 + 1 = 16_777_217 cannot be represented exactly as f32 (rounds to
        // 16_777_216), so cast(get(d)) returns a different inner value.
        let d = Decimal32::<1>(16_777_217_i32);
        assert!(f32::try_from(d).is_err());

        let d_neg = Decimal32::<1>(-16_777_217_i32);
        assert!(f32::try_from(d_neg).is_err());
    }
}
