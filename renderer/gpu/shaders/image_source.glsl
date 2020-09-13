#ifndef IMAGE_SOURCE_GLSL
#define IMAGE_SOURCE_GLSL


#define NO_IMAGE_SOURCE 0

struct ImageSource {
    Rect rect;
    vec4 parameters;
};

layout(std140, binding = IMAGE_SOURCES) uniform u_image_sources { ImageSource image_sources[512]; };

#endif
