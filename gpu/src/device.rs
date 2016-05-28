use super::objects::*;
use super::constants::*;
use vodk_data as data;

use std::mem;

pub type AttributeType = data::Type;
pub type UniformBindingIndex = i32;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct UniformBlockLocation { pub index: i16 }
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct VertexAttributeLocation { pub index: i16 }

#[derive(Copy, Clone, Debug)]
pub enum Range {
    VertexRange(u16, u16),
    IndexRange(u16, u16),
}

#[derive(Debug)]
pub struct TextureDescriptor {
    pub format: PixelFormat,
    pub width: u16,
    pub height: u16,
    pub mip_levels: u16,
    pub flags: TextureFlags,
}

#[derive(Copy, Clone, Debug)]
pub struct BufferDescriptor {
    pub size: u32,
    pub update_hint: UpdateHint,
    pub buffer_type: BufferType,
}

#[derive(Debug)]
pub struct GeometryDescriptor<'l> {
    pub attributes: &'l[VertexAttribute],
    pub index_buffer: Option<BufferObject>,
}

#[derive(Debug)]
pub struct ShaderStageDescriptor<'l> {
    pub stage_type: ShaderType,
    pub src: &'l[&'l str],
}

#[derive(Debug)]
pub struct ShaderPipelineDescriptor<'l> {
    pub stages: &'l[ShaderStageObject],
    pub attrib_locations: &'l[(&'l str, VertexAttributeLocation)],
}

#[derive(Debug)]
pub struct RenderTargetDescriptor<'l> {
    pub color_attachments: &'l[TextureObject],
    pub depth: Option<TextureObject>,
    pub stencil: Option<TextureObject>
}

#[derive(Copy, Clone, Debug)]
pub struct VertexAttribute {
    pub buffer: BufferObject,
    pub attrib_type: AttributeType,
    pub location: VertexAttributeLocation,
    pub stride: u16,
    pub offset: u16,
    pub normalize: bool,
}


pub struct Device<DeviceBackend> {
    pub backend: DeviceBackend,
}

impl<Backend: DeviceBackend> Device<Backend> {
    pub fn is_supported(
        &mut self,
        feature: Feature
    ) -> bool {
        return self.backend.is_supported(feature);
    }

    pub fn set_viewport(&mut self, x:i32, y:i32, w:i32, h:i32) {
        self.backend.set_viewport(x, y, w, h);
    }

    pub fn create_texture(&mut self,
        descriptor: &TextureDescriptor,
    ) -> Result<TextureObject, ResultCode> {
        return self.backend.create_texture(descriptor);
    }

    pub fn destroy_texture(
        &mut self,
        texture: TextureObject
    ) {
        self.backend.destroy_texture(texture);
    }

    pub fn set_texture_flags(
        &mut self,
        texture: TextureObject,
        flags: TextureFlags
    ) -> ResultCode {
        return self.backend.set_texture_flags(texture, flags);
    }

    pub fn create_shader_stage(
        &mut self,
        descriptor: &ShaderStageDescriptor,
    ) -> Result<ShaderStageObject, ResultCode> {
        return self.backend.create_shader_stage(descriptor);
    }

    pub fn get_shader_stage_result(
        &mut self,
        shader: ShaderStageObject,
    ) -> Result<(), (ResultCode, String)> {
        return self.backend.get_shader_stage_result(shader);
    }

    pub fn destroy_shader_stage(
        &mut self,
        stage: ShaderStageObject
    ) {
        self.backend.destroy_shader_stage(stage);
    }

    pub fn create_shader_pipeline(
        &mut self,
        descriptor: &ShaderPipelineDescriptor,
    ) -> Result<ShaderPipelineObject, ResultCode> {
        return self.backend.create_shader_pipeline(descriptor);
    }

    pub fn get_shader_pipeline_result(
        &mut self,
        shader: ShaderPipelineObject,
    ) -> Result<(), (ResultCode, String)> {
        return self.backend.get_shader_pipeline_result(shader);
    }

    pub fn destroy_shader_pipeline(
        &mut self,
        shader: ShaderPipelineObject
    ) {
        self.backend.destroy_shader_pipeline(shader);
    }

    pub fn create_buffer(
        &mut self,
        descriptor: &BufferDescriptor,
    ) -> Result<BufferObject, ResultCode> {
        return self.backend.create_buffer(descriptor);
    }

