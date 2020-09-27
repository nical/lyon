#ifndef IMAGE_SOURCE_GLSL
#define IMAGE_SOURCE_GLSL


#define NO_IMAGE_SOURCE 0

struct ImageSource {
    Rect rect;
    vec4 parameters;
};

layout(std140, set = COMMON_SET, binding = IMAGE_SOURCES) buffer u_image_sources { ImageSource image_sources[]; };

#endif
