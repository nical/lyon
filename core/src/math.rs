use euclid;

use std::cmp;

pub type Vec2 = euclid::Point2D<f32>;
pub type IntVec2 = euclid::Point2D<i32>;
pub type Size = euclid::Size2D<f32>;
pub type IntSize = euclid::Size2D<i32>;
pub type Rect = euclid::Rect<f32>;
pub type IntRect = euclid::Rect<i32>;

pub fn vec2(x: f32, y: f32) -> Vec2 { Vec2::new(x, y) }
pub fn int_vec2(x: i32, y: i32) -> IntVec2 { IntVec2::new(x, y) }
pub fn size(w: f32, h: f32) -> Size { Size::new(w, h) }
pub fn int_size(w: i32, h: i32) -> IntSize { IntSize::new(w, h) }
pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect { Rect::new(vec2(x, y), size(w, h)) }
pub fn int_rect(x: i32, y: i32, w: i32, h: i32) -> IntRect { IntRect::new(int_vec2(x, y), int_size(w, h)) }

pub trait Vec2Tuple<S> { fn tuple(self) -> (S, S); }

impl<S> Vec2Tuple<S> for euclid::Point2D<S> { fn tuple(self) ->(S, S) { (self.x, self.y) } }

pub trait Vec2Array<S> { fn array(self) -> [S; 2]; }

impl<S> Vec2Array<S> for euclid::Point2D<S> { fn array(self) ->[S; 2] { [self.x, self.y] } }

pub trait Vec2Length {
    fn length(self) -> f32;
}

pub trait Vec2SquareLength {
    fn square_length(self) -> f32;
}

impl Vec2Length for Vec2 {
    fn length(self) -> f32 { self.square_length().sqrt() }
}

impl Vec2SquareLength for Vec2 {
    fn square_length(self) -> f32 { self.x*self.x + self.y*self.y }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct IntFraction {
    pub numerator: i64,
    pub denominator: i64,
}

impl IntFraction {
    pub fn from_int(val: i32) -> IntFraction {  IntFraction { numerator: val as i64, denominator: 1 } }

    pub fn to_f32(self) -> f32 { self.numerator as f32 / self.denominator as f32 }

    pub fn int_approximation(self) -> i32 { (self.numerator / self.denominator) as i32 }

    pub fn int_approximation_has_error(self) -> bool { self.numerator % self.denominator != 0 }
}

impl cmp::PartialOrd<IntFraction> for IntFraction {
    fn partial_cmp(&self, rhs: &IntFraction) -> Option<cmp::Ordering> {
        return (self.numerator * rhs.denominator).partial_cmp(&(rhs.numerator * self.denominator));
    }
}

impl cmp::PartialOrd<i32> for IntFraction {
    fn partial_cmp(&self, rhs: &i32) -> Option<cmp::Ordering> {
        return self.numerator.partial_cmp(&(&(*rhs as i64) * self.denominator));
    }
}

impl cmp::PartialEq<i32> for IntFraction {
    fn eq(&self, rhs: &i32) -> bool {
        return self.numerator == *rhs as i64 * self.denominator;
    }
}

#[derive(Copy, Clone, Debug)]
struct IntVec2Fraction {
    numerator: IntVec2,
    denominator: i32,
}

impl IntVec2Fraction {
    pub fn from_int_vec2(val: IntVec2) -> IntVec2Fraction { IntVec2Fraction { numerator: val, denominator: 1 } }

    pub fn to_vec2(self) -> Vec2 {
        let denom = self.denominator as f32;
        return vec2(self.numerator.x as f32 / denom, self.numerator.y as f32 / denom);
    }

    pub fn int_approximation(self) -> IntVec2 { self.numerator / self.denominator }

    pub fn int_approximation_has_error(self) -> bool {
        self.numerator.x % self.denominator != 0 || self.numerator.y % self.denominator != 0
    }
}