    pub fn destroy_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        self.backend.destroy_buffer(buffer);
    }

    pub unsafe fn map_buffer<T>(
        &mut self,
        buffer: BufferObject,
        flags: MapFlags,
        data: &mut &mut[T]
    ) -> ResultCode {
        unsafe {
            let mut ptr = 0 as *mut u8;
            let result = self.backend.map_buffer(buffer, flags, &mut ptr);
            if result != ResultCode::Ok {
                return result;
            }
            if ptr == 0 as *mut u8 {
                return ResultCode::UnknownError;
            }
            *data = mem::transmute((
                ptr,
                buffer.size as usize / mem::size_of::<T>()
            ));
        }
        return ResultCode::Ok;
    }

    pub fn unmap_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        self.backend.unmap_buffer(buffer);
    }

    pub fn with_mapped_buffer<T>(
        &mut self,
        buffer: BufferObject,
        cb: &Fn(&mut[T])
    ) -> ResultCode {
        let mut mapped_data: &mut[T] = &mut[];
        unsafe {
            let result = self.map_buffer(
                buffer,
                READ_MAP|WRITE_MAP,
                &mut mapped_data
            );
            if result != ResultCode::Ok { return result; }
        }

        cb(mapped_data);

        self.unmap_buffer(buffer);
        return ResultCode::Ok;
    }

    pub fn with_read_only_mapped_buffer<T>(
        &mut self,
        buffer: BufferObject,
        cb: &Fn(&[T])
    ) -> ResultCode {
        let mut mapped_data: &mut[T] = &mut[];
        unsafe {
            let result = self.map_buffer(
                buffer,
                READ_MAP,
                &mut mapped_data
            );
            if result != ResultCode::Ok { return result; }
        }

        cb(mapped_data);

        self.unmap_buffer(buffer);
        return ResultCode::Ok;
    }

    pub fn with_write_only_mapped_buffer<T>(
        &mut self,
        buffer: BufferObject,
        cb: &Fn(&mut[T])
    ) -> ResultCode {
        let mut mapped_data: &mut[T] = &mut[];
        unsafe {
            let result = self.map_buffer(
                buffer,
                WRITE_MAP,
                &mut mapped_data
            );
            if result != ResultCode::Ok { return result; }
        }

        cb(mapped_data);

        self.unmap_buffer(buffer);
        return ResultCode::Ok;
    }

    pub fn destroy_geometry(
        &mut self,
        geom: GeometryObject
    ) {
        self.backend.destroy_geometry(geom);
    }

    pub fn create_geometry(
        &mut self,
        descriptor: &GeometryDescriptor,
    ) -> Result<GeometryObject, ResultCode> {
        return self.backend.create_geometry(descriptor);
    }

    pub fn get_vertex_attribute_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> VertexAttributeLocation {
        return self.backend.get_vertex_attribute_location(shader, name);
    }

    pub fn create_render_target(
        &mut self,
        descriptor: &RenderTargetDescriptor,
    ) -> Result<RenderTargetObject, ResultCode> {
        return self.backend.create_render_target(descriptor);
    }

    pub fn destroy_render_target(
        &mut self,
        target: RenderTargetObject
    ) {
        self.backend.destroy_render_target(target);
    }

    pub fn get_default_render_target(&mut self) -> RenderTargetObject {
        return self.backend.get_default_render_target();
    }

    pub fn copy_buffer_to_texture(
        &mut self,
        buffer: BufferObject,
        texture: TextureObject
    ) -> ResultCode {
        return self.backend.copy_buffer_to_texture(buffer, texture);
    }

    pub fn copy_texture_to_buffer(
        &mut self,
        texture: TextureObject,
        buffer: BufferObject
    ) -> ResultCode {
        return self.backend.copy_texture_to_buffer(texture, buffer);
    }

    pub fn set_shader(
        &mut self,
        pipeline: ShaderPipelineObject
    ) -> ResultCode {
        return self.backend.set_shader(pipeline);
    }

    pub fn bind_uniform_buffer(
        &mut self,
        binding_index: UniformBindingIndex,
        ubo: BufferObject,
        range: Option<(u16, u16)>
    ) -> ResultCode {
        return self.backend.bind_uniform_buffer(
            binding_index,
            ubo,
            range
        );
    }

    pub fn set_uniform_block(
        &mut self,
        shader: ShaderPipelineObject,
        block_index: UniformBlockLocation,
        binding_index: UniformBindingIndex,
    ) -> ResultCode {
        return self.backend.set_uniform_block(
            shader,
            block_index,
            binding_index
        );
    }

    pub fn get_uniform_block_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> UniformBlockLocation {
        return self.backend.get_uniform_block_location(shader, name);
    }

    pub fn draw(&mut self,
        geom: GeometryObject,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> ResultCode {
        return self.backend.draw(geom, range, flags, blend, targets);
    }

    pub fn flush(&mut self) -> ResultCode {
        return self.backend.flush();
    }

    pub fn clear(&mut self, targets: TargetTypes) -> ResultCode {
        return self.backend.clear(targets);
    }

    pub fn set_clear_color(&mut self, r:f32, g: f32, b: f32, a: f32) {
        self.backend.set_clear_color(r, g, b, a);
    }
}


