
use cgmath;
use std;

// unit should be an empty struct but for some reason rustc requires the unit
// to implement all of the numeric traits (~200 lines of silly boilerplate) so
// lets reuse types that alredy do implement those traits for now.
macro_rules! declare_unit (
    ($module:ident, $silly_work_around_type: ident) => (
        pub mod $module {
            use super::Float32;
            use cgmath;
            pub type Unit = $silly_work_around_type;
            pub type Vec2 = cgmath::vector::Vector2<Float32<Unit>>;
            pub type Vec3 = cgmath::vector::Vector3<Float32<Unit>>;
            pub type Vec4 = cgmath::vector::Vector4<Float32<Unit>>;
            pub type Mat3 = cgmath::matrix::Matrix3<Float32<Unit>>;
            pub type Mat4 = cgmath::matrix::Matrix4<Float32<Unit>>;
            fn float32(v: f32) -> Float32<Unit> { Float32 { val: v} }
            pub mod Mat3 {
                pub use super::Mat3;
                pub use cgmath;
                pub fn identity() -> Mat3 { cgmath::matrix::Matrix3::identity() }
            }
            pub mod Mat4 {
                pub use super::Mat4;
                pub use cgmath;
                pub fn identity() -> Mat4 { cgmath::matrix::Matrix4::identity() }
            }
        }
    )
)

declare_unit!(world, f32)
declare_unit!(pixels, f64)

#[deriving(Show)]
struct Float32<U> {
    val: f32
}

fn float_unit<U>(val: f32) -> Float32<U> { Float32 { val: val } }
fn option_float_unit<U>() -> Option<Float32<U>> { None }
fn option_float() -> Option<f32> { None }

impl<U: std::fmt::Show> cgmath::num::BaseNum for Float32<U> {}

impl<U> std::num::Num for Float32<U> {}

impl<U> std::num::Primitive for Float32<U> {}

impl<U> std::num::ToPrimitive for Float32<U> {
    fn to_i64(&self) -> Option<i64> { self.val.to_i64() }
    fn to_u64(&self) -> Option<u64> { self.val.to_u64() }
    fn to_int(&self) -> Option<int> { self.val.to_int() }
    fn to_i8(&self) -> Option<i8> { self.val.to_i8() }
    fn to_i16(&self) -> Option<i16> { self.val.to_i16() }
    fn to_i32(&self) -> Option<i32> { self.val.to_i32() }
    fn to_uint(&self) -> Option<uint> { self.val.to_uint() }
    fn to_u8(&self) -> Option<u8> { self.val.to_u8() }
    fn to_u16(&self) -> Option<u16> { self.val.to_u16() }
    fn to_u32(&self) -> Option<u32> { self.val.to_u32() }
    fn to_f32(&self) -> Option<f32> { self.val.to_f32() }
    fn to_f64(&self) -> Option<f64> { self.val.to_f64() }
}

impl<U> std::num::NumCast for Float32<U> {
    fn from<T: ToPrimitive>(n: T) -> Option<Float32<U>> {
        let v: Option<f32> = NumCast::from(n);
        match v {
            Some(r) => Some(float_unit(r)),
            None => None
        }
    }
}

impl<U> std::num::Bounded for Float32<U> {
    fn min_value() -> Float32<U> { float_unit(std::num::Bounded::min_value()) }
    fn max_value() -> Float32<U> { float_unit(std::num::Bounded::max_value()) }
}

impl<U> std::clone::Clone for Float32<U> {
    fn clone(&self) -> Float32<U> { float_unit(self.val.clone()) }
    fn clone_from(&mut self, source: &Float32<U>) { self.val.clone_from(&source.val); }
}
    
impl<U> std::ops::Add<Float32<U>, Float32<U>> for Float32<U> {
    fn add(&self, rhs: &Float32<U>) -> Float32<U> { float_unit(self.val.add(&rhs.val)) }    
}

impl<U> std::ops::Sub<Float32<U>, Float32<U>> for Float32<U> {
    fn sub(&self, rhs: &Float32<U>) -> Float32<U> { float_unit(self.val.sub(&rhs.val)) }
}

impl<U> std::ops::Mul<Float32<U>, Float32<U>> for Float32<U> {
    fn mul(&self, rhs: &Float32<U>) -> Float32<U> { float_unit(self.val.mul(&rhs.val)) }
}

impl<U> std::ops::Div<Float32<U>, Float32<U>> for Float32<U> {
    fn div(&self, rhs: &Float32<U>) -> Float32<U> { float_unit(self.val.div(&rhs.val)) }
}

impl<U> std::ops::Neg<Float32<U>> for Float32<U> {
    fn neg(&self) -> Float32<U> { float_unit(self.val.neg()) }
}

