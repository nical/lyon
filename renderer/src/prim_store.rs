use buffer::CpuBuffer;
use tessellation::geometry_builder::VertexBuffers;
use buffer::{BufferRange, BufferId, BufferElement };
use std::ops;

pub struct PrimStore<Vertex, Primitive> {
    pub geometry: GeometryStore<Vertex>,
    pub primitives: BufferStore<Primitive>,
}

impl<Vertex: Copy, Primitive: Copy+Default> PrimStore<Vertex, Primitive> {
    pub fn new() -> Self {
        PrimStore {
            geometry: GeometryStore::new(0),
            primitives: BufferStore::new(0, 0),
        }
    }
}

pub struct BufferStore<Primitive> {
    pub buffers: Vec<CpuBuffer<Primitive>>,
    current: BufferId<Primitive>,
    buffer_len: u16,
}

impl<Primitive: Copy+Default> BufferStore<Primitive> {
    pub fn new(count: u16, size: u16) -> Self {
        let mut store = BufferStore {
            buffers: Vec::new(),
            current: BufferId::new(0),
            buffer_len: size,
        };
        for _ in 0..count {
            store.alloc_buffer();
        }
        return store;
    }

    pub fn first_buffer_id(&self) -> BufferId<Primitive> { BufferId::new(0) }

    pub fn current_buffer_id(&self) -> BufferId<Primitive> { self.current }

    pub fn current_buffer(&self) -> &CpuBuffer<Primitive> { &self[self.current] }

    pub fn mut_current_buffer(&mut self) -> &mut CpuBuffer<Primitive> {
        let current = self.current;
        &mut self[current]
    }

    pub fn bump_current_buffer(&mut self) {
        self.current = BufferId::new(self.current.to_u32() + 1);
        while self.current.index() >= self.buffers.len() {
            self.alloc_buffer();
        }
    }

    pub fn alloc_buffer(&mut self) {
        let len = self.buffer_len;
        self.buffers.push(CpuBuffer::new(len));
    }

    pub fn alloc_range(&mut self, count: u16) -> BufferRange<Primitive> {
        assert!(count <= self.buffer_len);
        loop {
            if let Some(range) = self.mut_current_buffer().try_alloc_range(count) {
                return BufferRange {
                    buffer: self.current,
                    range: range,
                }
            }
            self.bump_current_buffer();
        }
    }

    pub fn alloc(&mut self) -> BufferElement<Primitive> {
        loop {
            if let Some(id) = self.mut_current_buffer().try_alloc() {
                return BufferElement {
                    buffer: self.current,
                    element: id,
                };
            }
            self.bump_current_buffer();
        }
    }

    pub fn push(&mut self, value: Primitive) -> BufferElement<Primitive> {
        let id = self.alloc();
        self[id.buffer][id.element] = value;
        return id;
    }

    pub fn alloc_range_back(&mut self, count: u16) -> BufferRange<Primitive> {
        assert!(count <= self.buffer_len);
        loop {
            if let Some(range) = self.mut_current_buffer().try_alloc_range(count) {
                return BufferRange {
                    buffer: self.current,
                    range: range,
                }
            }
            self.bump_current_buffer();
        }
    }

    pub fn alloc_back(&mut self) -> BufferElement<Primitive> {
        loop {
            if let Some(id) = self.mut_current_buffer().try_alloc_back() {
                return BufferElement {
                    buffer: self.current,
                    element: id,
                };
            }
            self.bump_current_buffer();
        }
    }

    pub fn push_back(&mut self, value: Primitive) -> BufferElement<Primitive> {
        let id = self.alloc_back();
        self[id.buffer][id.element] = value;
        return id;
    }
}

impl<T> ops::Index<BufferId<T>> for BufferStore<T> {
    type Output = CpuBuffer<T>;
    fn index(&self, id: BufferId<T>) -> &CpuBuffer<T> {
        &self.buffers[id.index()]
    }
}

impl<T> ops::IndexMut<BufferId<T>> for BufferStore<T> {
    fn index_mut(&mut self, id: BufferId<T>) -> &mut CpuBuffer<T> {
        &mut self.buffers[id.index()]
    }
}

impl<T: Copy+Default> ops::Index<BufferRange<T>> for BufferStore<T> {
    type Output = [T];
    fn index(&self, id: BufferRange<T>) -> &[T] {
        &self.buffers[id.buffer.index()][id.range]
    }
}

impl<T: Copy+Default> ops::IndexMut<BufferRange<T>> for BufferStore<T> {
    fn index_mut(&mut self, id: BufferRange<T>) -> &mut [T] {
        &mut self.buffers[id.buffer.index()][id.range]
    }
}

impl<T: Copy+Default> ops::Index<BufferElement<T>> for BufferStore<T> {
    type Output = T;
    fn index(&self, id: BufferElement<T>) -> &T {
        &self.buffers[id.buffer.index()][id.element]
    }
}

impl<T: Copy+Default> ops::IndexMut<BufferElement<T>> for BufferStore<T> {
    fn index_mut(&mut self, id: BufferElement<T>) -> &mut T {
        &mut self.buffers[id.buffer.index()][id.element]
    }
}

pub struct GeometryStore<Vertex> {
    pub buffers: Vec<VertexBuffers<Vertex>>,
}

impl<Vertex: Copy> GeometryStore<Vertex> {
    pub fn new(count: u16) -> Self {
        let mut store = GeometryStore { buffers: Vec::new() };
        for _ in 0..count {
            store.buffers.push(VertexBuffers::new());
        }
        return store;
    }

    pub fn first_buffer(&self) -> BufferId<Vertex> { BufferId::new(0) }
}

impl<Vertex> ops::Index<BufferId<Vertex>> for GeometryStore<Vertex> {
    type Output = VertexBuffers<Vertex>;
    fn index(&self, id: BufferId<Vertex>) -> &VertexBuffers<Vertex> {
        &self.buffers[id.index()]
    }
}

impl<Vertex> ops::IndexMut<BufferId<Vertex>> for GeometryStore<Vertex> {
    fn index_mut(&mut self, id: BufferId<Vertex>) -> &mut VertexBuffers<Vertex> {
        &mut self.buffers[id.index()]
    }
}

