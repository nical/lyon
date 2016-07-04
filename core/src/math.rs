use euclid;

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
    fn square_length(self) -> f32;
}

impl Vec2Length for Vec2 {
    fn length(self) -> f32 { self.square_length().sqrt() }
    fn square_length(self) -> f32 { self.x*self.x + self.y*self.y }
}

/*
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h:f32) -> Rect {
        let mut rect = Rect { x: x, y: y, width: w, height: h };
        rect.ensure_invariant();
        return rect;
    }

    pub fn origin(&self) -> Vec2 { vec2(self.x, self.y) }

    pub fn size(&self) -> Size2<f32> { Size2::new(self.width, self.height) }

    pub fn move_by(&mut self, v: Vec2) {
        self.x = self.x + v.x;
        self.y = self.y + v.y;
    }

    pub fn scale_by(&mut self, v: f32) {
        self.x = self.x * v;
        self.y = self.y * v;
        self.width = self.width * v;
        self.height = self.height * v;
        self.ensure_invariant();
    }

    pub fn top_left(&self) -> Vec2 { vec2(self.x, self.y) }

    pub fn top_right(&self) -> Vec2 { vec2(self.x + self.width, self.y) }

    pub fn bottom_right(&self) -> Vec2 { vec2(self.x + self.width, self.y + self.height) }

    pub fn bottom_left(&self) -> Vec2 { vec2(self.x, self.y + self.height) }

    pub fn x_most(&self) -> f32 { self.x + self.width }

    pub fn y_most(&self) -> f32 { self.y + self.height }

    pub fn contains(&self, other: &Rect) -> bool {
        return self.x <= other.x &&
               self.y <= self.y &&
               self.x_most() >= other.x_most() &&
               self.y_most() >= other.y_most();
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        return self.x < other.x_most() && other.x < self.x_most() &&
            self.y < other.y_most() && other.y < self.y_most();
    }

    pub fn inflate(&mut self, d: f32) {
        self.x -= d;
        self.y -= d;
        self.width += 2.0*d;
        self.height += 2.0*d;
    }

    pub fn deflate(&mut self, d: f32) { self.inflate(-d); }

    pub fn ensure_invariant(&mut self) {
        self.x = self.x.min(self.x + self.width);
        self.y = self.y.min(self.y + self.height);
        self.width = self.width.abs();
        self.height = self.height.abs();
    }
}
*/