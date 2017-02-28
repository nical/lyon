use std::collections::HashMap;
use std::sync::Arc;
use std::default::Default;

use api::*;
use path::Path;
use path_iterator::*;
use resource_builder;
use resource_builder::{ ResourceBuilder, WithShapeDataId, TypedSimpleBufferAllocator };
use renderer::FillVertex as GpuFillVertex;
use renderer::StrokeVertex as GpuStrokeVertex;
use renderer::ShapeData as GpuPrimData;
use renderer::ShapeDataId as PrimDataId;

use frame::{
    DrawCmd, RenderTargetCmds, RenderTargetId,
    VertexBufferRange, IndexBufferRange, VertexBufferId, IndexBufferId, UniformBufferId
};

use core::math::*;
use tessellation::basic_shapes;
use tessellation::path_fill::*;
use tessellation::path_stroke::*;
use tessellation::geometry_builder::{VertexBuffers, VertexConstructor, BuffersBuilder, Count};

pub type CpuMesh<VertexType> = VertexBuffers<VertexType>;

#[derive(Clone, Debug)]
pub struct RenderNodeInternal {
    descriptor: RenderNode,
    fill_prim: Option<PrimDataId>,
    stroke_prim: Option<PrimDataId>,
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

pub struct FrameBuilder {
    render_nodes: Vec<RenderNodeInternal>,

    animated_transforms: Vec<Transform>,
    animated_colors: Vec<Color>,
    animated_floats: Vec<f32>,

    static_transforms: Vec<Transform>,
    static_colors: Vec<Color>,
    static_floats: Vec<f32>,

    prim_data: Vec<GpuPrimData>,

    paths: Vec<PathTemplate>,
    // the vbo allocator
    path_meshes: Vec<PathMeshData>,
    // the cpu-side tessellated meshes
    fill_meshes: Vec<CpuMesh<GpuFillVertex>>,
    stroke_meshes: Vec<CpuMesh<GpuStrokeVertex>>,

    transform_alloc: TypedSimpleBufferAllocator<TransformId>,
    prim_alloc: TypedSimpleBufferAllocator<GpuPrimData>,
}

struct PathTemplate {
    data: Arc<Path>,
    fill_mesh: Option<usize>,
    stroke_mesh: Option<usize>,
}

struct PathMeshData {
    fill: Option<MeshData>,
    stroke: Option<MeshData>,
}

impl Default for PathMeshData {
    fn default() -> Self { PathMeshData { fill: None, stroke: None, } }
}

#[derive(Copy, Clone, Debug)]
pub struct MeshData {
    pub vertices: VertexBufferRange,
    pub indices: IndexBufferRange,
}

unsafe impl Send for FrameBuilder {}

impl FrameBuilder {
    pub fn new() -> Self {
        FrameBuilder {
            render_nodes: vec![Default::default(); 128],
            prim_data: vec![Default::default(); 128],
            animated_transforms: Vec::new(),
            animated_colors: Vec::new(),
            animated_floats: Vec::new(),
            static_transforms: Vec::new(),
            static_colors: Vec::new(),
            static_floats: Vec::new(),
            paths: Vec::new(),
            path_meshes: Vec::new(),
            fill_meshes: Vec::new(),
            stroke_meshes: Vec::new(),
            transform_alloc: TypedSimpleBufferAllocator::new(2048),
            prim_alloc: TypedSimpleBufferAllocator::new(2048),
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
                    z += 1;
                } else {
                    transparent_fills.push(Op { z_index: z, render_node: node });
                    z += 1;
                }
            }
            if let Some(ref style) = render_node.descriptor.stroke {
                if style.pattern.is_opaque() {
                    opaque_strokes.push(Op { z_index: z, render_node: node });
                    z += 1;
                } else {
                    transparent_strokes.push(Op { z_index: z, render_node: node });
                    z += 1;
                }
            }
            node += 1;
        }
        opaque_fills.reverse();
        opaque_strokes.reverse();

        let mut fill_ctx = FillCtx {
            tessellator: FillTessellator::new(),
            offsets: Count { vertices: 0, indices: 0 },
            buffers: &mut VertexBuffers::new(),
            vbo: VertexBufferId(0),
            ibo: IndexBufferId(0),
        };
        let mut stroke_ctx = StrokeCtx {
            tessellator: StrokeTessellator::new(),
            offsets: Count { vertices: 0, indices: 0 },
            buffers: &mut VertexBuffers::new(),
            vbo: VertexBufferId(0),
            ibo: IndexBufferId(0),
        };

        let default_cmd_params = DrawCmd {
            mesh: MeshData {
                vertices: VertexBufferRange {
                    buffer: VertexBufferId(0),
                    first: 0, count: 0,
                },
                indices: IndexBufferRange {
                    buffer: IndexBufferId(0),
                    first: 0, count: 0,
                },
            },
            instances: 1,
            prim_data: UniformBufferId(0), // TODO
            transforms: UniformBufferId(0),
        };

        let default_transform = TransformId::new(0);
        let tolerance = 0.5;

        let mut frame = RenderTargetCmds::new(RenderTargetId(0));

