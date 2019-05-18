use crate::allocator::*;
use crate::writer::*;
use crate::{Renderer, GPU_BLOCK_SIZE, GpuData, BufferRange, BufferId};

use std::ops::Range;
use std::sync::Arc;

pub struct GpuBufferAllocator {
    allocator: Arc<BumpAllocator>,
    buffer_id: BufferId,
}

impl GpuBufferAllocator {
    pub fn new(buffer_id: BufferId, allocator: Arc<BumpAllocator>) -> Self {
        GpuBufferAllocator {
            allocator,
            buffer_id,
        }
    }

    pub fn allocate_front(&self, size_in_bytes: u32) -> Result<BufferRange, AllocError> {
        self.allocator.allocate_front(size_in_bytes).map(|range| self.buffer_id.range(range))
    }

    pub fn allocate_back(&self, size_in_bytes: u32) -> Result<BufferRange, AllocError> {
        self.allocator.allocate_back(size_in_bytes).map(|range| self.buffer_id.range(range))
    }
}


pub struct TransferBufferWriter {
    writer: MemoryWriter,
    copy_commands: BufferCopyCommands,
}

impl TransferBufferWriter {
    pub fn new(writer: MemoryWriter, buffer_id: BufferId) -> Self {
        TransferBufferWriter {
            writer,
            copy_commands: BufferCopyCommands::new(buffer_id),
        }
    }

    pub fn transfer_buffer_id(&self) -> BufferId {
        self.copy_commands.src_buffer_id
    }

    pub fn write_front<T>(&mut self, slice: &[T], dst: &GpuBufferAllocator) -> Result<(BufferRange, BufferRange), AllocError>
    where T: GpuData {
        self.write_front_bytes(as_bytes(slice), dst)
    }

    pub fn write_back<T>(&mut self, slice: &[T], dst: &GpuBufferAllocator) -> Result<(BufferRange, BufferRange), AllocError>
    where T: GpuData {
        self.write_back_bytes(as_bytes(slice), dst)
    }

    pub fn flush_copy_commands(&mut self) -> BufferCopyCommands {
        self.copy_commands.flush()
    }

    fn write_front_bytes(&mut self, slice: &[u8], dst: &GpuBufferAllocator) -> Result<(BufferRange, BufferRange), AllocError> {
        let size = align_u32(slice.len() as u32, GPU_BLOCK_SIZE as u32);
        let dst_range = dst.allocate_front(size)?;
        let src_range = self.writer.write_front(slice)?;

        let src_range = BufferRange {
            buffer: self.transfer_buffer_id(),
            range: src_range,
        };

        self.copy_commands.push((src_range.clone(), dst_range.clone()));

        Ok((src_range, dst_range))
    }

    fn write_back_bytes(&mut self, slice: &[u8], dst: &GpuBufferAllocator) -> Result<(BufferRange, BufferRange), AllocError> {
        let size = align_u32(slice.len() as u32, GPU_BLOCK_SIZE as u32);
        let dst_range = dst.allocate_front(size)?;
        let src_range = self.writer.write_back(slice)?;

        let src_range = BufferRange {
            buffer: self.transfer_buffer_id(),
            range: src_range,
        };

        self.copy_commands.push((src_range.clone(), dst_range.clone()));

        Ok((src_range, dst_range))
    }
}

pub struct BufferCopyCommands {
    updates: Vec<(Range<u32>, BufferRange)>,
    src_buffer_id: BufferId,
}

impl BufferCopyCommands {
    pub fn new(src_buffer_id: BufferId) -> Self {
        BufferCopyCommands {
            updates: Vec::new(),
            src_buffer_id,
        }
    }

    pub fn push(&mut self, update: (BufferRange, BufferRange)) {
        let (src, dst) = update;
        debug_assert!(src.buffer == self.src_buffer_id);

        if let Some((prev_src, prev_dst)) = self.updates.last_mut() {
            if prev_dst.buffer == prev_dst.buffer
                && prev_src.end == src.range.start
                && prev_dst.range.end == dst.range.start {
                prev_src.end = src.range.end;
                prev_dst.range.end = dst.range.end;

                return;
            }
        }

        self.updates.push((src.range, dst));
    }

    pub fn apply(&mut self, src_buffer: &wgpu::Buffer, renderer: &Renderer, encoder: &mut wgpu::CommandEncoder) {
        for &(ref src, ref dst) in &self.updates {
            if dst.len() == 0 {
                continue;
            }

            encoder.copy_buffer_to_buffer(
                src_buffer, src.start,
                &renderer[dst.buffer], dst.range.start,
                dst.len(),
            );
        }
    }

    pub fn flush(&mut self) -> Self {
        BufferCopyCommands {
            updates: std::mem::replace(&mut self.updates, Vec::new()),
            src_buffer_id: self.src_buffer_id,
        }
    }
}


pub struct TransferBufferPool {
    buffer_size: u32,
}

// TODO: with wgpu's current API it is really hard to implement a pool of buffers that can be
// synchronously mapped. This pool is a stub implementation that doesn't actually pool any
// buffer and reallocates each time. Cf. https://github.com/gfx-rs/wgpu-rs/issues/9
impl  TransferBufferPool {
    pub fn new(buffer_size: u32) -> Self {
        TransferBufferPool {
            buffer_size
        }
    }

    pub fn get<'d>(&mut self, device: &'d wgpu::Device) -> wgpu::CreateBufferMapped<'d, u8> {
        device.create_buffer_mapped(
            self.buffer_size as usize,
            wgpu::BufferUsageFlags::TRANSFER_SRC,
        )
    }

    pub fn recycle(&mut self, _buffer: wgpu::Buffer) {}
}

/*
use std::sync::mpsc::{channel, Sender, Receiver};

pub struct TransferBufferPool2 {
    buffers: Vec<Option<wgpu::Buffer>>,
    sender: Sender<usize>,
    receiver: Receiver<usize>,
}

impl TransferBufferPool2 {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        TransferBufferPool2 {
            buffers: Vec::new(),
            sender,
            receiver,
        }
    }

    pub fn get(&mut self, device: &wgpu::Device) -> wgpu::Buffer {
        match self.receiver.try_recv() {
            Ok(index) => {
                return std::mem::replace(&mut self.buffers[index], None).unwrap();
            }
            _ => {}
        }

        device.create_buffer(&wgpu::BufferDescriptor {
            size: 4096 * 8, // TODO
            usage: wgpu::BufferUsageFlags::TRANSFER_SRC,
        })
    }

    pub fn recycle(&mut self, buffer: wgpu::Buffer) {
        let mut index = None;
        for (i, slot) in self.buffers.iter().enumerate() {
            if slot.is_none() {
                index = Some(i);
                break;
            }
        }

        let index = index.unwrap_or_else(|| {
            self.buffers.push(None);
            self.buffers.len() - 1
        });

        let sender = self.sender.clone();
        buffer.map_write_async(0, 0, |mapping: wgpu::BufferMapAsyncResult<&mut [u8]>| {
            if let wgpu::BufferMapAsyncResult::Success(..) = mapping {
                sender.send(index).unwrap();
            }
        });

        self.buffers[index] = Some(buffer);
    }
}
*/
