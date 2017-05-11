use gfx_device_gl;
use gfx;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub type CmdEncoder = gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>;
pub type BufferObject<T> = gfx::handle::Buffer<gfx_device_gl::Resources, T>;
pub type Vbo<T> = gfx::handle::Buffer<gfx_device_gl::Resources, T>;
pub type Ibo = gfx::IndexBuffer<gfx_device_gl::Resources>;
pub type Pso<T> = gfx::PipelineState<gfx_device_gl::Resources, T>;
pub type IndexSlice = gfx::Slice<gfx_device_gl::Resources>;
pub type ColorTarget = gfx::handle::RenderTargetView<gfx_device_gl::Resources,
                                                     (gfx::format::R8_G8_B8_A8,
                                                      gfx::format::Unorm)>;
pub type DepthTarget = gfx::handle::DepthStencilView<gfx_device_gl::Resources,
                                                     (gfx::format::D24_S8, gfx::format::Unorm)>;
pub type GlDevice = gfx_device_gl::Device;
pub type GlFactory = gfx_device_gl::Factory;
