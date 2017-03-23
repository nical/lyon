use core::math::*;
use path::Path;
use buffer::*;
use batch_builder;
//use renderer::{FillVertex, StrokeVertex};
use frame::{FillVertexBufferRange, StrokeVertexBufferRange, IndexBufferRange};

use tessellation::path_fill::FillOptions;
use tessellation::path_stroke::StrokeOptions;

use std::collections::HashMap;
//use std::sync::mpsc::channel;
//use std::thread;

#[derive(Copy, Clone, Debug)]
pub struct Image;
#[derive(Copy, Clone, Debug)]
pub struct Transform;
#[derive(Copy, Clone, Debug)]
pub struct Mesh;
#[derive(Copy, Clone, Debug)]
pub struct Ellipse;
#[derive(Copy, Clone, Debug)]
pub struct Effect;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Epoch(u64);

pub type ImageId = Id<Image>;
pub type TransformId = Id<Transform>;
pub type TransformIdRange = IdRange<Transform>;
pub type RenderNodeId = Id<RenderNode>;
pub type PathId = Id<Path>;
pub type RectId = Id<Rect>;
pub type EllipseId = Id<Ellipse>;
pub type MeshId = Id<Mesh>;
pub type ColorId = Id<Color>;
pub type GradientId = Id<LinearGradient>;
pub type EffectId = Id<Effect>;

pub enum PatternId {
    Color(ColorId),
    Gradient(GradientId)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Color { r: u8, g: u8, b: u8, a: u8 }

impl Color {
    pub fn transparent_black() -> Self { Color { r: 0, g: 0, b: 0, a: 0 } }

    pub fn black() -> Self { Color { r: 0, g: 0, b: 0, a: 255 } }

    pub fn white() -> Self { Color { r: 255, g: 255, b: 255, a: 255 } }

    pub fn array(self) -> [u8; 4] { [self.r, self.g, self.b, self.a] }

    pub fn f32_array(self) -> [f32; 4] {[
        self.r as f32 / 255.0,
        self.g as f32 / 255.0,
        self.b as f32 / 255.0,
        self.a as f32 / 255.0,
    ]}
}

#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub enum ShapeId {
    Path(PathId),
    Ellipse(EllipseId),
    Rect(RectId),
    None, // meh
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GradientStop {
    pub color: Color,
    pub d: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    Color(Color),
    Image(ImagePattern),
    LinearGradient(LinearGradient),
}

impl Pattern {
    pub fn is_opaque(&self) -> bool {
        match self {
            &Pattern::Color(color) => { color.a == 255 }
            &Pattern::LinearGradient(ref gradient) => { gradient.is_opaque }
            &Pattern::Image(ref img) => { img.is_opaque }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    pub stops: Vec<GradientStop>,
    pub is_opaque: bool,
}

impl LinearGradient {
    pub fn new(stops: Vec<GradientStop>) -> Self {
        let mut is_opaque = true;
        for stop in &stops {
            if stop.color.a != 255 {
                is_opaque = false;
                break;
            }
        }
        LinearGradient {
            stops: stops,
            is_opaque: is_opaque,
        }
    }

    pub fn stops(&self) -> &[GradientStop] { &self.stops }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImagePattern {
    pub image_id: ImageId,
    pub rect: Rect,
    pub is_opaque: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub pattern: Pattern,
    pub width: f32,
    pub aa: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FillStyle {
    pub pattern: Pattern,
    pub aa: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderNode {
    pub shape: ShapeId,
    pub transform: Option<TransformId>,
    pub stroke: Option<StrokeStyle>,
    pub fill: Option<FillStyle>,
}

pub struct Api {
    // TODO!
}


pub struct PropertyFlags {
    flags: u32,
}

impl PropertyFlags {
    pub fn default() -> Self { PropertyFlags { flags: 0 } }
    pub fn animated() -> Self { PropertyFlags { flags: 1 } }
}

