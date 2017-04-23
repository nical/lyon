use std::sync::Arc;
use std::default::Default;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use api::*;
use buffer::*;
use path::Path;
use path_iterator::*;
use glsl::PRIM_BUFFER_LEN;
use renderer::{ GpuFillVertex, GpuStrokeVertex };
use renderer::{ GpuFillPrimitive, GpuStrokePrimitive };
use renderer::{ FillPrimitiveId, StrokePrimitiveId, WithId, GpuTransform };
use frame::{
    FillCmd, StrokeCmd, RenderTargetCmds, RenderTargetId,
    FillVertexBufferRange, StrokeVertexBufferRange, IndexBufferRange,
    FillVertexBufferId, StrokeVertexBufferId, IndexBufferId,
};

use core::math::*;
use tessellation::basic_shapes;
use tessellation::path_fill::*;
use tessellation::path_stroke::*;
use tessellation::geometry_builder::{ VertexBuffers, BuffersBuilder, Count };

pub type Geometry<VertexType> = VertexBuffers<VertexType>;

#[derive(Clone, Debug)]
pub struct RenderNodeInternal {
    descriptor: RenderNode,
    fill_prim: Option<BufferElement<GpuFillPrimitive>>,
    stroke_prim: Option<BufferElement<GpuStrokePrimitive>>,
    in_use: bool,
}

impl ::std::default::Default for RenderNodeInternal {
    fn default() -> Self {
        RenderNodeInternal {
            descriptor: RenderNode {
                shape: ShapeId::None,
                transform: None,
                stroke: None,
                fill: None,
            },
            fill_prim: None,
            stroke_prim: None,
            in_use: false,
        }
    }
}

pub struct ShapeData {
    path: Arc<Path>,
    fill: Option<FillGeometryRanges>,
    stroke: Option<StrokeGeometryRanges>,
}

pub struct GeometryStore<Vertex> {
    geom: Geometry<Vertex>,
    ranges: HashMap<ShapeId, GeometryRanges<Vertex>>,
}

impl<Vertex> GeometryStore<Vertex> {
    pub fn new() -> Self {
        Self {
            geom: Geometry::new(),
            ranges: HashMap::new(),
        }
    }

    pub fn get(&self, id: ShapeId) -> Option<&GeometryRanges<Vertex>> {
        self.ranges.get(&id)
    }

    pub fn clear(&mut self) {
        self.geom.vertices.clear();
        self.geom.indices.clear();
        self.ranges.clear();
    }
}

pub struct ShapeStore {
    paths: Vec<ShapeData>,
}

impl ShapeStore {
    pub fn new() -> Self { Self { paths: Vec::new() } }

