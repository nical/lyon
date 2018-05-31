use std::marker::PhantomData;
use std::cmp;
use std::ops;
use std::fmt;
use std::hash;
use std::convert;
use std::i32;

pub struct _1;
pub struct _2;
pub struct _3;
pub struct _4;
pub struct _5;
pub struct _6;
pub struct _7;
pub struct _8;
pub struct _9;
pub struct _10;
pub struct _11;
pub struct _12;
pub struct _13;
pub struct _14;
pub struct _15;
pub struct _16;
pub struct _24;
pub struct _32;

pub trait FractionalBits {
    fn bits() -> u32;
}
impl FractionalBits for _1 {
    #[inline]
    fn bits() -> u32 { 1 }
}
impl FractionalBits for _2 {
    #[inline]
    fn bits() -> u32 { 2 }
}
impl FractionalBits for _3 {
    #[inline]
    fn bits() -> u32 { 3 }
}
impl FractionalBits for _4 {
    #[inline]
    fn bits() -> u32 { 4 }
}
impl FractionalBits for _5 {
    #[inline]
    fn bits() -> u32 { 5 }
}
impl FractionalBits for _6 {
    #[inline]
    fn bits() -> u32 { 6 }
}
impl FractionalBits for _7 {
    #[inline]
    fn bits() -> u32 { 7 }
}
impl FractionalBits for _8 {
    #[inline]
    fn bits() -> u32 { 8 }
}
impl FractionalBits for _9 {
    #[inline]
    fn bits() -> u32 { 9 }
}
impl FractionalBits for _10 {
    #[inline]
    fn bits() -> u32 { 10 }
}
impl FractionalBits for _11 {
    #[inline]
    fn bits() -> u32 { 11 }
}
impl FractionalBits for _12 {
    #[inline]
    fn bits() -> u32 { 12 }
}
impl FractionalBits for _13 {
    #[inline]
    fn bits() -> u32 { 13 }
}
impl FractionalBits for _14 {
    #[inline]
    fn bits() -> u32 { 14 }
}
impl FractionalBits for _15 {
    #[inline]
    fn bits() -> u32 { 15 }
}
impl FractionalBits for _16 {
    #[inline]
    fn bits() -> u32 { 16 }
}
impl FractionalBits for _24 {
    #[inline]
    fn bits() -> u32 { 24 }
}
impl FractionalBits for _32 {
    #[inline]
    fn bits() -> u32 { 32 }
}

/// A 32 fixed point number.
/// The size of the fractional is defined by the type parameter F.
pub struct Fp32<F = _16> {
    bits: i32,
    _fract: PhantomData<F>,
}

/// A 64 bits fixed point number.
/// The size of the fractional is defined by the type parameter F.
pub struct Fp64<F = _24> {
    bits: i64,
    _fract: PhantomData<F>,
}

