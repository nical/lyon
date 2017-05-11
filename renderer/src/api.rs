use core::math::*;
use path::Path;
use buffer::*;
use gpu_data::{GpuBlock4, GpuBlock8, GpuBlock16};
use gpu_data::{GpuTransform2D, GpuRect};

use std::sync::Arc;

#[derive(Copy, Clone, Debug)]
pub struct Mesh;
#[derive(Copy, Clone, Debug)]
pub struct Ellipse;
#[derive(Copy, Clone, Debug)]
pub struct Effect;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Epoch(u64);

#[derive(Copy, Clone, Debug)]
pub struct ApiImage;
#[derive(Copy, Clone, Debug)]
pub struct ApiScene;
#[derive(Copy, Clone, Debug)]
pub struct ApiNode;
#[derive(Copy, Clone, Debug)]
pub struct ApiRenderTarget;
#[derive(Copy, Clone, Debug)]
pub struct ApiRenderSurface;

pub type Transform2dBlock = GpuBlock8;
pub type Transform3dBlock = GpuBlock16;
pub type RectBlock = GpuBlock4;

pub type Transform = GpuTransform2D;

pub type ImageId = Id<ApiImage>;
pub type NodeId = Id<ApiNode>;
pub type SceneId = Id<ApiScene>;
pub type TransformId = Id<Transform2dBlock>;
pub type TransformIdRange = IdRange<Transform2dBlock>;
pub type PathId = Id<Path>;
pub type EllipseId = Id<Ellipse>;
pub type MeshId = Id<Mesh>;
pub type ColorId = Id<Color>;
pub type ColorIdRange = IdRange<Color>;
pub type GradientId = Id<LinearGradient>;
pub type EffectId = Id<Effect>;
pub type NumberId = Id<f32>;
pub type NumberIdRange = IdRange<f32>;
pub type PointId = Id<Point>;
pub type PointIdRange = IdRange<Point>;
pub type RectId = Id<RectBlock>;
pub type RectIdRange = IdRange<RectBlock>;
pub type RenderTargetId = Id<ApiRenderTarget>;
pub type RenderSurfaceId = Id<ApiRenderSurface>;