        for of in &opaque_fills {
            let node = &mut self.render_nodes[of.render_node as usize];

            if node.fill_prim.is_none() {
                node.fill_prim = self.prim_alloc.alloc_static();
            }
            let prim_id = node.fill_prim.unwrap();

            self.prim_data[prim_id.index()] = GpuPrimData {
                color: match node.descriptor.fill.as_ref().unwrap().pattern {
                    Pattern::Color(color) => { color.f32_array() }
                    _ => { unimplemented!(); }
                },
                z_index: of.z_index as f32 / 1000.0,
                transform_id: node.descriptor.transform.as_ref().unwrap_or(&default_transform).to_i32(),
                width: 0.0,
                .. Default::default()
            };

            let draw_cmd = match node.descriptor.shape {
                ShapeId::Path(path_id) => {
                    if let Some(mesh) = self.path_meshes[path_id.index()].fill {
                        DrawCmd { mesh: mesh, ..default_cmd_params }
                    } else {
                        // TODO: if let Some(mesh_id) = self.paths[path_id.index()].fill_mesh {
                        //     if self.mesh_cache.contains(mesh_id) {
                        //         self.mesh_cache.mark_used(mesh_id)
                        //     } else {
                        //         tessellate and push into cache.
                        //     }
                        // }
                        //
                        //
                        let mesh = fill_ctx.add_path(&self.paths[path_id.index()].data, prim_id, tolerance);
                        self.path_meshes[path_id.index()].fill = Some(mesh);
                        DrawCmd { mesh: mesh, ..default_cmd_params }
                    }
                }
                _ => { unimplemented!(); }
            };

            frame.opaque_fills.push(draw_cmd);
        }

        for os in &opaque_strokes {
            let node = &mut self.render_nodes[os.render_node as usize];

            if node.stroke_prim.is_none() {
                node.stroke_prim = self.prim_alloc.alloc_static();
            }
            let prim_id = node.stroke_prim.unwrap();

            let stroke_style = &node.descriptor.stroke.as_ref().unwrap();
            self.prim_data[prim_id.index()] = GpuPrimData {
                color: match stroke_style.pattern {
                    Pattern::Color(color) => { color.f32_array() }
                    _ => { unimplemented!(); }
                },
                z_index: os.z_index as f32 / 1000.0,
                transform_id: node.descriptor.transform.as_ref().unwrap_or(&default_transform).to_i32(),
                width: stroke_style.width,
                .. Default::default()
            };

            match node.descriptor.shape {
                ShapeId::Path(path_id) => {
                    let draw_cmd = if let Some(mesh) = self.path_meshes[path_id.index()].stroke {
                        DrawCmd { mesh: mesh, ..default_cmd_params }
                    } else {
                        let mesh = stroke_ctx.add_path(&self.paths[path_id.index()].data, prim_id, tolerance);
                        self.path_meshes[path_id.index()].stroke = Some(mesh);
                        DrawCmd { mesh: mesh, ..default_cmd_params }
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
    vbo: VertexBufferId,
    ibo: IndexBufferId,
}

impl<'l> FillCtx<'l> {
    fn add_path(&mut self, path: &Path, prim_id: PrimDataId, tolerance: f32) -> MeshData {
        let count = self.tessellator.tessellate_path(
            path.path_iter().flattened(tolerance),
            &FillOptions::default(),
            &mut BuffersBuilder::new(self.buffers, WithShapeDataId(prim_id))
        ).unwrap();

        self.offsets = self.offsets + count;

        return MeshData {
            vertices: VertexBufferRange {
                buffer: self.vbo,
                first: self.offsets.vertices as u16,
                count: count.vertices as u16,
            },
            indices: IndexBufferRange {
                buffer: self.ibo,
                first: self.offsets.indices as u16,
                count: count.indices as u16,
            },
        };
    }

    fn add_ellipse(&mut self, center: Point, radii: Vec2, prim_id: PrimDataId, tolerance: f32) -> MeshData {
        // TODO: compute num vertices for a given tolerance!
        let count = basic_shapes::fill_ellipse(
            center, radii, 64,
            &mut BuffersBuilder::new(&mut self.buffers, WithShapeDataId(prim_id))
        );

        self.offsets = self.offsets + count;

        return MeshData {
            vertices: VertexBufferRange {
                buffer: self.vbo,
                first: self.offsets.vertices as u16,
                count: count.vertices as u16,
            },
            indices: IndexBufferRange {
                buffer: self.ibo,
                first: self.offsets.indices as u16,
                count: count.indices as u16,
            },
        };
    }
}

struct StrokeCtx<'l> {
    tessellator: StrokeTessellator,
    buffers: &'l mut VertexBuffers<GpuStrokeVertex>,
    offsets: Count,
    vbo: VertexBufferId,
    ibo: IndexBufferId,
}

impl<'l> StrokeCtx<'l> {
    fn add_path(&mut self, path: &Path, prim_id: PrimDataId, tolerance: f32) -> MeshData {
        let count = self.tessellator.tessellate(
            path.path_iter().flattened(tolerance),
            &StrokeOptions::default(),
            &mut BuffersBuilder::new(self.buffers, WithShapeDataId(prim_id))
        ).unwrap();

        self.offsets = self.offsets + count;

        return MeshData {
            vertices: VertexBufferRange {
                buffer: self.vbo,
                first: self.offsets.vertices as u16,
                count: count.vertices as u16,
            },
            indices: IndexBufferRange {
                buffer: self.ibo,
                first: self.offsets.indices as u16,
                count: count.indices as u16,
            },
        };
    }
}


#[test]
fn simple_frame() {
    use api::PathId;

    let mut frame_builder = FrameBuilder::new();

    let node_id = RenderNodeId::new(0);
    let shape_id = ShapeId::Path(PathId::new(0));

    frame_builder.create_render_node(node_id, RenderNode {
        shape: shape_id,
        transform: None,
        fill: Some(FillStyle {
            pattern: Pattern::Color(Color::black()),
            aa: false,
        }),
        stroke: None,
    });

    let frame = frame_builder.build_frame();
}