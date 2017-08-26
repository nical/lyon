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
            opaque_fill_cmds: Vec::new(),
            opaque_stroke_cmds: Vec::new(),
            z_index: 0,
            id: VectorImageId(id),
            shared_data: GpuData::new(),
            shared_data_layout: MemoryLayout::new(),
            instance_data: GpuData::new(),
            instance_data_layout: MemoryLayout::new(),
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

impl GeometryBuilder {
    pub fn id(&self) -> GeometryId { self.id }
    pub fn fill(&self) -> &VertexBuffers<GpuFillVertex> { &self.fill }
    pub fn stroke(&self) -> &VertexBuffers<GpuStrokeVertex> { &self.stroke }
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
pub struct VectorImageId(u32);

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
    z_index: u32,
    id: VectorImageId,
    shared_data: GpuData,
    shared_data_layout: MemoryLayout,
    instance_data: GpuData,
    instance_data_layout: MemoryLayout,
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

    pub fn shared_data_mut(&mut self) -> &mut MemoryLayout { &mut self.shared_data_layout }

    pub fn build(mut self, geom: &mut GeometryBuilder) -> VectorImageTemplate {
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
                Shape::Path { path, .. } => {
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

        VectorImageTemplate {
            info: Arc::new(VectorImageInfo {
                geometry: geom.id,
                id: self.id,
                z_range: self.z_index,
                mem_per_instance: self.shared_data.len() as u32,
                contains_fill_ops,
                contains_stroke_ops,
                shared_data: self.shared_data,
                shared_data_layout: self.shared_data_layout,
                instance_data: self.instance_data.clone(),
                instance_data_layout: self.instance_data_layout,
            }),
        }
    }

    fn add_fill_primitve(&mut self, prim: FillPrimitive) -> FillId {
        let offset = self.shared_data.push(&prim);
        return FillId(GpuAddress::shared(offset));
    }

    fn add_stroke_primitve(&mut self, prim: StrokePrimitive) -> StrokeId {
        let offset = self.shared_data.push(&prim);
        return StrokeId(GpuAddress::shared(offset));
    }

    pub fn add_transform(&mut self, transform: &GpuTransform2D) -> TransformId {
        let offset = self.shared_data.push(transform);
        return TransformId(GpuAddress::shared(offset));
    }

    pub fn add_color(&mut self, color: &GpuColorF) -> ColorId {
        let offset = self.shared_data.push(color);
        return ColorId(GpuAddress::shared(offset));
    }

    pub fn add_instance_transform(&mut self, transform: &GpuTransform2D) -> TransformId {
        let offset = self.instance_data.push(transform);
        return TransformId(GpuAddress::instance(offset));
    }

    pub fn add_instance_color(&mut self, color: &GpuColorF) -> ColorId {
        let offset = self.instance_data.push(color);
        return ColorId(GpuAddress::instance(offset));
    }
}

#[derive(Debug)]
pub struct VectorImageInfo {
    geometry: GeometryId,
    id: VectorImageId,
    z_range: u32,
    mem_per_instance: u32,
    contains_fill_ops: bool,
    contains_stroke_ops: bool,
    shared_data: GpuData,
    shared_data_layout: MemoryLayout,
    instance_data: GpuData,
    instance_data_layout: MemoryLayout,
}

#[derive(Clone, Debug)]
pub struct VectorImageTemplate {
    info: Arc<VectorImageInfo>,
}

impl VectorImageTemplate {
    pub fn id(&self) -> VectorImageId { self.info.id }

    pub fn z_range(&self) -> u32 { self.info.z_range }

    pub fn geometry(&self) -> GeometryId { self.info.geometry }
}

impl VectorImageTemplate {
    pub fn new_instance(&self) -> VectorImageInstance {
        VectorImageInstance {
            template: self.clone(),
            gpu_data: self.info.instance_data.clone(),
        }
    }
}

#[derive(Clone)]
pub struct VectorImageInstance {
    pub template: VectorImageTemplate,
    gpu_data: GpuData,
}

impl VectorImageInstance {
    pub fn set_transform(&mut self, id: TransformId, transform: &GpuTransform2D) {
        self.gpu_data.set(id.0.offset(), transform);
    }

