use vodk_math::{ Vec2 };

pub trait Position2D {
    fn position(&self) -> Vec2;
}

impl Position2D for Vec2 { fn position(&self) -> Vec2 { *self } }
