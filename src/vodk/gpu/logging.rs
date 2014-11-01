use super::device::*;
use super::constants::*;
use super::objects::*;

pub struct LoggingProxy<Backend> {
    pub backend: Backend,
}

impl<Backend: DeviceBackend> DeviceBackend for LoggingProxy<Backend> {
    fn is_supported(
        &mut self,
        feature: Feature
    ) -> bool {
        println!("device.is_supported({})", feature);
        let result = self.backend.is_supported(feature);
        println!("-> {}", result);
        return result;
    }

    fn set_viewport(
        &mut self,
        x: i32, y: i32,
        w: i32, h: i32
    ) {
        println!("device.set_viewport({}, {}, {}, {})", x, y, w, h);
        self.backend.set_viewport(x, y, w, h);
    }

    fn create_texture(&mut self,
        descriptor: &TextureDescriptor,
    ) -> Result<TextureObject, ResultCode> {
        println!("device.create_texture({})", descriptor);
        let result = self.backend.create_texture(descriptor);
        println!("-> {}", result);
        return result;
    }

    fn destroy_texture(
        &mut self,
        texture: TextureObject
    ) {
        println!("device.destroy_texture({})", texture);
        self.backend.destroy_texture(texture);
    }

    fn set_texture_flags(
        &mut self,
        texture: TextureObject,
        flags: TextureFlags
    ) -> ResultCode {
        println!("device.set_texture_flags({}, {})", texture, flags);
        let result = self.backend.set_texture_flags(texture, flags);
        println!("-> {}", result);
        return result;
    }

    fn create_shader_stage(
        &mut self,
        descriptor: &ShaderStageDescriptor,
    ) -> Result<ShaderStageObject, ResultCode> {
        println!("device.create_shader_stage({})", descriptor);
        let result = self.backend.create_shader_stage(descriptor);
        println!("-> {}", result);
        return result;
    }

    fn get_shader_stage_result(
        &mut self,
        shader: ShaderStageObject,
        result: &mut ShaderBuildResult,
    ) -> ResultCode {
        println!("device.get_shader_stage_result({}, [out])", shader);
        let result = self.backend.get_shader_stage_result(shader, result);
        println!("-> {}", result);
        return result;
    }

    fn destroy_shader_stage(
        &mut self,
        stage: ShaderStageObject
    ) {
        println!("device.destroy_shader_stage({})", stage);
        self.backend.destroy_shader_stage(stage);
    }

    fn create_shader_pipeline(
        &mut self,
        descriptor: &ShaderPipelineDescriptor,
    ) -> Result<ShaderPipelineObject, ResultCode> {
        println!("device.create_shader_pipeline({})", descriptor);
        let result = self.backend.create_shader_pipeline(descriptor);
        println!("-> {}", result);
        return result;
    }

    fn get_shader_pipeline_result(
        &mut self,
        shader: ShaderPipelineObject,
        result: &mut ShaderBuildResult,
    ) -> ResultCode {
        println!("device.get_shader_pipeline_result({}, [out])", shader);
        let result = self.backend.get_shader_pipeline_result(shader, result);
        println!("-> {}", result);
        return result;
    }

    fn destroy_shader_pipeline(
        &mut self,
        shader: ShaderPipelineObject
    ) {
        println!("device.destroy_shader_pipeline({})", shader);
        self.backend.destroy_shader_pipeline(shader);
    }

    fn create_buffer(
        &mut self,
        descriptor: &BufferDescriptor,
    ) -> Result<BufferObject, ResultCode> {
        println!("device.create_buffer({}, [out])", descriptor);
        let result = self.backend.create_buffer(descriptor);
        println!("-> {}", result);
        return result;
    }

