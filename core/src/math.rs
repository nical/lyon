use euclid;

pub use euclid::{Point2D, Vector2D, TypedPoint2D, Radians};

pub type Point = euclid::Point2D<f32>;
pub type IntPoint = euclid::Point2D<i32>;
pub type F64Point = euclid::Point2D<f64>;
pub type Vector = euclid::Vector2D<f32>;
pub type IntVector = euclid::Vector2D<i32>;
pub type Size = euclid::Size2D<f32>;
pub type IntSize = euclid::Size2D<i32>;
pub type Rect = euclid::Rect<f32>;
pub type IntRect = euclid::Rect<i32>;
pub type Transform2D = euclid::Transform2D<f32>;
pub type Transform3D = euclid::Transform3D<f32>;

pub use euclid::vec2 as vector;
pub use euclid::vec3 as vector3d;
pub use euclid::point2 as point;
pub use euclid::point3 as point3d;
pub use euclid::size2 as size;
pub use euclid::rect;
