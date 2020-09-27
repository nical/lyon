#ifndef INSTANCE_GLSL
#define INSTANCE_GLSL


struct Instance {
    uint rect_id;
    uint transform_id;
    uint primitive_id;
    uint src_color_id;
    uint src_mask_id;
    uint user_data;
    float z;
};

layout(location = A_INSTANCE) in uvec4 a_instance;

Instance unpack_instance() {
    Instance instance;
    instance.rect_id = a_instance[0] >> 16;
    instance.transform_id = a_instance[0] & 0x0000ffffu;
    instance.primitive_id = a_instance[1] >> 16;
    instance.src_color_id = a_instance[1] & 0x0000ffffu;
    instance.src_mask_id = a_instance[2] >> 16;
    instance.user_data = a_instance[2] & 0x0000ffffu;
    instance.z = float(a_instance[3]) / 16384.0;

    return instance;
}

#endif
