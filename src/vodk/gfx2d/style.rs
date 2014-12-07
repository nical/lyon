use math::units::texels;
use color::Rgba;

pub enum FillStyle<'l> {
    Texture(&'l texels::Mat3),
    Color(&'l Rgba<u8>),
    None,
}

pub enum StrokeStyle<'l> {
    Texture(&'l texels::Mat3),
    Color(&'l Rgba<u8>),
    None,
}

pub type StrokeFlags = u16;
pub static STROKE_DEFAULT : StrokeFlags = 0;
pub static STROKE_INWARD  : StrokeFlags = 1 << 0;
pub static STROKE_OUTWARD : StrokeFlags = 1 << 1;
pub static STROKE_CLOSED  : StrokeFlags = 1 << 2;
