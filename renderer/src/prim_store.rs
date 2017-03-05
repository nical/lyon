use buffer::CpuBuffer;
use tessellation::geometry_builder::VertexBuffers;
//use renderer::{ GpuFillVertex, GpuStrokeVertex, GpuFillPrimitive, GpuStrokePrimitive, GpuTransform };


pub struct PrimStore<Vertex, Primitive> {
    pub geometry: GeometryStore<Vertex>,
    pub primitives: BufferStore<Primitive>,
}

impl<Vertex, Primitive> PrimStore<Vertex, Primitive> {
    pub fn new() -> Self {
        PrimStore {
            geometry: GeometryStore::new(),
            primitives: BufferStore::new(),
        }
    }
}

pub struct BufferStore<Primitive> {
    pub buffers: Vec<CpuBuffer<Primitive>>,
}

impl<Primitive> BufferStore<Primitive> {
    pub fn new() -> Self { BufferStore { buffers: Vec::new() } }
}

pub struct GeometryStore<Vertex> {
    pub buffers: Vec<VertexBuffers<Vertex>>,
}

impl<Vertex> GeometryStore<Vertex> {
    pub fn new() -> Self { GeometryStore { buffers: Vec::new() } }
}

