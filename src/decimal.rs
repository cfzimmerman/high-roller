use core::f32;
use core::ops::{Add, Sub};

use num_traits::{CheckedAdd, CheckedSub, WrappingAdd, WrappingSub};

#[non_exhaustive]
pub enum DecimalErr {
    Lossy,
    BadPrecision,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Decimal32<const PRECISION: u32>(i32);

const fn scalar(precision: u32) -> u32 {
    10u32.pow(precision)
}

impl<const PRECISION: u32> Decimal32<PRECISION> {
    pub const ZERO: Self = Self(0);

    pub const fn cast_from(value: f32) -> Self {
        Self((value * self::scalar(PRECISION) as f32) as i32)
    }

    pub const fn get(self) -> f32 {
        self.0 as f32 / self::scalar(PRECISION) as f32
    }
}

impl<const P: u32> TryFrom<f32> for Decimal32<P> {
    type Error = DecimalErr;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        let dec = Self::cast_from(value);
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
        if Decimal32::<P>::cast_from(float) != value {
            return Err(DecimalErr::Lossy);
        }
        Ok(float)
    }
}

impl<const P: u32> Add for Decimal32<P> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.add(rhs.0))
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

impl<const P: u32> Sub for Decimal32<P> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.sub(rhs.0))
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

// TODO(corzimmerman): test this
// Without loss
// With loss
// Addition
// Wrapping addition
// Checked addition
// Subtraction
// Wrapping subtraction
// Checked Subtraction

#[cfg(test)]
mod decimal_tests {
    use crate::decimal::Decimal32;

    /// "Precision = 3" shorthand for diminished verbosity.
    type P3 = Decimal32<3>;

    fn assert_eq_f32(left: f32, right: f32, precision: u32) {
        assert!(
            (left - right).abs() < super::scalar(precision) as f32,
            "equality failed: {left:?} != {right:?}"
        );
    }

    #[test]
    fn without_loss() {
        const NUM1: P3 = P3::cast_from(0.001);
        const NUM2: P3 = P3::cast_from(7.12);
        let sum = NUM1 + NUM2;
        assert_eq_f32(sum.get(), 7.121, 3);
    }
}
