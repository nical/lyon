use range::Range;
use simple_allocator::{AllocatorHelper, BlockId, BlockState};

/// Helps dealing with geometry allocation by tracking allocated blocks in
/// a vertex buffer and an index buffer. Does not actually own the vertex
/// and index data.
pub struct GeomAllocatorHelper {
    vertices: AllocatorHelper,
    indices: AllocatorHelper,
}

#[deriving(Clone, Show, PartialEq)]
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

    pub fn add(&mut self, num_vertices: u16, num_indices: u16) -> Option<GeomDataId> {
        match (
            self.vertices.find_available_block(num_vertices),
            self.indices.find_available_block(num_indices)
        ) {
            (Some(vertex_id), Some(index_id)) => {
                return Some(GeomDataId {
                  vertices:
                    if self.vertices.get_range(vertex_id).count == num_vertices {
                        self.vertices.set_state(vertex_id, BlockState::Used);
                        vertex_id
                    } else {
                        let (id, _) = self.vertices.split(
                            vertex_id, num_vertices,
                            BlockState::Used, BlockState::Unused
                        );
                        id
                    },
                  indices:
                    if self.indices.get_range(index_id).count == num_indices {
                        self.indices.set_state(index_id, BlockState::Used);
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
        self.vertices.set_state(id.vertices, BlockState::Unused);
        if let Some(next) = self.vertices.get_next(id.vertices) {
            if self.vertices.get_state(next) == BlockState::Unused {
                self.vertices.merge_next(id.vertices, BlockState::Unused);
            }
        }
        if let Some(prev) = self.vertices.get_previous(id.vertices) {
            if self.vertices.get_state(prev) == BlockState::Unused {
                self.vertices.merge_next(prev, BlockState::Unused);
            }
        }
        // Same thing for indices
        self.indices.set_state(id.indices, BlockState::Unused);
        if let Some(next) = self.indices.get_next(id.indices) {
            if self.indices.get_state(next) == BlockState::Unused {
                self.indices.merge_next(id.indices, BlockState::Unused);
            }
        }
        if let Some(prev) = self.indices.get_previous(id.indices) {
            if self.indices.get_state(prev) == BlockState::Unused {
                self.indices.merge_next(prev, BlockState::Unused);
            }
        }
    }
}

#[test]
fn test_batch_allocator_simple() {
    let mut batch = GeomAllocatorHelper::new(Range::new(1, 1024), Range::new(0, 1024));
    let a = batch.add(32, 10).unwrap();
    let b = batch.add(18, 20).unwrap();
    assert!(a != b);
    // too big to fit in the vbo
    assert_eq!(batch.add(1000, 40), None);
    // too big to fit in the ibo
    assert_eq!(batch.add(20, 1000), None);
    let c = batch.add(18, 20).unwrap();
    batch.remove(b);
    batch.remove(a);
    let d = batch.add(50, 25).unwrap();
}