    fn destroy_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        self.backend.destroy_buffer(buffer);
    }

    unsafe fn map_buffer(
        &mut self,
        buffer: BufferObject,
        flags: MapFlags,
        data: *mut *mut u8
    ) -> ResultCode {
        println!("device.map_buffer({}, {}, [out])", buffer, flags);
        let result = unsafe {
            self.backend.map_buffer(buffer, flags, data)
        };
        println!("-> {}", result);
        return result;
    }

    fn unmap_buffer(
        &mut self,
        buffer: BufferObject
    ) {
        println!("device.unmap_buffer({})", buffer);
        self.backend.unmap_buffer(buffer);
    }

    fn destroy_geometry(
        &mut self,
        geom: GeometryObject
    ) {
        println!("device.destroy_geometry({})", geom);
        self.backend.destroy_geometry(geom);
    }

    fn create_geometry(
        &mut self,
        descriptor: &GeometryDescriptor,
    ) -> Result<GeometryObject, ResultCode> {
        println!("device.create_geometry({})", descriptor);
        let result = self.backend.create_geometry(descriptor);
        println!("-> {}", result);
        return result;
    }

    fn get_vertex_attribute_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> VertexAttributeLocation {
        println!("get_vertex_attribute_location({}, {})", shader, name);
        let result = self.backend.get_vertex_attribute_location(shader, name);
        println!("-> {}", result);
        return result;
    }

    fn create_render_target(
        &mut self,
        descriptor: &RenderTargetDescriptor,
    ) -> Result<RenderTargetObject, ResultCode> {
        println!("device.create_render_target({})", descriptor);
        let result = self.backend.create_render_target(descriptor);
        println!("-> {}", result);
        return result;
    }

    fn destroy_render_target(
        &mut self,
        target: RenderTargetObject
    ) {
        println!("device.destroy_render_target({})", target);
        self.backend.destroy_render_target(target);
    }

    fn get_default_render_target(&mut self) -> RenderTargetObject {
        println!("device.get_default_render_target()");
        let result = self.backend.get_default_render_target();
        println!("-> {}", result);
        return result;
    }

    fn copy_buffer_to_texture(
        &mut self,
        buffer: BufferObject,
        texture: TextureObject
    ) -> ResultCode {
        println!("device.copy_buffer_to_texture({}, {})", buffer, texture);
        let result = self.backend.copy_buffer_to_texture(buffer, texture);
        println!("-> {}", result);
        return result;
    }

    fn copy_texture_to_buffer(
        &mut self,
        texture: TextureObject,
        buffer: BufferObject
    ) -> ResultCode {
        println!("device.copy_texture_to_buffer({}, {})", texture, buffer);
        let result = self.backend.copy_texture_to_buffer(texture, buffer);
        println!("-> {}", result);
        return result;
    }

    fn copy_buffer_to_buffer(
        &mut self,
        src_buffer: BufferObject,
        dest_buffer: BufferObject,
        src_offset: u16,
        dest_offset: u16,
        size: u16
    ) -> ResultCode {
        println!(
            "device.copy_buffer_to_buffer({}, {}, {}, {}, {})",
            src_buffer, dest_buffer, src_offset, dest_offset, size
        );
        let result = self.backend.copy_buffer_to_buffer(
            src_buffer, dest_buffer, src_offset, dest_offset, size
        );
        println!("-> {}", result);
        return result;
    }

    fn bind_uniform_buffer(
        &mut self,
        binding_index: UniformBindingIndex,
        ubo: BufferObject,
        range: Option<(u16, u16)>
    ) -> ResultCode {
        println!(
            "device.bind_uniform_buffer({}, {}, {})",
            binding_index, ubo, range
        );
        let result = self.backend.bind_uniform_buffer(
            binding_index,
            ubo,
            range
        );
        println!("-> {}", result);
        return result;
    }

    fn set_uniform_block(
        &mut self,
        shader: ShaderPipelineObject,
        block_index: UniformBlockLocation,
        binding_index: UniformBindingIndex,
    ) -> ResultCode {
        println!(
            "device.set_uniform_block({}, {}, {})",
            shader, block_index, binding_index
        );
        let result = self.backend.set_uniform_block(
            shader,
            block_index,
            binding_index
        );
        println!("-> {}", result);
        return result;
    }
    
    fn get_uniform_block_location(
        &mut self,
        shader: ShaderPipelineObject,
        name: &str
    ) -> UniformBlockLocation {
        println!(
            "device.get_uniform_block_location({}, {})",
            shader, name
        );
        let result = self.backend.get_uniform_block_location(
            shader,
            name
        );
        println!("-> {}", result);
        return result;
    }

    fn set_shader(
        &mut self,
        pipeline: ShaderPipelineObject
    ) -> ResultCode {
        println!("device.set_shader({})", pipeline);
        let result = self.backend.set_shader(pipeline);
        println!("-> {}", result);
        return result;
    }

    fn draw(&mut self,
        geom: GeometryObject,
        range: Range,
        flags: GeometryFlags,
        blend: BlendMode,
        targets: TargetTypes
    ) -> ResultCode {
        println!(
            "device.draw({}, {}, {}, {}, {})",
            geom, range, flags, blend, targets
        );
        let result = self.backend.draw(geom, range, flags, blend, targets);
        println!("-> {}", result);
        return result;
    }

    fn flush(&mut self) -> ResultCode {
        println!("device.flush()");
        let result = self.backend.flush();
        println!("-> {}", result);
        return result;
    }

    fn clear(&mut self, targets: TargetTypes) -> ResultCode {
        println!("device.clear({})", targets);
        let result = self.backend.clear(targets);
        println!("-> {}", result);
        return result;
    }

    fn set_clear_color(&mut self, r:f32, g: f32, b: f32, a: f32) {
        println!("device.set_clear_color({}, {}, {}, {}) -> ()", r, g, b, a);
        self.backend.set_clear_color(r, g, b, a);
    }
}
