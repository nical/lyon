use tessellation;
use tessellation::path_fill::*;
use tessellation::path_stroke::*;
use tessellation::basic_shapes;
use tessellation::geometry_builder::{VertexBuffers, VertexConstructor, BuffersBuilder};
use core::math::*;
use path_iterator::*;
use path::Path;
use renderer::{ PrimitiveId, GpuFillVertex, GpuStrokeVertex };
use renderer;

use std::sync::Arc;

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
    FillMesh(VertexBuffers<GpuFillVertex>),
    StrokeMesh(VertexBuffers<GpuStrokeVertex>),
    Error,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RequestId(u16, u16);

pub struct FillRquest {
    pub shape: Shape,
    pub transform: Option<Transform2d>,
    pub tolerance: Option<f32>,
    pub shape_id: PrimitiveId,
    pub request_id: RequestId,
}

impl ResourceBuilder {
    pub fn request_fill(
        &mut self,
        shape: Shape,
        transform: Option<Transform2d>,
        options: &FillOptions,
        tolerance: Option<f32>,
        shape_id: renderer::PrimitiveId,
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
        _transform: Option<Transform2d>,
        _options: &FillOptions,
        shape_id: renderer::PrimitiveId,
        id: RequestId
    ) {
        let mut buffers = VertexBuffers::new();

        basic_shapes::fill_rectangle(
            rect,
            &mut BuffersBuilder::new(&mut buffers, WithPrimitiveId(shape_id))
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
        _transform: Option<Transform2d>,
        _options: &FillOptions,
        tolerance: Option<f32>,
        shape_id: renderer::PrimitiveId,
        id: RequestId
    ) {
        let tolerance = tolerance.unwrap_or(0.5);
        let r = if radii.x > radii.y { radii.x } else { radii.y };
        let c = ::std::f32::consts::PI * r * 2.0;

        let num_points = (c / tolerance) as u32;

        let mut buffers = VertexBuffers::new();

        basic_shapes::fill_ellipse(
            center, radii, num_points,
            &mut BuffersBuilder::new(&mut buffers, WithPrimitiveId(shape_id))
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
        _transform: Option<Transform2d>,
        options: &FillOptions,
        tolerance: Option<f32>,
        shape_id: renderer::PrimitiveId,
        id: RequestId
    ) {
        let tolerance = tolerance.unwrap_or(0.5);

        let mut buffers = VertexBuffers::new();

        self.fill_tess.tessellate_path(
            path.path_iter().flattened(tolerance),
            options,
            &mut BuffersBuilder::new(&mut buffers, WithPrimitiveId(shape_id))
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
        _transform: Option<Transform2d>,
        options: &StrokeOptions,
        tolerance: Option<f32>,
        shape_id: renderer::PrimitiveId,
        id: RequestId
    ) {
        let tolerance = tolerance.unwrap_or(0.5);

        let mut buffers = VertexBuffers::new();

        self.stroke_tess.tessellate(
            path.path_iter().flattened(tolerance),
            options,
            &mut BuffersBuilder::new(&mut buffers, WithPrimitiveId(shape_id))
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
pub struct WithPrimitiveId(pub renderer::PrimitiveId);

impl VertexConstructor<tessellation::StrokeVertex, GpuStrokeVertex> for WithPrimitiveId {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex) -> GpuStrokeVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        GpuStrokeVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}

// The fill tessellator does not implement normals yet, so this implementation
// just sets it to [0, 0], for now.
impl VertexConstructor<tessellation::FillVertex, GpuFillVertex> for WithPrimitiveId {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> GpuFillVertex {
        assert!(!vertex.position.x.is_nan());
        assert!(!vertex.position.y.is_nan());
        assert!(!vertex.normal.x.is_nan());
        assert!(!vertex.normal.y.is_nan());
        GpuFillVertex {
            position: vertex.position.array(),
            normal: vertex.normal.array(),
            shape_id: self.0.to_i32(),
        }
    }
}
