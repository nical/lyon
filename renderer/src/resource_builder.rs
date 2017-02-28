use tessellation;
use tessellation::path_fill::*;
use tessellation::path_stroke::*;
use tessellation::basic_shapes;
use tessellation::geometry_builder::{VertexBuffers, VertexConstructor, BuffersBuilder};
use core::math::*;
use path_iterator::*;
use path::Path;
use renderer;
use renderer::ShapeDataId;
use buffer::{Id, IdRange};


use std::sync::Arc;
use std::marker::PhantomData;

pub enum Shape {
    Path(Arc<Path>),
    Ellipse(Point, Vec2),
    Rect(Rect),
}

pub struct ResourceBuilder {
    fill_tess: FillTessellator,
    stroke_tess: StrokeTessellator,
    resolved_requests: Vec<ResolvedRequest>
}

pub struct ResolvedRequest {
    pub mesh: TessRequest,
    pub request_id: RequestId,
}

pub enum TessRequest {
    FillMesh(VertexBuffers<renderer::FillVertex>),
    StrokeMesh(VertexBuffers<renderer::StrokeVertex>),
    Error,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RequestId(u16, u16);

pub struct FillRquest {
    pub shape: Shape,
    pub transform: Option<Transform2d>,
    pub tolerance: Option<f32>,
    pub shape_id: ShapeDataId,
    pub request_id: RequestId,
}

impl ResourceBuilder {
    pub fn request_fill(
        &mut self,
        shape: Shape,
        transform: Option<Transform2d>,
        options: &FillOptions,
        tolerance: Option<f32>,
        shape_id: renderer::ShapeDataId,
        id: RequestId
    ) {
        match shape {
            Shape::Path(path) => {
                self.request_path_fill(path, transform, options, tolerance, shape_id, id);
            }
            Shape::Rect(rect) => {
                self.request_rect_fill(&rect, transform, options, shape_id, id);
            }
            Shape::Ellipse(center, radii) => {
                self.request_ellipse_fill(center, radii, transform, options, tolerance, shape_id, id);
            }
        }
    }

    pub fn request_rect_fill(
        &mut self,
        rect: &Rect,
        transform: Option<Transform2d>,
        options: &FillOptions,
        shape_id: renderer::ShapeDataId,
        id: RequestId
    ) {
        let mut buffers = VertexBuffers::new();

        basic_shapes::fill_rectangle(
            rect,
            &mut BuffersBuilder::new(&mut buffers, WithShapeDataId(shape_id))
        );

        self.resolved_requests.push(
            ResolvedRequest {
                request_id: id,
                mesh: TessRequest::FillMesh(buffers),
            }
        );
    }
    pub fn request_ellipse_fill(
        &mut self,
        center: Point,
        radii: Vec2,
        transform: Option<Transform2d>,
        options: &FillOptions,
        tolerance: Option<f32>,
        shape_id: renderer::ShapeDataId,
        id: RequestId
    ) {
        let tolerance = tolerance.unwrap_or(0.5);
        let r = if radii.x > radii.y { radii.x } else { radii.y };
        let c = ::std::f32::consts::PI * r * 2.0;

        let num_points = (c / tolerance) as u32;

        let mut buffers = VertexBuffers::new();

        basic_shapes::fill_ellipse(
            center, radii, num_points,
            &mut BuffersBuilder::new(&mut buffers, WithShapeDataId(shape_id))
        );

        self.resolved_requests.push(
            ResolvedRequest {
                request_id: id,
                mesh: TessRequest::FillMesh(buffers),
            }
        );
    }

    pub fn request_path_fill(
        &mut self,
        path: Arc<Path>,
        transform: Option<Transform2d>,
        options: &FillOptions,
        tolerance: Option<f32>,
        shape_id: renderer::ShapeDataId,
        id: RequestId
    ) {
        let tolerance = tolerance.unwrap_or(0.5);

        let mut buffers = VertexBuffers::new();

        self.fill_tess.tessellate_path(
            path.path_iter().flattened(tolerance),
            options,
            &mut BuffersBuilder::new(&mut buffers, WithShapeDataId(shape_id))
        ).unwrap();

        self.resolved_requests.push(
            ResolvedRequest {
                request_id: id,
                mesh: TessRequest::FillMesh(buffers),
            }
        );
    }