macro_rules! impl_fixed_point {
    ($name:ident: $bits_type:ty) => {

        impl<F: FractionalBits> $name<F> {

            /// Returns the internal representation.
            ///
            /// This internal represenataion can be used for computations to avoid bit-shifting
            /// between each operation. The number of divisions should be equal to the number of
            /// multiplications performed in order to balance out the bit shifts that were skipped.
            #[inline]
            pub fn raw(self) -> $bits_type { self.bits }

            #[inline]
            pub fn from_raw(bits: $bits_type) -> Self { $name { bits: bits, _fract: PhantomData } }

            #[inline]
            pub fn zero() -> Self { Self::from_raw(0) }

            #[inline]
            pub fn is_zero(self) -> bool { self.bits == 0 }

            /// Smallest increment that can be reresented with this type.
            #[inline]
            pub fn epsilon() -> Self { Self::from_raw(1) }

            /// Converts from a 32 bits floating point value.
            #[inline]
            pub fn from_f32(val: f32) -> Self { Self::from_raw((val * (1 << F::bits()) as f32) as $bits_type) } // TODO

            /// Converts to a 32 bits floating point value.
            #[inline]
            pub fn to_f32(self) -> f32 { self.bits as f32 / (1 << F::bits()) as f32 } // TODO

            /// Converts from a 64 bits floating point value.
            #[inline]
            pub fn from_f64(val: f64) -> Self { Self::from_raw((val * f64::from(1 << F::bits())) as $bits_type) } // TODO

            /// Converts to a 64 bits floating point value.
            #[inline]
            pub fn to_f64(self) -> f64 { self.bits as f64 / f64::from(1 << F::bits()) } // TODO

            /// Returns 1 if the number of positive, -1 if it is negative.
            #[inline]
            pub fn sign(self) -> $bits_type { self.bits / self.bits.abs() } // TODO

            /// Returns the result of self % other.
            #[inline]
            pub fn rem(self, other: Self) -> Self { Self::from_raw(self.bits % other.bits) }

            /// Returns the lowest of the two values.
            #[inline]
            pub fn min(self, other: Self) -> Self { Self::from_raw(cmp::min(self.bits, other.bits)) }

            /// Returns the highest of the two values.
            #[inline]
            pub fn max(self, other: Self) -> Self { Self::from_raw(cmp::max(self.bits, other.bits)) }

            /// Returns the lowest and highest of the two values in order.
            #[inline]
            pub fn min_max(self, other: Self) -> (Self, Self) {
                if self.bits < other.bits { (self, other) } else { (other, self) }
            }

            /// Returns the absolute value of this number.
            #[inline]
            pub fn abs(self) -> Self { Self::from_raw(self.bits.abs()) }

            /// Returns the same number with a different fractional precision.
            #[inline]
            pub fn to_fixed<NewF: FractionalBits>(self) -> $name<NewF> {
                if F::bits() == NewF::bits() { $name::from_raw(self.bits) }
                else if F::bits() < NewF::bits() { $name::from_raw(self.bits << (NewF::bits() - F::bits())) }
                else { $name::from_raw(self.bits >> (F::bits() - NewF::bits())) }
            }
        }


        impl<F> Copy for $name<F> {}

        impl<F> Clone for $name<F> { fn clone(&self) -> Self { *self } }

        impl<F> PartialEq for $name<F> {
            #[inline]
            fn eq(&self, other: &Self) -> bool { self.bits == other.bits }
        }

        impl<F> Eq for $name<F> {}

        impl<F:FractionalBits> fmt::Debug for $name<F> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.to_f64().fmt(f) }
        }

        impl<F:FractionalBits> fmt::Display for $name<F> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.to_f64().fmt(f) }
        }

        impl<F> hash::Hash for $name<F> {
            fn hash<H: hash::Hasher>(&self, h: &mut H) {
                self.bits.hash(h);
            }
        }

        impl<F: FractionalBits> PartialOrd for $name<F> {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
                self.bits.partial_cmp(&other.bits)
            }
        }

        impl<F: FractionalBits> Ord for $name<F> {
            #[inline]
            fn cmp(&self, other: &Self) -> cmp::Ordering { self.bits.cmp(&other.bits) }
        }

        impl<F: FractionalBits> ops::Add<$name<F>> for $name<F> {
            type Output = Self;
            #[inline]
            fn add(self, other: Self) -> Self {
                $name::from_raw(self.bits + other.bits)
            }
        }

        impl<F: FractionalBits> ops::Sub<$name<F>> for $name<F> {
            type Output = Self;
            #[inline]
            fn sub(self, other: Self) -> Self {
                $name::from_raw(self.bits - other.bits)
            }
        }

        impl<F: FractionalBits> ops::Mul<$name<F>> for $name<F> {
            type Output = Self;
            #[inline]
            #[cfg_attr(feature = "cargo-clippy", allow(suspicious_arithmetic_impl))]
            fn mul(self, other: Self) -> Self {
                $name::from_raw(((i64::from(self.bits) * i64::from(other.bits)) >> F::bits()) as $bits_type)
            }
        }

        impl<F: FractionalBits> ops::Neg for $name<F> {
            type Output = Self;
            #[inline]
            fn neg(self) -> Self { $name::from_raw(-self.bits) }
        }

        impl<F: FractionalBits> ops::Mul<$bits_type> for $name<F> {
            type Output = $name<F>;
            #[inline]
            fn mul(self, other: $bits_type) -> Self {
                $name::from_raw(self.bits * other)
            }
        }

        impl<F: FractionalBits> ops::Div<$bits_type> for $name<F> {
            type Output = $name<F>;
            #[inline]
            fn div(self, other: $bits_type) -> Self {
                $name::from_raw(self.bits / other)
            }
        }

        impl<F: FractionalBits> ops::AddAssign<$name<F>> for $name<F> {
            #[inline]
            fn add_assign(&mut self, other: Self) { self.bits += other.bits; }
        }

        impl<F: FractionalBits> ops::SubAssign<$name<F>> for $name<F> {
            #[inline]
            fn sub_assign(&mut self, other: Self) { self.bits -= other.bits; }
        }

        impl<F: FractionalBits> convert::Into<f32> for $name<F> {
            #[inline]
            fn into(self) -> f32 { self.to_f32() }
        }

        impl<F: FractionalBits> convert::From<f32> for $name<F> {
            #[inline]
            fn from(val: f32) -> Self { Self::from_f32(val) }
        }

        impl<F: FractionalBits> convert::Into<f64> for $name<F> {
            #[inline]
            fn into(self) -> f64 { self.to_f64() }
        }

        impl<F: FractionalBits> convert::From<f64> for $name<F> {
            #[inline]
            fn from(val: f64) -> Self { Self::from_f64(val) }
        }
    }
}

