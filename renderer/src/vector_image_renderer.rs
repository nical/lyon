use std::sync::Arc;
use std::mem;
use std::rc::Rc;
use std::collections::HashMap;

use gfx_renderer::{GpuFillVertex, GpuStrokeVertex};
use tessellation as tess;
use tessellation::geometry_builder::{VertexBuffers, BuffersBuilder};
use core::math::*;
use path::Path;
use gpu_data::*;

pub struct Context {
    next_vector_image_id: u32,
    next_geometry_id: u32,
    device: Box<Device>,
}

impl Context {
    pub fn new(device: Box<Device>) -> Self {
        Context {
            next_vector_image_id: 0,
            next_geometry_id: 0,
            device,
        }
    }

    pub fn new_vector_image(&mut self) -> VectorImageBuilder {
        let id = self.next_vector_image_id;
        self.next_vector_image_id += 1;
        VectorImageBuilder {
            gpu_data: GpuMemory::new(),
            opaque_fill_cmds: Vec::new(),
            opaque_stroke_cmds: Vec::new(),
            z_index: 0,
            id: VectorImageId(id),
        }
    }    

    pub fn new_geometry(&mut self) -> GeometryBuilder {
        let id = self.next_geometry_id;
        self.next_geometry_id += 1;
        GeometryBuilder {
            fill: VertexBuffers::new(),
            stroke: VertexBuffers::new(),
            id: GeometryId(id),
        }
    }

    pub fn new_layer(&mut self) -> LayerBuilder {
        LayerBuilder {
            vector_images: HashMap::new(),
            z_index: 0,
        }
    }

    pub fn submit_geometry(&mut self, geom: GeometryBuilder) {
        self.device.submit_geometry(geom);
    }
}

pub struct GeometryBuilder {
    fill: VertexBuffers<GpuFillVertex>,
    stroke: VertexBuffers<GpuStrokeVertex>,
    id: GeometryId,
}

