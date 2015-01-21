use range::Range;
use simple_allocator::{AllocatorHelper, BlockId, BlockState};

/// Helps dealing with geometry allocation by tracking allocated blocks in
/// a vertex buffer and an index buffer. Does not actually own the vertex
/// and index data.
pub struct GeomAllocatorHelper {
    vertices: AllocatorHelper,
    indices: AllocatorHelper,
}

#[derive(Copy, Clone, Show, PartialEq)]
pub struct GeomDataId {
    vertices: BlockId,
    indices: BlockId,
}

impl GeomAllocatorHelper {
    pub fn new(vertex_range: Range, index_range: Range) -> GeomAllocatorHelper {
        GeomAllocatorHelper {
            vertices: AllocatorHelper::new(vertex_range, BlockState::Unused),
            indices: AllocatorHelper::new(index_range, BlockState::Unused),
        }
    }

    pub fn can_add(&self, num_vertices: u16, num_indices: u16) -> bool {
        return match (
            self.vertices.find_available_block(num_vertices).is_some(),
            self.indices.find_available_block(num_indices).is_some()
        ) {
            (true, true) => { true }
            _ => { false }
        }
    }

    pub fn add(&mut self, num_vertices: u16, num_indices: u16) -> Option<GeomDataId> {
        match (
            self.vertices.find_available_block(num_vertices),
            self.indices.find_available_block(num_indices)
        ) {
            (Some(vertex_id), Some(index_id)) => {
                return Some(GeomDataId {
                  vertices:
                    if self.vertices.get_block_range(vertex_id).count == num_vertices {
                        self.vertices.set_block_state(vertex_id, BlockState::Used);
                        vertex_id
                    } else {
                        let (id, _) = self.vertices.split(
                            vertex_id, num_vertices,
                            BlockState::Used, BlockState::Unused
                        );
                        id
                    },
                  indices:
                    if self.indices.get_block_range(index_id).count == num_indices {
                        self.indices.set_block_state(index_id, BlockState::Used);
                        index_id
                    } else {
                        let (id, _) = self.indices.split(
                            index_id, num_indices,
                            BlockState::Used, BlockState::Unused
                        );
                        id
                    }
                });
            }
            _ => { return None; }
        }
    }

    pub fn remove(&mut self, id: GeomDataId) {
        // Mark block as unused and try to merge with the adjacent ones.
        self.vertices.set_block_state(id.vertices, BlockState::Unused);
        if let Some(next) = self.vertices.get_next(id.vertices) {
            if self.vertices.get_block_state(next) == BlockState::Unused {
                self.vertices.merge_next(id.vertices, BlockState::Unused);
            }
        }
        if let Some(prev) = self.vertices.get_previous(id.vertices) {
            if self.vertices.get_block_state(prev) == BlockState::Unused {
                self.vertices.merge_next(prev, BlockState::Unused);
            }
        }
        // Same thing for indices
        self.indices.set_block_state(id.indices, BlockState::Unused);
        if let Some(next) = self.indices.get_next(id.indices) {
            if self.indices.get_block_state(next) == BlockState::Unused {
                self.indices.merge_next(id.indices, BlockState::Unused);
            }
        }
        if let Some(prev) = self.indices.get_previous(id.indices) {
            if self.indices.get_block_state(prev) == BlockState::Unused {
                self.indices.merge_next(prev, BlockState::Unused);
            }
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn contains_id(&self, id: GeomDataId) -> bool {
        return self.vertices.contains_block_id(id.vertices)
            && self.indices.contains_block_id(id.indices);
    }

    pub fn get_vertex_range(&self, id: GeomDataId) -> Range {
        return self.vertices.get_block_range(id.vertices);
    }

    pub fn get_index_range(&self, id: GeomDataId) -> Range {
        return self.indices.get_block_range(id.indices);
    }
}

#[test]
fn test_batch_allocator_simple() {
    let mut batch = GeomAllocatorHelper::new(Range::new(1, 1024), Range::new(0, 1024));
    assert!(batch.can_add(32, 10));
    assert!(batch.can_add(18, 20));
    let a = batch.add(32, 10).unwrap();
    let b = batch.add(18, 20).unwrap();
    assert!(a != b);
    assert!(batch.contains_id(a));
    assert!(batch.contains_id(b));
    assert_eq!(batch.get_vertex_range(a), Range::new(1, 32));
    assert_eq!(batch.get_vertex_range(b), Range::new(33, 18));
    assert_eq!(batch.get_index_range(a), Range::new(0, 10));
    assert_eq!(batch.get_index_range(b), Range::new(10, 20));

    // too big to fit in the vbo
    assert!(!batch.can_add(1000, 40));
    assert_eq!(batch.add(1000, 40), None);
    // too big to fit in the ibo
    assert!(!batch.can_add(20, 1000));
    assert_eq!(batch.add(20, 1000), None);

    let c = batch.add(18, 20).unwrap();
    batch.remove(b);
    batch.remove(a);
    assert!(!batch.contains_id(a));
    assert!(!batch.contains_id(b));
    assert!(batch.contains_id(c));

    let d = batch.add(50, 25).unwrap();
    assert!(batch.contains_id(d));

    batch.clear();
    assert!(!batch.contains_id(d));
    assert!(batch.can_add(0,0));

    assert!(batch.can_add(1023, 1024));
    let e = batch.add(1023, 1024).unwrap();
    assert!(batch.contains_id(e));

    assert!(!batch.can_add(1,1));
    assert!(!batch.can_add(0,0));
}
