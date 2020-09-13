// Constants must match the ones in bindings.glsl.

// Uniform buffers
pub const GLOBALS: u32 = 0;
pub const TRANSFORMS: u32 = 1;
pub const PRIMITIVE_RECTS: u32 = 2;
pub const PRIMITIVE_DATA: u32 = 3;
pub const IMAGE_SOURCES: u32 = 4;

pub const F32_DATA: u32 = 5;
pub const U32_DATA: u32 = 6;
pub const TILING_INFO: u32 = 2;

// Textures.
pub const INPUT_COLOR_0: u32 = 10;
pub const INPUT_COLOR_1: u32 = 11;
pub const U8_MASK: u32 = 12;
pub const FLOAT_MASK: u32 = 13;
pub const DEFAULT_SAMPLER: u32 = 14;

// Descriptor sets
pub const COMMON_SET: u32 = 0;
pub const INPUT_SAMPLERS_SET: u32 = 1;
pub const SPECIFIC_SET: u32 = 2;

pub const A_INSTANCE: u32 = 0;
pub const A_POSITION: u32 = 1;
