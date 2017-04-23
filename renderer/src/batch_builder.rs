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

pub struct BatchBuilder {
    render_nodes: Vec<RenderNodeInternal>,

    transforms: BufferStore<GpuTransform>,
    fill_primitives: BufferStore<GpuFillPrimitive>,
    stroke_primitives: BufferStore<GpuStrokePrimitive>,

    shapes: Vec<ShapeData>,
    // the cpu-side tessellated meshes
    fill_geom: Geometry<GpuFillVertex>,
    stroke_geom: Geometry<GpuStrokeVertex>,
}

struct ShapeData {
    path: Arc<Path>,
    fill: Option<FillGeometryRanges>,
    stroke: Option<StrokeGeometryRanges>,
}

pub struct GeometryStore<Vertex> {
    geom: Geometry<Vertex>,
    ranges: HashMap<ShapeId, GeometryRanges<Vertex>>,
}

impl<Vertex> GeometryStore<Vertex> {
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
    fn get_path(&self, id: PathId) -> &ShapeData {
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

pub trait VertexBuilder<Vertex, PrimitiveId> {
    fn add_path(
        &mut self,
        path: &Path,
        prim_id: PrimitiveId,
        tolerance: f32,
        geom: &mut Geometry<Vertex>
    ) -> FillGeometryRanges;
}

pub struct OpaqueBatcher<VertexBuilder> {
    render_nodes: Vec<PrimitiveParams<FillStyle>>,
    allocated_primitives: Vec<Option<BufferElement<GpuFillPrimitive>>>,
    vertex_builder: VertexBuilder,
}

impl<V: VertexBuilder<GpuFillVertex, FillPrimitiveId>> OpaqueBatcher<V> {
    pub fn new(vertex_builder: V) -> Self {
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

    pub fn build(
        &mut self,
        shapes: &ShapeStore,
        geom_store: &mut GeometryStore<GpuFillVertex>,
        prim_store: &mut BufferStore<GpuFillPrimitive>,
    ) -> Vec<FillCmd> {
        // This is a gross overestimate if commands get merged through batching or instancing.
        let mut cmds = Vec::with_capacity(self.render_nodes.len());

        let default_transform = TransformId { buffer: BufferId::new(0), element: Id::new(0) };
        let tolerance = 0.5;

        // Go through render nodes in reverse order to make it more likely that
        // primitives are rendered front to back.
        for index in (0..self.render_nodes.len()).rev() {
            let node = &mut self.render_nodes[index];
            let allocated_primitive = &mut self.allocated_primitives[index];

            let prim_id = allocated_primitive.unwrap_or_else(&mut||{
                prim_store.alloc()
            });
            *allocated_primitive = Some(prim_id);

            prim_store[prim_id] = GpuFillPrimitive {
                color: match node.style.pattern {
                    Pattern::Color(color) => { color.f32_array() }
                    _ => { unimplemented!(); }
                },
                z_index: node.z_index as f32 / 1000.0,
                local_transform: node.transforms.local.unwrap_or(default_transform).element.to_i32(),
                view_transform: node.transforms.view.unwrap_or(default_transform).element.to_i32(),
                width: 0.0,
                .. Default::default()
            };

            let draw_cmd = match geom_store.ranges.entry(node.shape) {
                Entry::Occupied(entry) => {
                    FillCmd { geometry: *entry.get(), ..FillCmd::default() }
                }
                Entry::Vacant(entry) => {
                    match node.shape {
                        ShapeId::Path(path_id) => {
                            let geom = self.vertex_builder.add_path(
                                &*shapes.get_path(path_id).path,
                                prim_id.element,
                                tolerance,
                                &mut geom_store.geom,
                            );
                            entry.insert(geom);
                            FillCmd { geometry: geom, ..FillCmd::default() }
                        }
                        _ => { unimplemented!(); }
                    }
                }
            };

            // TODO: if current geom == previous geom && prim_id = previous id + 1
            // just increment the previous command's instance count.
            // or do it as a later pass ?

            cmds.push(draw_cmd);
        }

        return cmds;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GeometryRanges<Vertex> {
    pub vertices: BufferRange<Vertex>,
    pub indices: IndexBufferRange,
}

pub type FillGeometryRanges = GeometryRanges<GpuFillVertex>;
pub type StrokeGeometryRanges = GeometryRanges<GpuStrokeVertex>;

unsafe impl Send for BatchBuilder {}

impl BatchBuilder {
    pub fn new() -> Self {
        BatchBuilder {
            render_nodes: vec![Default::default(); 128],
            fill_primitives: BufferStore::new(PRIM_BUFFER_LEN as u16, 1),
            stroke_primitives: BufferStore::new(PRIM_BUFFER_LEN as u16, 1),
            transforms: BufferStore::new(PRIM_BUFFER_LEN as u16, 1),
            shapes: Vec::new(),
            fill_geom: Geometry::new(),
            stroke_geom: Geometry::new(),
        }
    }

    pub fn create_render_node(&mut self, id: RenderNodeId, descriptor: RenderNode) {
        assert!(!self.render_nodes[id.index()].in_use);
        self.render_nodes[id.index()] = RenderNodeInternal {
            descriptor: descriptor,
            fill_prim: None,
            stroke_prim: None,
            in_use: true,
        };
    }

    pub fn remove_render_node(&mut self, id: RenderNodeId) {
        assert!(self.render_nodes[id.index()].in_use);
        self.render_nodes[id.index()].in_use = false;
    }

    pub fn build_frame(&mut self) {
        let mut opaque_fills = Vec::new();
        let mut opaque_strokes = Vec::new();
        let mut transparent_fills = Vec::new();
        let mut transparent_strokes = Vec::new();

        struct Op {
            z_index: u32,
            render_node: u32,
        }

        let mut z = 0;
        let mut node = 0;
        for render_node in &mut self.render_nodes {
            if !render_node.in_use {
                continue;
            }
            if let Some(ref style) = render_node.descriptor.fill {
                if style.pattern.is_opaque() {
                    opaque_fills.push(Op { z_index: z, render_node: node });
                } else {
                    transparent_fills.push(Op { z_index: z, render_node: node });
                }
                z += 1;
            }
            if let Some(ref style) = render_node.descriptor.stroke {
                if style.pattern.is_opaque() {
                    opaque_strokes.push(Op { z_index: z, render_node: node });
                } else {
                    transparent_strokes.push(Op { z_index: z, render_node: node });
                }
                z += 1;
            }
            node += 1;
        }
        opaque_fills.reverse();
        opaque_strokes.reverse();

        let mut fill_ctx = FillCtx {
            tessellator: FillTessellator::new(),
            offsets: Count { vertices: 0, indices: 0 },
            buffers: &mut self.fill_geom,
            vbo: FillVertexBufferId::new(0),
            ibo: IndexBufferId::new(0),
        };
        let mut stroke_ctx = StrokeCtx {
            tessellator: StrokeTessellator::new(),
            offsets: Count { vertices: 0, indices: 0 },
            buffers: &mut self.stroke_geom,
            vbo: StrokeVertexBufferId::new(0),
            ibo: IndexBufferId::new(0),
        };

        let default_transform = TransformId { buffer: BufferId::new(0), element: Id::new(0) };
        let tolerance = 0.5;

        let mut frame = RenderTargetCmds::new(RenderTargetId(0));

        for of in &opaque_fills {
            let node = &mut self.render_nodes[of.render_node as usize];

            if node.fill_prim.is_none() {
                node.fill_prim = Some(self.fill_primitives.alloc());
            }

            let prim_id = node.fill_prim.unwrap();

            self.fill_primitives[prim_id] = GpuFillPrimitive {
                color: match node.descriptor.fill.as_ref().unwrap().pattern {
                    Pattern::Color(color) => { color.f32_array() }
                    _ => { unimplemented!(); }
                },
                z_index: of.z_index as f32 / 1000.0,
                local_transform: node.descriptor.transform.unwrap_or(default_transform).element.to_i32(),
                width: 0.0,
                .. Default::default()
            };

            let draw_cmd = match node.descriptor.shape {
                ShapeId::Path(path_id) => {
                    if let Some(geom) = self.shapes[path_id.index()].fill {
                        FillCmd { geometry: geom, ..FillCmd::default() }
                    } else {
                        let geom = fill_ctx.add_path(&self.shapes[path_id.index()].path, prim_id.element, tolerance);
                        self.shapes[path_id.index()].fill = Some(geom);
                        FillCmd { geometry: geom, ..FillCmd::default() }
                    }
                }
                _ => { unimplemented!(); }
            };

            frame.opaque_fills.push(draw_cmd);
        }

        for os in &opaque_strokes {
            let node = &mut self.render_nodes[os.render_node as usize];

            if node.stroke_prim.is_none() {
                node.stroke_prim = Some(self.stroke_primitives.alloc());
            }
            let prim_id = node.stroke_prim.unwrap();

            let stroke_style = &node.descriptor.stroke.as_ref().unwrap();
            self.stroke_primitives[prim_id] = GpuStrokePrimitive {
                color: match stroke_style.pattern {
                    Pattern::Color(color) => { color.f32_array() }
                    _ => { unimplemented!(); }
                },
                z_index: os.z_index as f32 / 1000.0,
                local_transform: node.descriptor.transform.unwrap_or(default_transform).element.to_i32(),
                width: stroke_style.width,
                .. Default::default()
            };

            match node.descriptor.shape {
                ShapeId::Path(path_id) => {
                    let draw_cmd = if let Some(geom) = self.shapes[path_id.index()].stroke {
                        StrokeCmd { geometry: geom, ..StrokeCmd::default() }
                    } else {
                        let geom = stroke_ctx.add_path(&self.shapes[path_id.index()].path, prim_id.element, tolerance);
                        self.shapes[path_id.index()].stroke = Some(geom);
                        StrokeCmd { geometry: geom, ..StrokeCmd::default() }
                    };

                    frame.opaque_strokes.push(draw_cmd);
                }
                _ => { unimplemented!(); }
            };
        }
    }
}

struct FillCtx<'l> {
    tessellator: FillTessellator,
    buffers: &'l mut VertexBuffers<GpuFillVertex>,
    offsets: Count,
    vbo: FillVertexBufferId,
    ibo: IndexBufferId,
}

impl<'l> FillCtx<'l> {
    fn add_path(&mut self, path: &Path, prim_id: FillPrimitiveId, tolerance: f32) -> FillGeometryRanges {
        let count = self.tessellator.tessellate_path(
            path.path_iter().flattened(tolerance),
            &FillOptions::default(),
            &mut BuffersBuilder::new(self.buffers, WithId(prim_id))
        ).unwrap();

        self.offsets = self.offsets + count;

        return FillGeometryRanges {
            vertices: FillVertexBufferRange {
                buffer: self.vbo,
                range: IdRange::from_start_count(self.offsets.vertices as u16, count.vertices as u16),
            },
            indices: IndexBufferRange {
                buffer: self.ibo,
                range: IdRange::from_start_count(self.offsets.indices as u16, count.indices as u16),
            },
        };
    }

    fn add_ellipse(&mut self, center: Point, radii: Vec2, prim_id: FillPrimitiveId, tolerance: f32) -> FillGeometryRanges {
        // TODO: compute num vertices for a given tolerance!
        let count = basic_shapes::fill_ellipse(
            center, radii, 64,
            &mut BuffersBuilder::new(&mut self.buffers, WithId(prim_id))
        );

        self.offsets = self.offsets + count;

        return FillGeometryRanges {
            vertices: FillVertexBufferRange {
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

    let mut frame_builder = BatchBuilder::new();

    let node_id = RenderNodeId::new(0);
    let prim_id = ShapeId::Path(PathId::new(0));

    frame_builder.create_render_node(node_id, RenderNode {
        shape: prim_id,
        transform: None,
        fill: Some(FillStyle {
            pattern: Pattern::Color(Color::black()),
            aa: false,
        }),
        stroke: None,
    });

    let frame = frame_builder.build_frame();
}