    pub fn request_path_stroke(
        &mut self,
        path: Arc<Path>,
        transform: Option<Transform2d>,
        options: &StrokeOptions,
        tolerance: Option<f32>,
        shape_id: renderer::ShapeDataId,
        id: RequestId
    ) {
        let tolerance = tolerance.unwrap_or(0.5);

        let mut buffers = VertexBuffers::new();

        self.stroke_tess.tessellate(
            path.path_iter().flattened(tolerance),
            options,
            &mut BuffersBuilder::new(&mut buffers, WithShapeDataId(shape_id))
        ).unwrap();

        self.resolved_requests.push(
            ResolvedRequest {
                request_id: id,
                mesh: TessRequest::StrokeMesh(buffers),
            }
        );
    }
}

// Implement a vertex constructor.
// The vertex constructor sits between the tessellator and the geometry builder.
// it is called every time a new vertex needs to be added and creates a the vertex
// from the information provided by the tessellator.
//
// This vertex constructor forwards the positions and normals provided by the
// tessellators and add a shape id.
pub struct WithShapeDataId(pub renderer::ShapeDataId);

impl VertexConstructor<tessellation::StrokeVertex, renderer::StrokeVertex> for WithShapeDataId {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> renderer::StrokeVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        renderer::StrokeVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}

// The fill tessellator does not implement normals yet, so this implementation
// just sets it to [0, 0], for now.
impl VertexConstructor<tessellation::FillVertex, renderer::FillVertex> for WithShapeDataId {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> renderer::FillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        renderer::FillVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}

pub struct SimpleBufferAllocator {
    back_index: u16,
    front_index: u16,
    len: u16,
}

impl SimpleBufferAllocator {
    pub fn new(len: u16) -> Self {
        SimpleBufferAllocator {
            back_index: len,
            front_index: 0,
            len: len,
        }
    }

    pub fn len(&self) -> u16 { self.len }

    pub fn available_size(&self) -> u16 { self.back_index - self.front_index }

    pub fn alloc_range_dynamic(&mut self, len: u16) -> Option<(u16, u16)> {
        if self.available_size() < len {
            return None;
        }

        self.back_index -= len;

        return Some((self.back_index, len));
    }

    pub fn alloc_dynamic(&mut self) -> Option<u16> {
        self.alloc_range_dynamic(1).map(|range|{ range.0 })
    }

    pub fn alloc_range_static(&mut self, len: u16) -> Option<(u16, u16)> {
        if self.available_size() < len {
            return None;
        }

        let id = self.front_index;
        self.front_index += len;

        return Some((id, len));
    }

    pub fn alloc_static(&mut self) -> Option<u16> {
        self.alloc_range_static(1).map(|range|{ range.0 })
    }
}

pub struct TypedSimpleBufferAllocator<T> {
    alloc: SimpleBufferAllocator,
    _marker: PhantomData<T>,
}

impl<T> TypedSimpleBufferAllocator<T> {
    pub fn new(len: u16) -> Self {
        TypedSimpleBufferAllocator {
            alloc: SimpleBufferAllocator::new(len),
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> u16 { self.alloc.len() }

    pub fn alloc_dynamic(&mut self) -> Option<Id<T>> {
        self.alloc.alloc_dynamic().map(|id|{ Id::new(id) })
    }

    pub fn alloc_static(&mut self) -> Option<Id<T>> {
        self.alloc.alloc_static().map(|id|{ Id::new(id) })
    }

    pub fn alloc_range_dynamic(&mut self, len: u16) -> Option<IdRange<T>> {
        self.alloc.alloc_range_dynamic(len).map(|(first, count)|{
            IdRange::new(Id::new(first), count)
        })
    }

    pub fn alloc_range_static(&mut self, len: u16) -> Option<IdRange<T>> {
        self.alloc.alloc_range_static(len).map(|(first, count)|{
            IdRange::new(Id::new(first), count)
        })
    }
}
