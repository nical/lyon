#ifndef BINDINGS_GLSL
#define BINDINGS_GLSL

// Constants must match the ones in bindings.rs.

// Uniform buffers
#define GLOBALS 0
#define TRANSFORMS 1
#define PRIMITIVE_RECTS 2
#define PRIMITIVE_DATA 3
#define IMAGE_SOURCES 4

#define F32_DATA 6
#define U32_DATA 7
#define TILING_INFO 2

// Textures.
#define INPUT_COLOR_0 10
#define INPUT_COLOR_1 11
#define U8_MASK 12
#define FLOAT_MASK 13
#define DEFAULT_SAMPLER 14

// Descriptor sets
#define COMMON_SET 0
#define INPUT_SAMPLERS_SET 1
#define SPECIFIC_SET 2

// Vertex attributes
#define A_INSTANCE 0
#define A_POSITION 1

// Varyings
#define V_IMAGE_UV 0
#define V_IMAGE_UV_RECT 1
#define V_MASK_UV 2
#define V_MASK_UV_RECT 3

#endif
