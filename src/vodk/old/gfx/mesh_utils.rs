
fn index(x:u32, y:u32, w:u32) -> u32 { x + y * w }

pub enum GridType {
    NO_INDICES,
    PER_QUAD_INDICES, // indices are reused within quads
    PER_GRID_INDICES, // indices are reused as much as possible
}

pub fn generate_grid_indices(grid_w: u32, grid_h: u32, grid: GridType,
                             indices: &mut [u32],
                             vertex_offset: u32, vertex_stride: u32) {
    let mut idx = 0;
    for j in range(0, grid_h - 1) {
        for i in range(0, grid_w - 1) {
            match grid {
                PER_GRID_INDICES => {
                    indices[idx]   = vertex_offset + vertex_stride * index(i,   j,   grid_w); // A - B
                    indices[idx+1] = vertex_offset + vertex_stride * index(i+1, j,   grid_w); // | /
                    indices[idx+2] = vertex_offset + vertex_stride * index(i,   j+1, grid_w); // C

                    indices[idx+3] = vertex_offset + vertex_stride * index(i,   j+1, grid_w); //     B
                    indices[idx+4] = vertex_offset + vertex_stride * index(i+1, j,   grid_w); //   / |
                    indices[idx+5] = vertex_offset + vertex_stride * index(i+1, j+1, grid_w); // C - D
                    idx += 6;
                }
                PER_QUAD_INDICES => {
                    let current_quad = vertex_offset + vertex_stride * ( i * 4 + j * grid_w * 4);
                    indices[idx]   = current_quad;                     // A - B
                    indices[idx+1] = current_quad + vertex_stride;     // | /
                    indices[idx+2] = current_quad + vertex_stride * 2; // C

                    indices[idx+3] = current_quad + vertex_stride * 2; //     B
                    indices[idx+4] = current_quad + vertex_stride;     //   / |
                    indices[idx+5] = current_quad + vertex_stride * 3; // C - D
                    idx += 6;
                }
                _ => { fail!("Unupported grid type"); }
            }
        }
    }
}

pub fn generate_grid_vertices(grid_w: u32, grid_h: u32,
                              vertices: &mut [f32], vertex_stride: usize,
                              use_indices: bool,
                              vertex_cb: |x: u32, y:u32, vertex_slice: &mut[f32]|) {
    let mut v = 0;
    for j in range(0, grid_h) {
        for i in range(0, grid_w) {
            if use_indices {
                vertex_cb(i, j, vertices.mut_slice(v, v + vertex_stride));
                v += vertex_stride;
            } else {
                vertex_cb(i, j, vertices.mut_slice(v, v + vertex_stride));
                v += vertex_stride;
            }
        }
    }
}

pub fn num_indices_for_grid(w:u32, h:u32, grid: GridType) -> u32 {
    return (w-1)*(h-1)*6;
}

pub fn num_vertices_for_grid(w:u32, h:u32, grid: GridType) -> u32 {
    return match grid {
        NO_INDICES => { (w-1)*(h-1)*6 }
        PER_QUAD_INDICES => { (w-1)*(h-1)*4 }
        PER_GRID_INDICES => { w*h }
    }
}