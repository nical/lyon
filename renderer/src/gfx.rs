pub use gfx_backend as backend;
pub use gfx_hal as hal;

pub use gfx_hal::{
    adapter, buffer, command, device, error, format, image,
    mapping, memory, pass, pool, pso, query, queue, range, window,
    Primitive, Features, Limits, SubmissionResult, SubmissionError,
    SwapchainConfig, SwapImageIndex, PresentMode, MemoryProperties,
    MemoryType, MemoryTypeId,
    memory::Requirements as BufferRequirements,
};

pub use gfx_backend::{Backend, Device, PhysicalDevice, Instance};

pub type Gpu = hal::Gpu<Backend>;
pub type PsoEntryPoint<'l> = pso::EntryPoint<'l, Backend>;
pub type Adapter = hal::Adapter<Backend>;
pub type QueueGroup<Ty> = hal::QueueGroup<Backend, Ty>;
//pub type FrameSync<'l> = hal::FrameSync<'l, Backend>;
//pub type Backbuffer = hal::Backbuffer<Backend>;
pub type Buffer = <Backend as hal::Backend>::Buffer;
pub type BufferView = <Backend as hal::Backend>::BufferView;
pub type UnboundBuffer = <Backend as hal::Backend>::UnboundBuffer;
pub type Memory = <Backend as hal::Backend>::Memory;
pub type Fence = <Backend as hal::Backend>::Fence;
pub type Semaphore = <Backend as hal::Backend>::Semaphore;
pub type RenderPass = <Backend as hal::Backend>::RenderPass;
pub type Swapchain = <Backend as hal::Backend>::Swapchain;
pub type QueryPool = <Backend as hal::Backend>::QueryPool;
pub type CommandPool = <Backend as hal::Backend>::CommandPool;
pub type DescriptorPool = <Backend as hal::Backend>::DescriptorPool;
pub type DescriptorSet = <Backend as hal::Backend>::DescriptorSet;
pub type DescriptorSetLayout = <Backend as hal::Backend>::DescriptorSetLayout;
pub type PipelineLayout = <Backend as hal::Backend>::PipelineLayout;
pub type PipelineCache = <Backend as hal::Backend>::PipelineCache;
pub type GraphicsPipeline = <Backend as hal::Backend>::GraphicsPipeline;
pub type ComputePipeline = <Backend as hal::Backend>::ComputePipeline;
pub type ShaderModule = <Backend as hal::Backend>::ShaderModule;
pub type Framebuffer = <Backend as hal::Backend>::Framebuffer;
pub type Sampler = <Backend as hal::Backend>::Sampler;
pub type Image = <Backend as hal::Backend>::Image;
pub type ImageView = <Backend as hal::Backend>::ImageView;
pub type UnboundImage = <Backend as hal::Backend>::UnboundImage;
pub type Surface = <Backend as hal::Backend>::Surface;

pub mod traits {
    pub use super::hal::Instance as InstanceTrait;
    pub use super::hal::Device as DeviceTrait;
    pub use super::hal::Swapchain as SwapchainTrait;
    pub use super::hal::PhysicalDevice as PhysicalDeviceTrait;
    pub use super::hal::DescriptorPool as DescriptorPoolTrait;
    pub use super::hal::Surface as SurfaceTrait;
}
