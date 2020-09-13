use crate::allocator::*;

use std::slice;
use std::ops::Range;
use std::sync::Arc;
use std::marker::PhantomData;
use std::mem::{size_of, transmute};

pub fn as_mut_bytes<T: Copy>(slice: &mut [T]) -> &mut[u8] {
    unsafe {
        let ptr = slice.as_mut_ptr();
        let len = slice.len();
        slice::from_raw_parts_mut(
            transmute(ptr),
            len * size_of::<T>(),
        )
    }
}

pub fn as_bytes<T: Copy>(slice: &[T]) -> &[u8] {
    unsafe {
        let ptr = slice.as_ptr();
        let len = slice.len();
        slice::from_raw_parts(
            transmute(ptr),
            len * size_of::<T>(),
        )
    }
}

pub struct WritableMemory<'l> {
    writer: Arc<MemoryWriterInner>,
    memory: PhantomData<&'l mut[u8]>,
}

impl<'l> WritableMemory<'l> {
    pub fn new(memory: &'l mut[u8], offset: u32) -> Self {
        let len = memory.len() as u32;
        let buffer_ptr = memory.as_mut_ptr();
        Self {
            memory: PhantomData,
            writer: Arc::new(MemoryWriterInner {
                allocator: BumpAllocator::new(offset..(offset + len)),
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

impl MemoryWriter {
    pub fn allocate_front(&self, size_in_bytes: u32) -> Result<(Range<u32>, &mut[u8]), AllocError> {
        let range = self.inner.allocator.allocate_front(size_in_bytes)?;
        unsafe { Ok(self.allocated(range)) }
    }

    pub fn allocate_back(&self, size_in_bytes: u32) -> Result<(Range<u32>, &mut[u8]), AllocError> {
        let range = self.inner.allocator.allocate_back(size_in_bytes)?;
        unsafe { Ok(self.allocated(range)) }
    }

    unsafe fn allocated(
        &self,
        range: Range<u32>
    ) -> (Range<u32>, &mut[u8]) {
        (
            range.clone(),
            slice::from_raw_parts_mut(
                self.inner.buffer_ptr.offset(range.start as isize),
                (range.end - range.start) as usize
            ),
        )
    }

    pub fn write_front<T>(&self, slice: &[T]) -> Result<Range<u32>, AllocError>
    where T: Copy {
        self.write_front_bytes(as_bytes(slice))
    }

    pub fn write_back<T>(&self, slice: &[T]) -> Result<Range<u32>, AllocError>
    where T: Copy {
        self.write_back_bytes(as_bytes(slice))
    }

    fn write_front_bytes(&self, slice: &[u8]) -> Result<Range<u32>, AllocError> {
        let bytes_len = slice.len();
        let size = bytes_len as u32;
        //let size = align_u32(bytes_len as u32, GPU_BLOCK_SIZE as u32);
        let (id, mem) = self.allocate_front(size)?;
        mem[..bytes_len].copy_from_slice(slice);

        Ok(id)
    }

    fn write_back_bytes(&self, slice: &[u8]) -> Result<Range<u32>, AllocError> {
        let bytes_len = slice.len();
        let size = bytes_len as u32;
        //let size = align_u32(bytes_len as u32, GPU_BLOCK_SIZE as u32);
        let (id, mem) = self.allocate_back(size)?;
        mem[..bytes_len].copy_from_slice(slice);

        Ok(id)
    }
}


struct MemoryWriterInner {
    allocator: BumpAllocator,
    buffer_ptr: *mut u8,
}

pub fn align_u32(size: u32, alignment: u32) -> u32 {
    let mask = alignment - 1;
    (size + mask) & !mask
}
