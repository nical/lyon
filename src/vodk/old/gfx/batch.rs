
pub struct BatchingContext {
    pub vertex_cursor: usize, // in vertices
    pub index_cursor: usize,  // in indices
    pub base_vertex: u16,
}

impl<T: VertexType> BatchingContext<VertexType> {
    pub fn new(base_vertex: u16) -> BatchingContext<VertexType> {
        BatchingContext {
            vertex_cursor: 0,
            index_cursor: 0,
            base_vertex: base_vertex,
        }
    }

    pub fn push_vertex<VertexType>(&self, v: &VertexType, vbo: &mut[VertexType]) -> u16 {
        vbo[self.vertex_cursor] = *v;
        self.vertex_cursor += 1;
        return (self.vertex_cursor - 1) as u16;
    }
    pub fn push_index(&self, idx: u16, ibo: &mut[u16]) {
        ibo[self.index_cursor] = idx + self.base_vertex;
        self.index_cursor += 1;
    }

    /// Returns the offset of a vertex (with base_vertex taken into account) from
    /// the offset of an index.
    pub fn lookup_vertex(&self, ibo: &[u16], offset: usize) -> usize {
        return (ibo[offset] - base_vertex) as usize;
    }
}

