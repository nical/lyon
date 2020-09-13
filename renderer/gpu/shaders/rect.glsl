#ifndef RECT_GLSL
#define RECT_GLSL

#define Rect vec4

vec2 rect_size(Rect r) {
    return r.zw - r.xy;
}

vec2 rect_min(Rect r) { return r.xy; }

vec2 rect_max(Rect r) { return r.zw; }

vec2 rect_sample(Rect rect, vec2 v) { return mix(rect.xy, rect.zw, v); }

vec2 rect_clamp(Rect rect, vec2 v) {
    v = max(v, rect.xy);
    v = min(v, rect.zw);
    return v;
}

Rect rect_inflate(Rect rect, float by) {
    rect.xy -= vec2(by, by);
    rect.zw += vec2(by, by);
    return rect;
}

// std140 struct size is rounded to a multiple of 16 bytes
//struct PackedIntRect {
//    uint min;
//    uint max;
//};

vec2 unpack_int_vec2(uint packed) {
    return vec2(
        float(packed & 0x0000FFFFu),
        float(packed >> 16)
    );
}

Rect unpack_int_rect(uint min, uint max) {
    return Rect(
        unpack_int_vec2(min),
        unpack_int_vec2(max)
    );
}

#endif
