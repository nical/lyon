#[cfg(not(any(feature = "dx12", feature = "metal", feature = "gl")))]
pub extern crate gfx_backend_vulkan as gfx_backend;
#[cfg(feature = "dx12")] pub extern crate gfx_backend_dx12 as gfx_backend;
#[cfg(feature = "gl")] pub extern crate gfx_backend_gl as gfx_backend;
#[cfg(feature = "metal")] pub extern crate gfx_backend_metal as gfx_backend;
pub extern crate gfx_hal;

pub mod gfx;
pub mod writer;
pub mod allocator;

use std::marker::PhantomData;
use std::ops::Range;

pub unsafe trait GpuData : Copy {}

pub type GpuBlock = [u32; 4];
pub type Index = u16;

unsafe impl GpuData for GpuBlock {}
unsafe impl GpuData for [u32; 8] {}
unsafe impl GpuData for [u32; 16] {}
unsafe impl GpuData for [u32; 32] {}
unsafe impl GpuData for [f32; 4] {}
unsafe impl GpuData for [f32; 8] {}
unsafe impl GpuData for [f32; 16] {}
unsafe impl GpuData for [f32; 32] {}
unsafe impl GpuData for [i32; 4] {}
unsafe impl GpuData for [i32; 8] {}
unsafe impl GpuData for [i32; 16] {}
unsafe impl GpuData for [i32; 32] {}

#[derive(Debug, PartialEq, Hash)]
pub struct GpuBuffer<T> {
    handle: u64,
    _marker: PhantomData<T>,
}

impl<T> Copy for GpuBuffer<T> {}
impl<T> Clone for GpuBuffer<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> GpuBuffer<T> {
    pub fn new(handle: u64) -> Self {
        GpuBuffer { handle, _marker: PhantomData }
    }

    pub fn as_gpu_blocks(&self) -> GpuBuffer<GpuBlock> {
        unsafe { self.cast() }
    }

    pub unsafe fn cast<T2: GpuData>(&self) -> GpuBuffer<T2> {
        GpuBuffer::new(self.handle)
    }

    pub fn range(&self, range: Range<u32>) -> GpuBufferRange<T> {
        GpuBufferRange { buffer: *self, range }
    }

    pub fn offset(&self, offset: u32) -> GpuBufferOffset<T> {
        GpuBufferOffset { buffer: *self, offset }
    }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct GpuBufferRange<T> {
    pub buffer: GpuBuffer<T>,
    pub range: Range<u32>,
}

impl<T> GpuBufferRange<T> {
    pub fn as_gpu_blocks(&self) -> GpuBufferRange<GpuBlock> {
        unsafe { self.cast() }
    }

    pub unsafe fn cast<T2: GpuData>(&self) -> GpuBufferRange<T2> {
        GpuBufferRange {
            buffer: self.buffer.cast(),
            range: self.range.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct GpuBufferOffset<T> {
    pub buffer: GpuBuffer<T>,
    pub offset: u32,
}

pub struct GpuMesh {
    pub vertices: GpuBufferRange<Vertex>,
    pub indices: GpuBufferRange<Index>,
}

pub struct Vertex;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CopyParams {
    pub row_size: u32,
    pub n_rows: u32,
    pub src_row_pitch: u32,
    pub dst_row_pitch: u32,
}

impl CopyParams {
    pub fn buffer(size: u32) -> Self {
        CopyParams {
            row_size: size,
            n_rows: 1,
            src_row_pitch: size,
            dst_row_pitch: size,
        }
    }

    pub fn image(width: u32, height: u32, bpp: u32) -> Self {
        let row_size = width * bpp;
        CopyParams {
            row_size,
            n_rows: height,
            src_row_pitch: row_size,
            dst_row_pitch: row_size,
        }
    }

    pub fn with_dst_alignment(self, alignment: u32) -> Self {
        let alignment_mask = alignment - 1;
        self.with_dst_row_pitch(
            (self.row_size + alignment_mask) & !alignment_mask
        )
    }

    pub fn with_src_row_pitch(self, src_row_pitch: u32) -> Self {
        debug_assert!(src_row_pitch >= self.row_size);
        CopyParams { src_row_pitch, ..self }
    }

    pub fn with_dst_row_pitch(self, dst_row_pitch: u32) -> Self {
        debug_assert!(dst_row_pitch >= self.row_size);
        CopyParams { dst_row_pitch, ..self }
    }

    pub fn src_size(&self) -> u32 {
        self.src_row_pitch * self.n_rows
    }

    pub fn dst_size(&self) -> u32 {
        self.dst_row_pitch * self.n_rows
    }

    pub fn copy_bytes(&self, src: &[u8], dst: &mut[u8]) {
        if self.src_row_pitch == self.dst_row_pitch {
            let len = self.dst_size() as usize;
            dst.copy_from_slice(&src[0..len]);
            return;
        }

        let w = self.row_size as usize;
        let h = self.n_rows as usize;
        for y in 0..h {
            let row = &src[y * w .. (y + 1) * w];
            let dst_offset = y * self.dst_row_pitch as usize;
            dst[dst_offset..dst_offset + row.len()].copy_from_slice(row);
        }
    }
}