impl_fixed_point!(Fp64: i64);

impl_fixed_point!(Fp32: i32);

impl<F: FractionalBits> Fp32<F> {
    #[inline]
    pub fn one() -> Self { Fp32::from_i32(1) }

    #[inline]
    pub fn from_i32(val: i32) -> Self { Fp32::from_raw(val << F::bits()) }

    /// Convert to an i32, truncating the fractional part
    #[inline]
    pub fn truncate_to_i32(self) -> i32 { self.bits >> F::bits() }

    /// Computes the self * m / d in one go, avoid the precision loss from shifting bits back
    /// and forth.
    #[inline]
    pub fn mul_div(self, m: Self, d: Self) -> Self {
        Fp32::from_raw((i64::from(self.bits) * i64::from(m.bits) / i64::from(d.bits)) as i32)
    }

    #[inline]
    pub fn min_val() -> Self { Fp32::from_raw(i32::MIN) }

    #[inline]
    pub fn max_val() -> Self { Fp32::from_raw(i32::MAX) }

    /// Casts into a 64 bits fixed point number.
    #[inline]
    pub fn to_fp64<NewF: FractionalBits>(self) -> Fp64<NewF> {
        let tmp: Fp64<F> = Fp64::from_raw(i64::from(self.bits));
        tmp.to_fixed()
    }

    // This is nice in theory but overflows with any Fp32<16> * Fp32<16> operation so it's not
    // so useful in practice.
    //#[inline]
    //pub fn multiply<F2: FractionalBits>(self, other: Fp32<F2>) -> Fp32<Multiplication<F, F2>> {
    //    Fp32::from_raw(self.bits * other.bits)
    //}
}

impl<F: FractionalBits> ops::Div<Fp32<F>> for Fp32<F> {
    type Output = Self;
    #[inline]
    #[cfg_attr(feature = "cargo-clippy", allow(suspicious_arithmetic_impl))]
    fn div(self, other: Self) -> Self {
        let self64 = i64::from(self.bits) << 32;
        let other64 = i64::from(other.bits);
        Fp32::from_raw(((self64 / other64) >> (32 - F::bits())) as i32)
    }
}

impl<F: FractionalBits> Fp64<F> {
    #[inline]
    pub fn one() -> Self { Fp64::from_i64(1) }

    #[inline]
    pub fn from_i64(val: i64) -> Self { Fp64::from_raw(val << F::bits()) }

    /// Convert to an i64, truncating the fractional part
    #[inline]
    pub fn truncate_to_i64(self) -> i64 { self.bits >> F::bits() }