    pub fn get_path(&self, id: PathId) -> &ShapeData {
        &self.paths[id.index()]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PrimitiveParams<Style> {
    pub z_index: u32,
    pub shape: ShapeId,
    pub transforms: Transforms,
    pub style: Style,
}

#[derive(Copy, Clone, Debug)]
pub struct Transforms {
    pub local: Option<TransformId>,
    pub view: Option<TransformId>,
}

pub trait VertexBuilder<PrimitiveId> {
    type Vertex;

    fn add_path(
        &mut self,
        path: &Path,
        prim_id: PrimitiveId,
        tolerance: f32,
        geom: &mut Geometry<Self::Vertex>
    ) -> GeometryRanges<Self::Vertex>;

    fn add_ellipse(
        &mut self,
        center: Point,
        radii: Vec2,
        prim_id: FillPrimitiveId,
        tolerance: f32,
        geom: &mut Geometry<Self::Vertex>
    ) -> GeometryRanges<Self::Vertex>;
}

pub trait PrimitiveBuilder<PrimitiveId, Params> {
    fn alloc_id(&mut self) -> PrimitiveId;
    fn build_primtive(&mut self, id: PrimitiveId, params: &Params);
}

#[derive(Clone)]
pub struct Cmd<Vertex> {
    pub geometry: GeometryRanges<Vertex>,
    pub instances: u32,
}

pub struct OpaqueBatcher<PrimitiveId, VertexBuilder> {
    render_nodes: Vec<PrimitiveParams<FillStyle>>,
    allocated_primitives: Vec<Option<PrimitiveId>>,
    vertex_builder: VertexBuilder,
}

impl<PrimitiveId, VtxBuilder> OpaqueBatcher<PrimitiveId, VtxBuilder>
where
    VtxBuilder: VertexBuilder<PrimitiveId>,
    PrimitiveId: Copy,
{
    pub fn new(vertex_builder: VtxBuilder) -> Self {
        Self {
            render_nodes: Vec::new(),
            allocated_primitives: Vec::new(),
            vertex_builder: vertex_builder,
        }
    }

    pub fn fill_shape(&mut self, params: PrimitiveParams<FillStyle>){
        self.render_nodes.push(params);
        self.allocated_primitives.push(None);
    }

    pub fn clear(&mut self) {
        self.render_nodes.clear();
        self.allocated_primitives.clear();
    }

    pub fn build<PrimBuilder: PrimitiveBuilder<PrimitiveId, PrimitiveParams<FillStyle>>>(
        &mut self,
        shapes: &ShapeStore,
        geom_store: &mut GeometryStore<VtxBuilder::Vertex>,
        prim_builder: &mut PrimBuilder,
    ) -> Vec<Cmd<VtxBuilder::Vertex>> {
        // This is a gross overestimate if commands get merged through batching or instancing.
        let mut cmds = Vec::with_capacity(self.render_nodes.len());

        let tolerance = 0.5;

        // Go through render nodes in reverse order to make it more likely that
        // primitives are rendered front to back.
        for index in (0..self.render_nodes.len()).rev() {
            let node = &mut self.render_nodes[index];
            let allocated_primitive = &mut self.allocated_primitives[index];

            let prim_id = allocated_primitive.unwrap_or_else(&mut||{
                prim_builder.alloc_id()
            });
            *allocated_primitive = Some(prim_id);

            prim_builder.build_primtive(prim_id, node);

            let draw_cmd = Cmd {
                geometry: match geom_store.ranges.entry(node.shape) {
                    Entry::Occupied(entry) => {
                        *entry.get()
                    }
                    Entry::Vacant(entry) => {
                        match node.shape {
                            ShapeId::Path(path_id) => {
                                // TODO: move this to a worker thread?
                                let geom = self.vertex_builder.add_path(
                                    &*shapes.get_path(path_id).path,
                                    prim_id,
                                    tolerance,
                                    &mut geom_store.geom,
                                );
                                entry.insert(geom);

                                geom
                            }
                            _ => { unimplemented!(); }
                        }
                    },
                },
                instances: 1,
            };

            // TODO: if current geom == previous geom && prim_id = previous id + 1
            // just increment the previous command's instance count.
            // or do it as a later pass ?

            cmds.push(draw_cmd);
        }

        return cmds;
    }
}

pub struct FillPrimitiveBuilder<'l> {
    // TODO: move this to a more generic primitive store where data is just put into
    // a texture like webrender.
    pub primitives: &'l mut CpuBuffer<GpuFillPrimitive>,
}

impl<'l> PrimitiveBuilder<FillPrimitiveId, PrimitiveParams<FillStyle>> for FillPrimitiveBuilder<'l> {
    fn alloc_id(&mut self) -> FillPrimitiveId {
        self.primitives.alloc()
    }

    fn build_primtive(&mut self, id: FillPrimitiveId, params: &PrimitiveParams<FillStyle>) {
        let default_transform = TransformId { buffer: BufferId::new(0), element: Id::new(0) };
        self.primitives[id] = GpuFillPrimitive {
            color: match params.style.pattern {
                Pattern::Color(color) => { color.f32_array() }
                _ => { unimplemented!(); }
            },
            z_index: params.z_index as f32 / 1000.0,
            local_transform: params.transforms.local.unwrap_or(default_transform).element.to_i32(),
            view_transform: params.transforms.view.unwrap_or(default_transform).element.to_i32(),
            width: 0.0,
            .. Default::default()
        };
    }
}

pub struct FillVertexBuilder {
    tessellator: FillTessellator,
    // TODO: this is bogus, should get that info from the buffers.
    offsets: Count,
}

impl FillVertexBuilder {
    pub fn new() -> Self {
        Self {
            tessellator: FillTessellator::new(),
            offsets: Count { vertices: 0, indices: 0 },
        }
    }
}

impl VertexBuilder<FillPrimitiveId> for FillVertexBuilder {
    type Vertex = GpuFillVertex;

