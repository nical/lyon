use std::ops;
use vodkmath::vector::ScalarMul;

#[derive(Copy, Clone, Debug)]
pub struct Rgba<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

#[allow(dead_code)]
impl<T: Copy+ops::Add<T, Output=T>> ops::Add<Rgba<T>> for Rgba<T> {

    type Output = Rgba<T>;

    #[inline]
    fn add(self, rhs: Rgba<T>) -> Rgba<T> {
        return Rgba {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        };
    }
}

#[allow(dead_code)]
impl<T: Copy+ops::Sub<T, Output=T>> ops::Sub<Rgba<T>> for Rgba<T> {

    type Output = Rgba<T>;

    #[inline]
    fn sub(self, rhs: Rgba<T>) -> Rgba<T> {
        return Rgba {
            r: self.r - rhs.r,
            g: self.g - rhs.g,
            b: self.b - rhs.b,
            a: self.a - rhs.a,
        };
    }
}

#[allow(dead_code)]
impl<T: Copy+ops::Mul<T, Output=T>> ScalarMul<T> for Rgba<T> {

    #[inline]
    fn scalar_mul(&self, rhs: T) -> Rgba<T> {
        return Rgba {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a * rhs,
        };
    }

    #[inline]
    fn scalar_mul_in_place(&mut self, rhs: T) {
        self.r = self.r * rhs;
        self.g = self.g * rhs;
        self.b = self.b * rhs;
        self.a = self.a * rhs;
    }
}