pub type TransformProperty = Property<Transform2dBlock>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PatternId {
    Color(ColorId),
    // TODO: Image, Gradient, etc.
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    #[inline]
    pub fn white() -> Self {
        Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }

    #[inline]
    pub fn black() -> Self {
        Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    #[inline]
    pub fn red() -> Self {
        Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    #[inline]
    pub fn green() -> Self {
        Color {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        }
    }

    #[inline]
    pub fn blue() -> Self {
        Color {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        }
    }

    #[inline]
    pub fn with_alpha(mut self, alpha: u8) -> Self {
        self.a = alpha;
        return self;
    }

    #[inline]
    pub fn transparent_black() -> Self { Color::black().with_alpha(0) }

    #[inline]
    pub fn is_opaque(&self) -> bool { self.a == 255 }

    #[inline]
    pub fn array(self) -> [u8; 4] { [self.r, self.g, self.b, self.a] }

    #[inline]
    pub fn f32_array(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShapeId {
    Path(PathId),
    Ellipse(EllipseId),
    Rect(RectId),
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
            &Pattern::Color(color) => color.is_opaque(),
            &Pattern::LinearGradient(ref gradient) => gradient.is_opaque,
            &Pattern::Image(ref img) => img.is_opaque,
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

pub enum Usage {
    Static,
    Dynamic,
}

pub trait Api {
    fn add_path(&mut self, path: PathDescriptor) -> ShapeId;

    fn add_colors(&mut self, values: &[Color], usage: Usage) -> ColorIdRange;

    fn add_transforms(&mut self, values: &[Transform], usage: Usage) -> TransformIdRange;

    fn add_numbers(&mut self, values: &[f32], usage: Usage) -> NumberIdRange;

    fn add_points(&mut self, values: &[Point], usage: Usage) -> PointIdRange;

    fn add_rects(&mut self, values: &[GpuRect], usage: Usage) -> RectIdRange;

    fn add_gradient_stops(&mut self, gradient: &[GradientStop], usage: Usage) -> GradientId;

    fn add_scene(&mut self, descriptor: &SceneDescriptor) -> SceneId;

    fn add_render_target(
        &mut self,
        descriptor: &RenderTargetDescriptor,
        scene: SceneId,
    ) -> RenderTargetId;


    fn set_colors(&mut self, range: ColorIdRange, values: &[Color]);

    fn set_transforms(&mut self, range: TransformIdRange, values: &[Transform]);

    fn set_numbers(&mut self, range: NumberIdRange, values: &[f32]);

    fn set_points(&mut self, range: PointIdRange, values: &[Point]);

    fn set_rects(&mut self, range: RectIdRange, values: &[GpuRect]);

    fn set_gradient_stops(&mut self, id: GradientId, gradient: &[GradientStop]);


    fn remove_shape(&mut self, shape: ShapeId);

    fn remove_scene(&mut self, scene: SceneId);


    fn render(&mut self, device: &mut Device) -> Result<(), ()>;
}

pub trait Device {}

#[derive(Clone, Debug, PartialEq)]
pub enum Property<T> {
    //Value(T),
    Global(Id<T>),
    SceneParam(u16),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SceneParam {
    Number(NumberId),
    NumberRange(NumberIdRange),
    Color(ColorId),
    ColorRange(ColorIdRange),
    Transform(TransformId),
    TransformRange(TransformIdRange),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SceneParamType {
    Number,
    NumberRange,
    Color,
    ColorRange,
    Transform,
    TransformRange,
}

impl SceneParam {
    pub fn get_type(&self) -> SceneParamType {
        match *self {
            SceneParam::Number(_) => SceneParamType::Number,
            SceneParam::NumberRange(_) => SceneParamType::NumberRange,
            SceneParam::Color(_) => SceneParamType::Color,
            SceneParam::ColorRange(_) => SceneParamType::ColorRange,
            SceneParam::Transform(_) => SceneParamType::Transform,
            SceneParam::TransformRange(_) => SceneParamType::TransformRange,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SceneDescriptor {
    pub items: Vec<RenderItem>,
    pub params: Vec<SceneParamType>,
}

#[derive(Clone, Debug)]
pub struct PathDescriptor {
    pub path: Arc<Path>,
    pub tolerance: f32,
    pub flags: u32,
}

#[derive(Clone, Debug)]
pub enum RenderItem {
    Paint(PaintOp),
    Scene(SceneInstance),
}

#[derive(Clone, Debug)]
pub struct SceneInstance {
    pub id: SceneId,
    pub params: Vec<SceneParam>,
    pub flags: u32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PaintType {
    Fill,
    Stroke,
}

#[derive(Clone, Debug)]
pub struct PaintOp {
    pub ty: PaintType,
    pub shape: ShapeId,
    pub pattern: PatternId,
    pub local_transform: TransformProperty,
    pub view_transform: TransformProperty,
    pub width: Property<f32>,
    pub flags: u32,
}

impl PaintOp {
    pub fn fill(shape: ShapeId, pattern: PatternId) -> Self {
        PaintOp {
            ty: PaintType::Fill,
            shape: shape,
            pattern: pattern,
            local_transform: Property::Global(Id::new(0)),
            view_transform: Property::Global(Id::new(0)),
            width: Property::Global(Id::new(0)),
            flags: 0,
        }
    }

    pub fn stroke(shape: ShapeId, pattern: PatternId) -> Self {
        PaintOp {
            ty: PaintType::Stroke,
            shape: shape,
            pattern: pattern,
            local_transform: Property::Global(Id::new(0)),
            view_transform: Property::Global(Id::new(0)),
            width: Property::Global(Id::new(0)),
            flags: 0,
        }
    }

    pub fn with_local_transform(mut self, transform: TransformProperty) -> Self {
        self.local_transform = transform;
        return self;
    }

    pub fn with_view_transform(mut self, transform: TransformProperty) -> Self {
        self.view_transform = transform;
        return self;
    }

    pub fn with_width(mut self, width: Property<f32>) -> Self {
        self.width = width;
        return self;
    }

    pub fn with_flags(mut self, flags: u32) -> Self {
        self.flags |= flags;
        return self;
    }
}

impl SceneDescriptor {
    pub fn paint(&mut self, op: PaintOp) { self.items.push(RenderItem::Paint(op)); }

    pub fn paint_scene(&mut self, op: SceneInstance) { self.items.push(RenderItem::Scene(op)) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SurfaceFormat {
    RgbaU8,
    RgbaF32,
    AlphaU8,
    AlphaF32,
    //Depth,
    //Stencil,
}

#[derive(Clone, Debug)]
pub struct RenderTargetDescriptor {
    pub format: SurfaceFormat,
    pub width: u16,
    pub height: u16,
    pub standalone: bool,
}