    fn add_path(
        &mut self,
        path: &Path,
        prim_id: FillPrimitiveId,
        tolerance: f32,
        geom: &mut Geometry<GpuFillVertex>
    ) -> GeometryRanges<GpuFillVertex> {
        let count = self.tessellator.tessellate_path(
            path.path_iter().flattened(tolerance),
            &FillOptions::default(),
            &mut BuffersBuilder::new(geom, WithId(prim_id))
        ).unwrap();

        self.offsets = self.offsets + count;

        return FillGeometryRanges {
            vertices: FillVertexBufferRange {
                buffer: BufferId::new(0),
                range: IdRange::from_start_count(self.offsets.vertices as u16, count.vertices as u16),
            },
            indices: IndexBufferRange {
                buffer: BufferId::new(0),
                range: IdRange::from_start_count(self.offsets.indices as u16, count.indices as u16),
            },
        };
    }

    fn add_ellipse(
        &mut self,
        center: Point,
        radii: Vec2,
        prim_id: FillPrimitiveId,
        tolerance: f32,
        geom: &mut Geometry<GpuFillVertex>
    ) -> GeometryRanges<GpuFillVertex> {
        // TODO: compute num vertices for a given tolerance!
        let count = basic_shapes::fill_ellipse(
            center, radii, 64,
            &mut BuffersBuilder::new(geom, WithId(prim_id))
        );

        self.offsets = self.offsets + count;

        return FillGeometryRanges {
            vertices: FillVertexBufferRange {
                buffer: BufferId::new(0),
                range: IdRange::from_start_count(self.offsets.vertices as u16, count.vertices as u16),
            },
            indices: IndexBufferRange {
                buffer: BufferId::new(0),
                range: IdRange::from_start_count(self.offsets.indices as u16, count.indices as u16),
            },
        };
    }
}


impl<T> Copy for GeometryRanges<T> {}
impl<T> Clone for GeometryRanges<T> { fn clone(&self) -> Self { *self } }
#[derive(Debug)]
pub struct GeometryRanges<Vertex> {
    pub vertices: BufferRange<Vertex>,
    pub indices: IndexBufferRange,
}

pub type FillGeometryRanges = GeometryRanges<GpuFillVertex>;
pub type StrokeGeometryRanges = GeometryRanges<GpuStrokeVertex>;

// TODO: remove
struct StrokeCtx<'l> {
    tessellator: StrokeTessellator,
    buffers: &'l mut VertexBuffers<GpuStrokeVertex>,
    offsets: Count,
    vbo: StrokeVertexBufferId,
    ibo: IndexBufferId,
}

impl<'l> StrokeCtx<'l> {
    fn add_path(&mut self, path: &Path, prim_id: StrokePrimitiveId, tolerance: f32) -> StrokeGeometryRanges {
        let count = self.tessellator.tessellate(
            path.path_iter().flattened(tolerance),
            &StrokeOptions::default(),
            &mut BuffersBuilder::new(self.buffers, WithId(prim_id))
        ).unwrap();

        self.offsets = self.offsets + count;

        return StrokeGeometryRanges {
            vertices: StrokeVertexBufferRange {
                buffer: self.vbo,
                range: IdRange::from_start_count(self.offsets.vertices as u16, count.vertices as u16),
            },
            indices: IndexBufferRange {
                buffer: self.ibo,
                range: IdRange::from_start_count(self.offsets.indices as u16, count.indices as u16),
            },
        };
    }
}


#[test]
fn simple_frame() {
    use api::PathId;

    let mut batcher = OpaqueBatcher::new(FillVertexBuilder::new());
    let shapes = ShapeStore::new();
    let mut geom = GeometryStore::new();
    let mut primitives = CpuBuffer::new(1024);
    let mut prim_builder = FillPrimitiveBuilder {
        primitives: &mut primitives,
    };

    let cmds = batcher.build(&shapes, &mut geom, &mut prim_builder);
}