impl<U> std::ops::Rem<Float32<U>, Float32<U>> for Float32<U> {
    fn rem(&self, rhs: &Float32<U>) -> Float32<U> { float_unit(self.val.rem(&rhs.val)) }
}

impl<U> std::num::Zero for Float32<U> {
    fn zero() -> Float32<U> { float_unit(std::num::Zero::zero()) }
    fn is_zero(&self) -> bool { self.val.is_zero() }
}

impl<U> std::num::One for Float32<U> {
    fn one() -> Float32<U> { float_unit(std::num::One::one()) }
}

impl<U> std::cmp::PartialEq for Float32<U> {
    fn eq(&self, other: &Float32<U>) -> bool { self.val.eq(&other.val) }
    fn ne(&self, other: &Float32<U>) -> bool { self.val.ne(&other.val) }    
}

impl<U> std::cmp::PartialOrd for Float32<U> {
    fn partial_cmp(&self, other: &Float32<U>) -> Option<std::cmp::Ordering> { self.val.partial_cmp(&other.val) }
    fn lt(&self, other: &Float32<U>) -> bool { self.val.lt(&other.val) }
    fn le(&self, other: &Float32<U>) -> bool { self.val.le(&other.val) }
    fn gt(&self, other: &Float32<U>) -> bool { self.val.gt(&other.val) }
    fn ge(&self, other: &Float32<U>) -> bool { self.val.ge(&other.val) }
}

impl<U> cgmath::num::PartialOrd for Float32<U> {
    fn partial_min(self, other: Float32<U>) -> Float32<U> { float_unit(self.val.min(other.val)) }
    fn partial_max(self, other: Float32<U>) -> Float32<U> { float_unit(self.val.max(other.val)) }
}

impl<U> Signed for Float32<U> {
    fn abs(&self) -> Float32<U> { float_unit(self.val.abs()) }
    fn abs_sub(&self, other: &Float32<U>) -> Float32<U> { float_unit(self.val.abs_sub(&other.val)) }
    fn signum(&self) -> Float32<U> { float_unit(self.val.signum()) }
    fn is_positive(&self) -> bool { self.val.is_positive() }
    fn is_negative(&self) -> bool { self.val.is_negative() }
}

impl<U> Float for Float32<U> {
    fn nan() -> Float32<U> { float_unit(Float::nan()) }
    fn infinity() -> Float32<U> { float_unit(Float::infinity()) }
    fn neg_infinity() -> Float32<U> { float_unit(Float::neg_infinity()) }
    fn neg_zero() -> Float32<U> { float_unit(Float::neg_zero()) }
    fn is_nan(self) -> bool { self.val.is_nan() }
    fn is_infinite(self) -> bool { self.val.is_infinite() }
    fn is_finite(self) -> bool { self.val.is_finite() }
    fn is_normal(self) -> bool { self.val.is_normal() }
    fn classify(self) -> std::num::FPCategory { self.val.classify() }
    fn mantissa_digits(unused_self: Option<Float32<U>>) -> uint { Float::mantissa_digits(option_float()) }
    fn digits(unused_self: Option<Float32<U>>) -> uint { Float::digits(option_float()) }
    fn epsilon() -> Float32<U> { float_unit(Float::epsilon()) }
    fn min_exp(unused_self: Option<Float32<U>>) -> int { Float::min_exp(option_float()) }
    fn max_exp(unused_self: Option<Float32<U>>) -> int { Float::max_exp(option_float()) }
    fn min_10_exp(unused_self: Option<Float32<U>>) -> int { Float::min_10_exp(option_float()) }
    fn max_10_exp(unused_self: Option<Float32<U>>) -> int { Float::max_10_exp(option_float()) }
    fn min_pos_value(unused_self: Option<Float32<U>>) -> Float32<U> { float_unit(Float::min_pos_value(option_float())) }
    fn integer_decode(self) -> (u64, i16, i8) { self.val.integer_decode() }
    fn floor(self) -> Float32<U> { float_unit(self.val.floor()) }
    fn ceil(self) -> Float32<U> { float_unit(self.val.ceil()) }
    fn round(self) -> Float32<U> { float_unit(self.val.round()) }
    fn trunc(self) -> Float32<U> { float_unit(self.val.trunc()) }
    fn fract(self) -> Float32<U> { float_unit(self.val.fract()) }
    fn mul_add(self, a: Float32<U>, b: Float32<U>) -> Float32<U> { float_unit(self.val.mul_add(a.val, b.val)) }
    fn recip(self) -> Float32<U> { float_unit(self.val.recip()) }
    fn powi(self, n: i32) -> Float32<U> { float_unit(self.val.powi(n)) }
    fn powf(self, n: Float32<U>) -> Float32<U> { float_unit(self.val.powf(n.val)) }
    fn sqrt2() -> Float32<U> { float_unit(Float::sqrt2()) }
    fn frac_1_sqrt2() -> Float32<U> { float_unit(Float::frac_1_sqrt2()) }
    fn sqrt(self) -> Float32<U> { float_unit(self.val.sqrt()) }
    fn rsqrt(self) -> Float32<U> { float_unit(self.val.rsqrt()) }
    fn pi() -> Float32<U> { float_unit(Float::pi()) }
    fn two_pi() -> Float32<U> { float_unit(Float::two_pi()) }
    fn frac_pi_2() -> Float32<U> { float_unit(Float::frac_pi_2()) }
    fn frac_pi_3() -> Float32<U> { float_unit(Float::frac_pi_3()) }
    fn frac_pi_4() -> Float32<U> { float_unit(Float::frac_pi_4()) }
    fn frac_pi_6() -> Float32<U> { float_unit(Float::frac_pi_6()) }
    fn frac_pi_8() -> Float32<U> { float_unit(Float::frac_pi_8()) }
    fn frac_1_pi() -> Float32<U> { float_unit(Float::frac_1_pi()) }
    fn frac_2_pi() -> Float32<U> { float_unit(Float::frac_2_pi()) }
    fn frac_2_sqrtpi() -> Float32<U> { float_unit(Float::frac_2_sqrtpi()) }
    fn e() -> Float32<U> { float_unit(Float::e()) }
    fn log2_e() -> Float32<U> { float_unit(Float::log2_e()) }
    fn log10_e() -> Float32<U> { float_unit(Float::log10_e()) }
    fn ln_2() -> Float32<U> { float_unit(Float::ln_2()) }
    fn ln_10() -> Float32<U> { float_unit(Float::ln_10()) }
    fn exp(self) -> Float32<U> { float_unit(self.val.exp()) }
    fn exp2(self) -> Float32<U> { float_unit(self.val.exp2()) }
    fn ln(self) -> Float32<U> { float_unit(self.val.ln()) }
    fn log(self, base: Float32<U>) -> Float32<U> { float_unit(self.val.log(base.val)) }
    fn log2(self) -> Float32<U> { float_unit(self.val.log2()) }
    fn log10(self) -> Float32<U> { float_unit(self.val.log10()) }
    fn to_degrees(self) -> Float32<U> { float_unit(self.val.to_degrees()) }
    fn to_radians(self) -> Float32<U> { float_unit(self.val.to_radians()) }
}

