use allocator::*;
use GpuBlock;
use GpuData;
use GpuBufferRange;
use GpuBufferOffset;
use GpuBuffer;

use std::slice;
use std::ops::Range;
use std::sync::Arc;
use std::marker::PhantomData;
use std::mem::{size_of, transmute};

pub fn as_mut_typed_slice<T>(blocks: &mut [GpuBlock]) -> &mut[T]
where T: GpuData {
    unsafe {
        let ptr = blocks.as_mut_ptr();
        let len = blocks.len();
        slice::from_raw_parts_mut(
            transmute(ptr),
            (len * size_of::<GpuBlock>()) / size_of::<T>(),
        )
    }
}

pub fn as_mut_gpu_blocks<T>(data: &mut [T]) -> &mut[GpuBlock]
where T: GpuData {
    unsafe {
        let ptr = data.as_mut_ptr();
        let len = data.len();
        slice::from_raw_parts_mut(
            transmute(ptr),
            (len * size_of::<T>()) / size_of::<GpuBlock>(),
        )
    }
}

pub fn as_gpu_blocks<T>(data: &[T]) -> &[GpuBlock]
where T: GpuData {
    unsafe {
        let ptr = data.as_ptr();
        let len = data.len();
        slice::from_raw_parts(
            transmute(ptr),
            (len * size_of::<T>()) / size_of::<GpuBlock>(),
        )
    }
}

pub fn as_mut_bytes(blocks: &mut [GpuBlock]) -> &mut[u8] {
    unsafe {
        let ptr = blocks.as_mut_ptr();
        let len = blocks.len();
        slice::from_raw_parts_mut(
            transmute(ptr),
            len * size_of::<GpuBlock>(),
        )
    }
}

pub struct WritableMemory<'l> {
    writer: Arc<MemoryWriterInner>,
    memory: PhantomData<&'l mut[GpuBlock]>,
}

impl<'l> WritableMemory<'l> {
    pub fn new(memory: &'l mut[GpuBlock], location: GpuBufferOffset<GpuBlock>) -> Self {
        let len = memory.len() as u32;
        let buffer_ptr = memory.as_mut_ptr();
        let offset = location.offset;
        Self {
            memory: PhantomData,
            writer: Arc::new(MemoryWriterInner {
                allocator: BumpAllocator::new(offset..(offset + len)),
                buffer_id: location.buffer,
                buffer_ptr,
            }),
        }
    }

    pub fn new_writer(&self) -> MemoryWriter {
        MemoryWriter {
            inner: Arc::clone(&self.writer),
        }
    }

    pub fn has_writers(&self) -> bool {
        Arc::strong_count(&self.writer) > 1
    }
}


pub struct MemoryWriter {
    inner: Arc<MemoryWriterInner>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AllocSize {
    pub n_blocks: u32,
}

impl AllocSize {
    pub fn blocks(n_blocks: u32) -> Self {
        AllocSize { n_blocks }
    }

    pub fn bytes(size: usize) -> Self {
        let block_size = size_of::<GpuBlock>() as u32;
        AllocSize {
            n_blocks: align_u32(size as u32, block_size) / block_size
        }
    }

    pub fn in_bytes(&self) -> usize {
        self.n_blocks as usize * size_of::<GpuBlock>()
    }

    pub fn in_blocks(&self) -> u32 {
        self.n_blocks
    }
}

impl MemoryWriter {
    pub fn allocate_front(&self, size: AllocSize) -> Result<(GpuBufferRange<GpuBlock>, &mut[GpuBlock]), AllocError> {
        let range = self.inner.allocator.allocate_front(size.n_blocks)?;
        unsafe { Ok(self.allocated(range)) }
    }

    pub fn allocate_back(&self, size: AllocSize) -> Result<(GpuBufferRange<GpuBlock>, &mut[GpuBlock]), AllocError> {
        let range = self.inner.allocator.allocate_back(size.n_blocks)?;
        unsafe { Ok(self.allocated(range)) }
    }

    unsafe fn allocated(
        &self,
        range: Range<u32>
    ) -> (GpuBufferRange<GpuBlock>, &mut[GpuBlock]) {
        (
            GpuBufferRange { buffer: self.inner.buffer_id, range: range.clone() },
            slice::from_raw_parts_mut(
                self.inner.buffer_ptr.offset(range.start as isize),
                (range.end - range.start) as usize
            ),
        )
    }

    pub fn write_front<T>(&self, slice: &[T]) -> Result<GpuBufferRange<T>, AllocError>
    where T: GpuData {
        debug_assert!(size_of::<T>() % size_of::<GpuBlock>() == 0);
        let blocks = as_gpu_blocks(slice);
        let size = AllocSize::blocks(blocks.len() as u32);
        let (id, mem) = self.allocate_front(size)?;
        mem.copy_from_slice(blocks);

        Ok(unsafe { id.cast() })
    }

    pub fn write_back<T>(&self, slice: &[T]) -> Result<GpuBufferRange<T>, AllocError>
    where T: GpuData {
        debug_assert!(size_of::<T>() % size_of::<GpuBlock>() == 0);
        let blocks = as_gpu_blocks(slice);
        let size = AllocSize::blocks(blocks.len() as u32);
        let (id, mem) = self.allocate_back(size)?;
        mem.copy_from_slice(blocks);

        Ok(unsafe { id.cast() })
    }
}

struct MemoryWriterInner {
    allocator: BumpAllocator,
    buffer_ptr: *mut GpuBlock,
    buffer_id: GpuBuffer<GpuBlock>,
}

pub fn align_u32(size: u32, alignment: u32) -> u32 {
    let mask = alignment - 1;
    (size + mask) & !mask
}