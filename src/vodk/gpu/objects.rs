use super::constants::{BufferType};
use std::fmt::Show;

#[derive(Show, Copy, Clone, PartialEq)]
pub struct SyncObject { pub handle: u32 }
#[derive(Show, Copy, Clone, PartialEq)]
pub struct BufferObject {
    pub handle: u32,
    pub size: u32,
    pub buffer_type: BufferType
}
#[derive(Show, Copy, Clone, PartialEq)]
pub struct TextureObject { pub handle: u32 }
#[derive(Show, Copy, Clone, PartialEq)]
pub struct GeometryObject { pub handle: u32 }
#[derive(Show, Copy, Clone, PartialEq)]
pub struct ShaderStageObject { pub handle: u32 }
#[derive(Show, Copy, Clone, PartialEq)]
pub struct ShaderPipelineObject { pub handle: u32 }
#[derive(Show, Copy, Clone, PartialEq)]
pub struct RenderTargetObject { pub handle: u32 }

impl SyncObject { pub fn new() -> SyncObject { SyncObject { handle: 0 } } }
impl TextureObject { pub fn new() -> TextureObject { TextureObject { handle: 0 } } }
impl GeometryObject { pub fn new() -> GeometryObject { GeometryObject { handle: 0 } } }
impl ShaderStageObject { pub fn new() -> ShaderStageObject { ShaderStageObject { handle: 0 } } }
impl ShaderPipelineObject { pub fn new() -> ShaderPipelineObject { ShaderPipelineObject { handle: 0 } } }
impl BufferObject {
    pub fn new() -> BufferObject {
        BufferObject { handle: 0, size: 0, buffer_type: BufferType::VERTEX }
    }
}

