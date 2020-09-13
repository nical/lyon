
vec4 unpack_color(uint color) {
    uint mask = 0x000000FFu;
    return vec4(
        float((color >> 24) & mask),
        float((color >> 16) & mask),
        float((color >>  8) & mask),
        float(color & mask)
    ) / 255.0;    
}

vec4 premultiply_alpha(vec4 color) {
    color.r *= color.a;
    color.g *= color.a;
    color.b *= color.a;

    return color;
}

vec4 unpremultiply_alpha(vec4 color) {
    color.r /= color.a;
    color.g /= color.a;
    color.b /= color.a;

    return color;
}