#[derive(Clone)]
pub struct GpuMemory {
    buffer: Vec<GpuWord>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColorId(GpuAddress);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TransformId(GpuAddress);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageId(u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NumberId(u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EffectId(u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GeometryId(u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GpuAddress(i32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GpuAddressRange {
    start: i32,
    end: i32,
}

impl GpuAddressRange {
    pub fn start(&self) -> GpuAddress { GpuAddress(self.start) }

    pub fn is_empty(&self) -> bool { self.start == self.end }

    pub fn shrink_left(&mut self, amount: u32) {
        assert!(self.end.abs() - self.start.abs() >= amount as i32);
        let sign = self.start.signum()  ;
        self.start = sign * (self.start.abs() + amount as i32);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VectorImageId(u32);

impl GpuAddress {
    // TODO: it would be better to use an offset plus the id of
    // the vector image to prevent from using a local transform
    // of an image on another image.
    pub fn is_local(&self) -> bool { self.0 >= 0 }
    pub fn is_global(&self) -> bool { self.0 < 0 }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FillId(GpuAddress);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StrokeId(GpuAddress);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FillType {
    Opaque,
    Transparent,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Pattern {
    Color(ColorId),
    Image(Rect, ImageId),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FillStyle {
    pattern: Pattern,
    // TODO: add effects
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pattern: Pattern,
    // TODO: add effects
}

pub enum Shape {
    Path { path: Arc<Path>, tolerance: f32 },
    Circle { center: Point, radius: f32, tolerance: f32 },
}

struct OpaqueFillCmd {
    shape: Shape,
    prim_address: GpuAddress,
}

struct OpaqueStrokeCmd {
    shape: Shape,
    prim_address: GpuAddress,
    options: tess::StrokeOptions,
}

pub struct VectorImageBuilder {
    opaque_fill_cmds: Vec<OpaqueFillCmd>,
    opaque_stroke_cmds: Vec<OpaqueStrokeCmd>,
    gpu_data: GpuMemory,
    z_index: u32,
    id: VectorImageId,
}

impl VectorImageBuilder {
    pub fn fill(
        &mut self,
        shape: Shape,
        style: FillStyle,
        transforms: [TransformId; 2],
    ) -> FillId {
        let fill_type = FillType::Opaque;
        let z = self.z_index;

        self.z_index += 1;

        let address = self.add_fill_primitve(
            FillPrimitive::new(
                z,
                match style.pattern {
                    Pattern::Color(color) => color,
                    _ => unimplemented!()
                },
                transforms,
            )
        );

        match fill_type {
            FillType::Opaque => {
                self.opaque_fill_cmds.push(OpaqueFillCmd {
                    shape: shape,
                    prim_address: address.0,
                });
            }
            _ => {
                unimplemented!()
            }
        };

        return address;
    }

    pub fn stroke(
        &mut self,
        shape: Shape,
        style: StrokeStyle,
        options: &tess::StrokeOptions,
        transforms: [TransformId; 2],
    ) -> StrokeId {
        let z = self.z_index;
        self.z_index += 1;

        let address = self.add_stroke_primitve(
            StrokePrimitive::new(
                z,
                options.line_width,
                match style.pattern {
                    Pattern::Color(color) => color,
                    _ => unimplemented!()
                },
                transforms,
            )
        );

        self.opaque_stroke_cmds.push(OpaqueStrokeCmd {
            shape: shape,
            prim_address: address.0,
            options: options.clone(),
        });

        return address;
    }

    pub fn build(mut self, geom: &mut GeometryBuilder) -> VectorImageInstance {
        let mut fill_tess = tess::FillTessellator::new();
        let mut stroke_tess = tess::StrokeTessellator::new();

        let cmds = mem::replace(&mut self.opaque_fill_cmds, Vec::new());
        let contains_fill_ops = cmds.len() > 0;
        for cmd in cmds.into_iter().rev() {
            match cmd.shape {
                Shape::Path { path, tolerance } => {
                    fill_tess.tessellate_path(
                        path.path_iter(),
                        &tess::FillOptions::tolerance(tolerance),
                        &mut BuffersBuilder::new(
                            &mut geom.fill,
                            VertexCtor(cmd.prim_address)
                        )
                    ).unwrap();
                }
                _ => {
                    unimplemented!()
                }
            }
        }

        let cmds = mem::replace(&mut self.opaque_stroke_cmds, Vec::new());
        let contains_stroke_ops = cmds.len() > 0;
        for cmd in cmds.into_iter().rev() {
            match cmd.shape {
                Shape::Path { path, tolerance } => {
                    stroke_tess.tessellate_path(
                        path.path_iter(),
                        &cmd.options,
                        &mut BuffersBuilder::new(
                            &mut geom.stroke,
                            VertexCtor(cmd.prim_address)
                        )
                    );
                }
                _ => {
                    unimplemented!()
                }
            }
        }

        VectorImageInstance {
            base: Arc::new(VectorImage {
                descriptor: VectorImageDescriptor {
                    geometry: geom.id,
                    id: self.id,
                    z_range: self.z_index,
                    mem_per_instance: self.gpu_data.len() as u32,
                    contains_fill_ops,
                    contains_stroke_ops,
                }
            }),
            gpu_data: self.gpu_data,
        }
    }

    fn add_fill_primitve(&mut self, prim: FillPrimitive) -> FillId {
        let address = self.gpu_data.push(&prim);
        return FillId(address);
    }

    fn add_stroke_primitve(&mut self, prim: StrokePrimitive) -> StrokeId {
        let address = self.gpu_data.push(&prim);
        return StrokeId(address);
    }

    pub fn add_transform(&mut self, transform: &GpuTransform2D) -> TransformId {
        let address = self.gpu_data.push(transform);
        return TransformId(address);
    }

    pub fn set_transform(&mut self, id: TransformId, transform: &GpuTransform2D) {
        self.gpu_data.set(id.0, transform);
    }

    pub fn add_color(&mut self, color: &GpuColorF) -> ColorId {
        let address = self.gpu_data.push(color);
        return ColorId(address);
    }

    pub fn set_color(&mut self, id: ColorId, color: &GpuColorF) {
        self.gpu_data.set(id.0, color);
    }
}

pub struct VectorImage {
    descriptor: VectorImageDescriptor,
    // TODO: keep track of which portion of the buffer the image uses.
}

#[derive(Copy, Clone, Debug)]
struct VectorImageDescriptor {
    geometry: GeometryId,
    id: VectorImageId,
    z_range: u32,
    mem_per_instance: u32,
    contains_fill_ops: bool,
    contains_stroke_ops: bool,
}

impl VectorImage {
    pub fn id(&self) -> VectorImageId { self.descriptor.id }

    pub fn z_range(&self) -> u32 { self.descriptor.z_range }

    pub fn geometry(&self) -> GeometryId { self.descriptor.geometry }
}

pub struct VectorImageInstance {
    base: Arc<VectorImage>,
    gpu_data: GpuMemory,
}

impl VectorImageInstance {
    pub fn clone_instance(&self) -> Self {
        VectorImageInstance {
            base: Arc::clone(&self.base),
            gpu_data: self.gpu_data.clone(),
        }
    }

    pub fn base(&self) -> &VectorImage {
        &*self.base
    }
}

pub struct VertexCtor(GpuAddress);

impl tess::VertexConstructor<tess::FillVertex, GpuFillVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tess::FillVertex) -> GpuFillVertex {
        debug_assert!(!vertex.position.x.is_nan());
        debug_assert!(!vertex.position.y.is_nan());
        debug_assert!(!vertex.normal.x.is_nan());
        debug_assert!(!vertex.normal.y.is_nan());
        GpuFillVertex {
            position: vertex.position.to_array(),
            normal: vertex.normal.to_array(),
            prim_id: (self.0).0 as i32,
        }
    }
}

impl tess::VertexConstructor<tess::StrokeVertex, GpuStrokeVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tess::StrokeVertex) -> GpuStrokeVertex {
        debug_assert!(!vertex.position.x.is_nan());
        debug_assert!(!vertex.position.y.is_nan());
        debug_assert!(!vertex.normal.x.is_nan());
        debug_assert!(!vertex.normal.y.is_nan());
        assert!(!vertex.advancement.is_nan());
        GpuStrokeVertex {
            position: vertex.position.to_array(),
            normal: vertex.normal.to_array(),
            advancement: vertex.advancement,
            prim_id: (self.0).0,
        }
    }
}

impl GpuMemory {
    pub fn new() -> Self {
        GpuMemory {
            buffer: Vec::with_capacity(128),
        }
    }

    pub fn len(&self) -> usize { self.buffer.len() }

    pub fn as_slice(&self) -> &[GpuWord] { &self.buffer[..] }

    fn push<Block: GpuBlock>(&mut self, block: &Block) -> GpuAddress {
        debug_assert_eq!(block.slice().len() % 4, 0);
        let address = GpuAddress(self.buffer.len() as i32);
        self.buffer.extend(block.slice().iter().cloned());
        return address;
    }

    fn set<Block: GpuBlock>(&mut self, address: GpuAddress, block: &Block) {
        let base = address.0  as usize;
        for (offset, element) in block.slice().iter().cloned().enumerate() {
            self.buffer[base + offset] = element;
        }
    }
}

pub struct DrawCmd {
    pub geometry: GeometryId,
    pub num_instances: u32,
    pub base_address: GpuAddress,
}

struct LayerVectorImage {
    descriptor: VectorImageDescriptor,
    instances: Vec<RenderedInstance>,
    allocated_range: Option<GpuAddressRange>,
}

struct RenderedInstance {
    instance: Rc<VectorImageInstance>,
    z_index: u32,
}

pub struct LayerBuilder {
    vector_images: HashMap<VectorImageId, LayerVectorImage>,
    // TODO: replace this with an external stacking context to allow
    // interleaving layers.
    z_index: u32,
}

impl LayerBuilder {
    pub fn add(&mut self, instance: Rc<VectorImageInstance>) {
        let img = instance.base().descriptor;
        self.vector_images.entry(img.id).or_insert(
            LayerVectorImage {
                instances: Vec::new(),
                descriptor: img,
                allocated_range: None,
            }
        ).instances.push(RenderedInstance {
            instance: instance,
            z_index: self.z_index,
        });

        self.z_index += img.z_range;
    }

    pub fn build(mut self, ctx: &mut Context) -> Layer {
        let mut fill_pass = Vec::new();
        let mut stroke_pass = Vec::new();

        // for each vector image
        for (_, item) in &mut self.vector_images {
            item.instances.sort_by(|a, b| {
                a.z_index.cmp(&b.z_index)
            });

            let num_instances = item.instances.len() as u32;
            let range = ctx.device.allocate_gpu_data(item.descriptor.mem_per_instance * num_instances);
            item.allocated_range = Some(range);

            // for each instance within a vector image
            let mut range_iter = range;
            for img_instance in &item.instances {
                ctx.device.set_gpu_data(range, &img_instance.instance.gpu_data);
                range_iter.shrink_left(item.descriptor.mem_per_instance);
            }

            let base_address = range.start();

            if item.descriptor.contains_fill_ops {
                fill_pass.push(DrawCmd {
                    geometry: item.descriptor.geometry,
                    num_instances,
                    base_address,
                });
            }

            if item.descriptor.contains_stroke_ops {
                stroke_pass.push(DrawCmd {
                    geometry: item.descriptor.geometry,
                    num_instances,
                    base_address,
                });
            }
        }

        Layer {
            fill_pass,
            stroke_pass,
        }
    }
}

pub struct Layer {
    // TODO: support arbitrary number of passes and effects.
    fill_pass: Vec<DrawCmd>,
    stroke_pass: Vec<DrawCmd>,
}

impl Layer {
    pub fn render_opaque_fills(&self, ctx: &mut Context) {
        ctx.device.render_pass(
            &self.fill_pass[..],
            &RenderPassOptions {
                vertex_type: VertexType::Fill,
                enable_blending: false,
                enable_depth_write: true,
                enable_depth_test: true,
                effect: EffectId(0), 
            }
        );
    }

    pub fn render_opaque_strokes(&self, ctx: &mut Context) {
        ctx.device.render_pass(
            &self.stroke_pass[..],
            &RenderPassOptions {
                vertex_type: VertexType::Stroke,
                enable_blending: false,
                enable_depth_write: true,
                enable_depth_test: true,
                effect: EffectId(0), 
            }
        );
    }

    pub fn render_all<'l, Iter>(layer_iter: Iter, ctx: &'l mut Context)
    where
        Iter: Iterator<Item=&'l Layer> + Clone
    {
        for layer in layer_iter.clone() {
            layer.render_opaque_fills(ctx);
        }
        for layer in layer_iter.clone() {
            layer.render_opaque_strokes(ctx);
        }
    }
}

pub struct RenderPassOptions {
    pub vertex_type: VertexType,
    pub enable_blending: bool,
    pub enable_depth_write: bool,
    pub enable_depth_test: bool,
    pub effect: EffectId,
}

pub enum VertexType {
    Fill,
    Stroke,
}

pub trait Device {
    fn allocate_gpu_data(&mut self, _size: u32) -> GpuAddressRange;
    fn set_gpu_data(&mut self, _range: GpuAddressRange, _data: &GpuMemory);
    fn submit_geometry(&mut self, geom: GeometryBuilder);
    fn render_pass(&mut self, cmds: &[DrawCmd], options: &RenderPassOptions);
}

//use gfx::Factory;
use gfx::traits::FactoryExt;
use gfx_renderer::{GlDevice, GlFactory, GpuGeometry, GlDataTexture};
//use gfx::traits::Device as GfxDevice;

pub struct GfxDevice {
    device: GlDevice,
    factory: GlFactory,

    alloc: u32,

    fill_geom: HashMap<GeometryId, GpuGeometry<GpuFillVertex>>,
    stroke_geom: HashMap<GeometryId, GpuGeometry<GpuStrokeVertex>>,
    data_texture: GlDataTexture,
}

impl Device for GfxDevice {
    fn allocate_gpu_data(&mut self, size: u32) -> GpuAddressRange {
        let start = self.alloc as i32;
        self.alloc += size;
        let end = self.alloc as i32;
        GpuAddressRange { start, end }
    }

    fn set_gpu_data(&mut self, range: GpuAddressRange, _data: &GpuMemory) {
        assert!(self.alloc >= range.end as u32);
    }

    fn submit_geometry(&mut self, geom: GeometryBuilder) {
        let (fill_vbo, fill_range) = self.factory.create_vertex_buffer_with_slice(
            &geom.fill.vertices[..],
            &geom.fill.indices[..]
        );
        self.fill_geom.insert(geom.id, GpuGeometry {
            vbo: fill_vbo,
            ibo: fill_range,
        });

        let (stroke_vbo, stroke_range) = self.factory.create_vertex_buffer_with_slice(
            &geom.stroke.vertices[..],
            &geom.stroke.indices[..]
        );
        self.stroke_geom.insert(geom.id, GpuGeometry {
            vbo: stroke_vbo,
            ibo: stroke_range,
        });
    }

    fn render_pass(&mut self, _cmds: &[DrawCmd], _options: &RenderPassOptions) {

    }
}

#[test]
fn simple_vector_image() {
    use path_builder::*;

    struct DummyDevice(u32);
    impl Device for DummyDevice {
        fn allocate_gpu_data(&mut self, size: u32) -> GpuAddressRange {
            let start = self.0 as i32;
            self.0 += size;
            let end = self.0 as i32;
            GpuAddressRange { start, end }
        }
        fn set_gpu_data(&mut self, range: GpuAddressRange, _data: &GpuMemory) {
            assert!(self.0 >= range.end as u32);
        }
        fn submit_geometry(&mut self, _geom: GeometryBuilder) {}
        fn render_pass(&mut self, _cmds: &[DrawCmd], _options: &RenderPassOptions) {}
    }

    let mut path = Path::builder();
    path.move_to(point(1.0, 1.0));
    path.line_to(point(2.0, 1.0));
    path.line_to(point(2.0, 2.0));
    path.line_to(point(1.0, 2.0));
    path.close();
    let path = Arc::new(path.build());

    let mut ctx = Context::new(Box::new(DummyDevice(0)));

    let mut builder = ctx.new_vector_image();
    let mut geom = ctx.new_geometry();

    let transform = builder.add_transform(&GpuTransform2D::new(
        Transform2D::identity()
    ));

    let color = builder.add_color(
        &GpuColorF { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }
    );

    builder.fill(
        Shape::Path { path: Arc::clone(&path), tolerance: 0.01 },
        FillStyle {
            pattern: Pattern::Color(color),
        },
        [transform, transform],
    );

    let img0 = Rc::new(builder.build(&mut geom));
    let img1 = Rc::new(img0.clone_instance());

    let prim_id = GpuAddress(geom.fill.vertices[0].prim_id);

    let mut layer = ctx.new_layer();
    layer.add(Rc::clone(&img0));
    layer.add(Rc::clone(&img1));

    ctx.submit_geometry(geom);
    let layer = layer.build(&mut ctx);

    {
        // render loop
        Layer::render_all([&layer].iter().cloned(), &mut ctx);
    }

    unsafe {
        let prim_offset = prim_id.0 as usize;
        assert_eq!(
            &img0.gpu_data.as_slice()[prim_offset..(prim_offset as usize + 4)],
            &[
                mem::transmute(0u32),
                mem::transmute(color),
                mem::transmute(transform),
                mem::transmute(transform),
            ]
        );
    }
}