    /// Computes the self * m / d in one go, avoid the precision loss from shifting bits back
    /// and forth.
    #[inline]
    pub fn mul_div(self, m: Self, d: Self) -> Self { Fp64::from_raw(self.bits * m.bits / d.bits) }

    /// Casts into a 32 bits fixed point number.
    pub fn to_fp32<NewF: FractionalBits>(self) -> Fp32<NewF> {
        let tmp = self.to_fixed::<NewF>();
        Fp32::from_raw(tmp.bits as i32)
    }
}

impl<F: FractionalBits> ops::Div<Fp64<F>> for Fp64<F> {
    type Output = Self;
    #[inline]
    #[cfg_attr(feature = "cargo-clippy", allow(suspicious_arithmetic_impl))]
    fn div(self, other: Self) -> Self { Fp64::from_raw((self.bits / other.bits) << F::bits()) }
}


//pub struct Multiplication<A, B> { _marker: PhantomData<(A, B)> }
//impl<A: FractionalBits, B:FractionalBits> FractionalBits for Multiplication<A, B> {
//    fn bits() -> u32 { A::bits() + B::bits() }
//}
//
//pub struct Division<A, B> { _marker: PhantomData<(A, B)> }
//impl<A: FractionalBits, B:FractionalBits> FractionalBits for Division<A, B> {
//    fn bits() -> u32 { A::bits() - B::bits() }
//}

#[test]
fn test_fp32() {
    pub fn fixed<F: FractionalBits>(val: f32) -> Fp32<F> { Fp32::from_f32(val) }


    pub type Fp = Fp32<_16>;

    let zero = fixed(0.0);
    let one = fixed(1.0);
    let minus_one = Fp::from_f32(-1.0);
    let ten = Fp::from_f32(10.0);
    let a = Fp::from_f32(1.5);

    println!("0: {:?} | {}", zero, zero.bits);
    println!("1: {:?} | {}", one, one.bits);
    println!("-1: {:?} | {}", minus_one, minus_one.bits);
    println!("10: {:?} | {}", ten, ten.bits);
    println!("1.5: {:?} | {}", a, a.bits);
    println!("1.5 * 10: {:?} | {}", a * ten, (a * ten).bits);
    println!(
        "0.5 / 2: {:?} | {}",
        Fp::from_f32(0.5) / fixed(2.0),
        (Fp::from_f32(0.5) / fixed(2.0)).bits
    );
    println!(
        "-0.5 / -2: {:?} | {}",
        Fp::from_f32(-0.5) / fixed(-2.0),
        (Fp::from_f32(-0.5) / fixed(-2.0)).bits
    );
    println!(
        "-0.5 / 2: {:?} | {}",
        Fp::from_f32(-0.5) / fixed(2.0),
        (Fp::from_f32(-0.5) / fixed(2.0)).bits
    );
    println!(
        "-0.5 * 2: {:?} | {}",
        Fp::from_f32(-0.5) * fixed(2.0),
        (Fp::from_f32(-0.5) * fixed(2.0)).bits
    );
    println!(
        "0.5 / -2: {:?} | {}",
        Fp::from_f32(0.5) / fixed(-2.0),
        (Fp::from_f32(0.5) / fixed(-2.0)).bits
    );
    println!("bits {}", 1 << 8);

    assert_eq!(Fp::from_i32(1), one);
    assert_eq!(Fp::one(), one);
    assert_eq!(Fp::from_i32(-1), minus_one);
    assert_eq!(Fp::from_i32(0), zero);
    assert_eq!(a.truncate_to_i32(), 1);
    assert_eq!(one.sign(), 1);
    assert_eq!(minus_one.sign(), -1);
    assert_eq!(a.rem(one), Fp::from_f32(0.5));
    assert_eq!(-one, minus_one);
    assert_eq!(one.to_fixed::<_8>().truncate_to_i32(), one.truncate_to_i32());
    assert_eq!(one.to_fixed::<_2>().truncate_to_i32(), one.truncate_to_i32());
    //assert_eq!(one.multiply(one).truncate_to_i32(), 1);

    println!("min {} max {}", Fp::min_val(), Fp::max_val());
}
