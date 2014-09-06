
use math::vector::Vector2D;
use math::units::world;
use math::units::texels;
use gfx2d::color::Rgba;
use gfx2d::shapes;
use data;

pub struct BezierSegment<T> {
    pub p0: Vector2D<T>,
    pub p1: Vector2D<T>,
    pub p2: Vector2D<T>,
    pub p3: Vector2D<T>
}

impl<T> BezierSegment<T> {
    pub fn linearize(&self, output: &mut[Vector2D<T>]) {
        let step = 1.0 / (output.len() - 1) as f32;
        for i in range(0, output.len()) {
            output[i] = self.point_at(i as f32 * step);
        }
    }

    pub fn point_at(&self, t: f32) -> Vector2D<T> {
        let t2 = t*t;
        let t3 = t2*t;
        let one_t = 1.0 - t;
        let one_t2 = one_t*one_t;
        let one_t3 = one_t2*one_t;
        return self.p0.times(one_t3)
             + self.p1.times(3.0*one_t2*t)
             + self.p2.times(3.0*one_t*t2)
             + self.p3.times(t3);
    }
}
