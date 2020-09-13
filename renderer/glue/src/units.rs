use crate::geom::euclid;

pub use crate::geom::euclid::{
    point2, vec2, point3, vec3, rect, Box2D, Point2D, Vector2D,
};

pub struct DevicePixels;
pub type DeviceIntSize = euclid::Size2D<i32, DevicePixels>;
pub type DeviceIntPoint = euclid::Point2D<i32, DevicePixels>;
pub type DeviceIntVector = euclid::Vector2D<i32, DevicePixels>;
pub type DeviceIntBox = euclid::Box2D<i32, DevicePixels>;
pub type DeviceSize = euclid::Size2D<f32, DevicePixels>;
pub type DevicePoint = euclid::Point2D<f32, DevicePixels>;
pub type DeviceVector = euclid::Vector2D<f32, DevicePixels>;
pub type DeviceBox = euclid::Box2D<f32, DevicePixels>;

pub struct LocalUnit;
pub type LocalSize = euclid::Size2D<f32, LocalUnit>;
pub type LocalPoint = euclid::Point2D<f32, LocalUnit>;
pub type LocalVector = euclid::Vector2D<f32, LocalUnit>;
pub type LocalBox = euclid::Box2D<f32, LocalUnit>;

pub struct LayerUnit;
pub type LayerSize = euclid::Size2D<f32, LayerUnit>;
pub type LayerPoint = euclid::Point2D<f32, LayerUnit>;
pub type LayerVector = euclid::Vector2D<f32, LayerUnit>;
pub type LayerBox = euclid::Box2D<f32, LayerUnit>;

pub struct WorldUnit;
pub type WorldSize = euclid::Size2D<f32, WorldUnit>;
pub type WorldPoint = euclid::Point2D<f32, WorldUnit>;
pub type WorldVector = euclid::Vector2D<f32, WorldUnit>;
pub type WorldBox = euclid::Box2D<f32, WorldUnit>;

