use vodkmath::units::world;

#[derive(Clone, Debug)]
pub struct Circle {
    pub center: world::Vec2,
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct Ellipsis {
    pub center: world::Vec2,
    pub radius: world::Vec2,
}

#[derive(Clone, Debug)]
pub struct RoundedRectangle {
    pub rectangle: world::Rectangle,
    pub top_left_radius: f32,
    pub top_right_radius: f32,
    pub bottom_right_radius: f32,
    pub bottom_left_radius: f32,
}