    pub fn set_color(&mut self, id: ColorId, color: &GpuColorF) {
        self.gpu_data.set(id.0.offset(), color);
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
            prim_id: (self.0).0 as i32,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrawCmd {
    pub geometry: GeometryId,
    pub num_instances: u32,
    pub base_address: GpuAddress,
}

struct LayerVectorImage {
    template: VectorImageTemplate,
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
        let z_range = instance.template.z_range();
        self.vector_images.entry(instance.template.id()).or_insert(
            LayerVectorImage {
                instances: Vec::new(),
                template: instance.template.clone(),
                allocated_range: None,
            }
        ).instances.push(RenderedInstance {
            instance: instance,
            z_index: self.z_index,
        });

        self.z_index += z_range;
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
            let range = ctx.device.allocate_gpu_data(item.template.info.mem_per_instance * num_instances);
            item.allocated_range = Some(range);

            // for each instance within a vector image
            let mut range_iter = range;
            for img_instance in &item.instances {
                ctx.device.set_gpu_data(range, &img_instance.instance.gpu_data);
                range_iter.shrink_left(item.template.info.mem_per_instance);
            }

            let base_address = range.start();

            if item.template.info.contains_fill_ops {
                fill_pass.push(DrawCmd {
                    geometry: item.template.info.geometry,
                    num_instances,
                    base_address,
                });
            }

            if item.template.info.contains_stroke_ops {
                stroke_pass.push(DrawCmd {
                    geometry: item.template.info.geometry,
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

#[derive(Clone, Debug)]
pub struct RenderPassOptions {
    pub vertex_type: VertexType,
    pub enable_blending: bool,
    pub enable_depth_write: bool,
    pub enable_depth_test: bool,
    pub effect: EffectId,
}

#[derive(Copy, Clone, Debug)]
pub enum VertexType {
    Fill,
    Stroke,
}

pub trait Device {
    fn allocate_gpu_data(&mut self, _size: u32) -> GpuAddressRange;
    fn set_gpu_data(&mut self, _range: GpuAddressRange, _data: &GpuData);
    fn submit_geometry(&mut self, geom: GeometryBuilder);
    fn render_pass(&mut self, cmds: &[DrawCmd], options: &RenderPassOptions);
}

#[test]
fn simple_vector_image() {
    use path_builder::*;

    struct DummyDevice(u32);
    impl Device for DummyDevice {
        fn allocate_gpu_data(&mut self, size: u32) -> GpuAddressRange {
            let start = GpuOffset(self.0);
            self.0 += size;
            let end = GpuOffset(self.0);
            GpuAddressRange { start: GpuAddress::global(start), end: GpuAddress::global(end) }
        }
        fn set_gpu_data(&mut self, range: GpuAddressRange, _data: &GpuData) {
            assert!(self.0 >= range.end.offset().as_u32());
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

    let global_transform = builder.add_transform(&GpuTransform2D::new(
        Transform2D::identity()
    ));
    let instance_transform = builder.add_instance_transform(&GpuTransform2D::new(
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
        [global_transform, instance_transform],
    );

    let template = builder.build(&mut geom);

    let mut img0 = template.new_instance();
    img0.set_transform(instance_transform, &GpuTransform2D::new(Transform2D::create_translation(1.0, 1.0)));

    let mut img1 = template.new_instance();
    img1.set_transform(instance_transform, &GpuTransform2D::new(Transform2D::create_translation(-2.0, 5.0)));

    let img0 = Rc::new(img0);
    let img1 = Rc::new(img1);

    let prim_id = GpuAddress::from_raw(geom.fill.vertices[0].prim_id as u32);

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
        let prim_offset = prim_id.offset().0 as usize;
        assert_eq!(
            &img0.template.info.shared_data.as_slice()[prim_offset..(prim_offset as usize + 4)],
            &[
                mem::transmute(0u32),
                mem::transmute(color),
                mem::transmute(global_transform),
                mem::transmute(instance_transform),
            ]
        );
    }
}