pub trait DeviceBackend {
    fn is_supported(
        &mut self,
        feature: Feature
    ) -> bool;

    fn set_viewport(
        &mut self,
        x:i32, y:i32,
        w:i32, h:i32
    );

    fn create_texture(
        &mut self,
        descriptor: &TextureDescriptor,
    ) -> Result<TextureObject, ResultCode>;

    fn destroy_texture(
        &mut self,
        tex: TextureObject
    );

    fn set_texture_flags(
        &mut self,
        tex: TextureObject,
        flags: TextureFlags
    ) -> ResultCode;

    fn create_shader_stage(
        &mut self,
        descriptor: &ShaderStageDescriptor,
    ) -> Result<ShaderStageObject, ResultCode>;

    fn get_shader_stage_result(
        &mut self,
        shader: ShaderStageObject,
    ) -> Result<(), (ResultCode, String)>;

    fn destroy_shader_stage(
        &mut self,
        stage: ShaderStageObject
    );

    fn create_shader_pipeline(
        &mut self,
        descriptor: &ShaderPipelineDescriptor,
    ) -> Result<ShaderPipelineObject, ResultCode>;

    fn get_shader_pipeline_result(
        &mut self,
        shader: ShaderPipelineObject,
    ) -> Result<(), (ResultCode, String)>;

    fn destroy_shader_pipeline(
        &mut self,
        shader: ShaderPipelineObject
    );

    fn create_buffer(
        &mut self,
        descriptor: &BufferDescriptor,
    ) -> Result<BufferObject, ResultCode>;

    fn destroy_buffer(
        &mut self,
        buffer: BufferObject
    );

    unsafe fn map_buffer(
        &mut self,
        buffer: BufferObject,
        flags: MapFlags,
        data: *mut *mut u8
    ) -> ResultCode;

    fn unmap_buffer(
        &mut self,
        buffer: BufferObject
    );

    fn destroy_geometry(
        &mut self,
        geom: GeometryObject
    );

    fn create_geometry(
        &mut self,
        descriptor: &GeometryDescriptor,
    ) -> Result<GeometryObject, ResultCode>;

    fn get_vertex_attribute_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> VertexAttributeLocation;

    fn create_render_target(
        &mut self,
        descriptor: &RenderTargetDescriptor,
    ) -> Result<RenderTargetObject, ResultCode>;

    fn destroy_render_target(
        &mut self,
        target: RenderTargetObject
    );

    fn get_default_render_target(&mut self) -> RenderTargetObject;

    fn copy_buffer_to_texture(
        &mut self,
        buffer: BufferObject,
        texture: TextureObject
    ) -> ResultCode;

    fn copy_texture_to_buffer(
        &mut self,
        texture: TextureObject,
        buffer: BufferObject
    ) -> ResultCode;

    fn copy_buffer_to_buffer(
        &mut self,
        src_buffer: BufferObject,
        dest_buffer: BufferObject,
        src_offset: u16,
        dest_offset: u16,
        size: u16
    ) -> ResultCode;

    fn set_shader(
        &mut self,
        pipeline: ShaderPipelineObject
    ) -> ResultCode;

    fn bind_uniform_buffer(
        &mut self,
        binding_index: UniformBindingIndex,
        ubo: BufferObject,
        range: Option<(u16, u16)>
    ) -> ResultCode;

    fn set_uniform_block(
        &mut self,
        shader: ShaderPipelineObject,
        block_index: UniformBlockLocation,
        binding_index: UniformBindingIndex,
    ) -> ResultCode;

    fn get_uniform_block_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> UniformBlockLocation;

    fn draw(&mut self,
        geom: GeometryObject,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> ResultCode;

    fn flush(&mut self) -> ResultCode;

    fn clear(&mut self, targets: TargetTypes) -> ResultCode;

    fn set_clear_color(&mut self, r:f32, g: f32, b: f32, a: f32);
}
