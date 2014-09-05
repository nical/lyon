use math::units::world;

#[deriving(Clone, Show)]
pub struct Circle {
    pub center: world::Vec2,
    pub radius: f32,
}

#[deriving(Clone, Show)]
pub struct Ellipsis {
    pub center: world::Vec2,
    pub radius: world::Vec2,
}

#[deriving(Clone, Show)]
pub struct RoundedRectangle {
    pub rectangle: world::Rectangle,
    pub top_left_radius: f32,
    pub top_right_radius: f32,
    pub bottom_right_radius: f32,
    pub bottom_left_radius: f32,
}