impl<U> FloatMath for Float32<U> {
    fn ldexp(x: Float32<U>, exp: int) -> Float32<U> { Float32 { val: FloatMath::ldexp(x.val, exp) } }
    fn frexp(self) -> (Float32<U>, int) { let (a, b) = self.val.frexp(); (Float32 {val: a}, b) }
    fn next_after(self, other: Float32<U>) -> Float32<U> { float_unit(self.val.next_after(other.val)) }
    fn max(self, other: Float32<U>) -> Float32<U> { float_unit(self.val.max(other.val)) }
    fn min(self, other: Float32<U>) -> Float32<U> { float_unit(self.val.min(other.val)) }
    fn cbrt(self) -> Float32<U> { float_unit(self.val.cbrt()) }
    fn hypot(self, other: Float32<U>) -> Float32<U> { float_unit(self.val.hypot(other.val)) }
    fn sin(self) -> Float32<U> { float_unit(self.val.sin()) }
    fn cos(self) -> Float32<U> { float_unit(self.val.cos()) }
    fn tan(self) -> Float32<U> { float_unit(self.val.tan()) }
    fn asin(self) -> Float32<U> { float_unit(self.val.asin()) }
    fn acos(self) -> Float32<U> { float_unit(self.val.acos()) }
    fn atan(self) -> Float32<U> { float_unit(self.val.atan()) }
    fn atan2(self, other: Float32<U>) -> Float32<U> { float_unit(self.val.atan2(other.val)) }
    fn sin_cos(self) -> (Float32<U>, Float32<U>) { let (a, b) = self.val.sin_cos(); (float_unit(a), float_unit(b)) }
    fn exp_m1(self) -> Float32<U> { float_unit(self.val.exp_m1()) }
    fn ln_1p(self) -> Float32<U> { float_unit(self.val.ln_1p()) }
    fn sinh(self) -> Float32<U> { float_unit(self.val.sinh()) }
    fn cosh(self) -> Float32<U> { float_unit(self.val.cosh()) }
    fn tanh(self) -> Float32<U> { float_unit(self.val.tanh()) }
    fn asinh(self) -> Float32<U> { float_unit(self.val.asinh()) }
    fn acosh(self) -> Float32<U> { float_unit(self.val.acosh()) }
    fn atanh(self) -> Float32<U> { float_unit(self.val.atanh()) }
}