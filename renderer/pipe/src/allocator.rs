use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

pub trait Allocator {
    fn allocate(&mut self, size: u32) -> Result<Range<u32>, AllocError>;
}


pub struct BumpAllocator {
    start: AtomicUsize,
    end: AtomicUsize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AllocError {
    Oom,
}

impl BumpAllocator {
    pub fn new(range: Range<u32>) -> Self {
        BumpAllocator {
            start: AtomicUsize::new(range.start as usize),
            end: AtomicUsize::new(range.end as usize),
        }
    }

    pub fn allocate_front(&self, size: u32) -> Result<Range<u32>, AllocError> {
        let start = self.start.fetch_add(size as usize, Ordering::SeqCst) as u32;
        let end = self.end.load(Ordering::SeqCst) as u32;
        if start + size > end {
            return Err(AllocError::Oom);
        }

        Ok(start..(start + size))
    }

    pub fn allocate_back(&self, size: u32) -> Result<Range<u32>, AllocError> {
        let end = self.end.fetch_sub(size as usize, Ordering::SeqCst) as u32;
        let start = self.start.load(Ordering::SeqCst) as u32;
        if start + size > end {
            return Err(AllocError::Oom);
        }

        Ok((end - size)..end)
    }

    pub fn split_front(&self, size: u32) -> Result<Self, AllocError> {
        Ok(BumpAllocator::new(self.allocate_front(size)?))
    }

    pub fn split_back(&self, size: u32) -> Result<Self, AllocError> {
        Ok(BumpAllocator::new(self.allocate_back(size)?))
    }

    pub fn reset(&self, range: Range<u32>) {
        // First set an empty range to avoid races with other threads trying to allocate.
        self.start.store(self.end.load(Ordering::SeqCst), Ordering::SeqCst);

        self.start.store(range.start as usize, Ordering::SeqCst);
        self.end.store(range.end as usize, Ordering::SeqCst);
    }
}

impl Allocator for BumpAllocator {
    fn allocate(&mut self, size: u32) -> Result<Range<u32>, AllocError> {
        self.allocate_front(size)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AllocatorSegmentId(u16);

pub struct SegmentAllocator {
    segments: Vec<AllocatorSegment>,
}

struct AllocatorSegment {
    offset: u32,
    len: u32,
    allocated: bool,
}

impl SegmentAllocator {
    pub fn new(len: u32) -> Self {
        SegmentAllocator {
            segments: vec![
                AllocatorSegment { offset: 0, len, allocated: false },
            ],
        }
    }

    pub fn allocate(&mut self, len: u32) -> Result<Range<u32>, AllocError> {
        for segment in &mut self.segments {
            if !segment.allocated && segment.len == len {
                segment.allocated = true;
                return Ok(segment.offset..(segment.offset + len));
            }
        }

        let mut new_segment = None;
        let mut result = None;
        for (i, segment) in self.segments.iter_mut().enumerate() {
            if !segment.allocated && segment.len > len {
                new_segment = Some((i+1, segment.offset + len, segment.len - len));
                segment.allocated = true;
                segment.len = len;
                result = Some(segment.offset..(segment.offset + len));
                break;
            }
        }

        if let Some((i, offset, len)) = new_segment {
            self.segments.insert(i, AllocatorSegment { offset, len, allocated: false })
        }

        return if let Some(alloc) = result {
            Ok(alloc)
        } else {
            Err(AllocError::Oom)
        };
    }

    pub fn deallocate(&mut self, offset: u32) {
        let mut index = None;
        for (i, segment) in self.segments.iter_mut().enumerate() {
            if segment.offset == offset {
                debug_assert!(segment.allocated);
                if segment.allocated {
                    segment.allocated = false;
                    index = Some(i);
                }
                break;
            }
        }

        debug_assert!(index.is_some());

        if let Some(i) = index {
            if i < self.segments.len() {
                if !self.segments[i + 1].allocated {
                    self.segments[i].len += self.segments[i + 1].len;
                    self.segments.remove(i + 1);
                }
            }
            if i > 0 {
                if !self.segments[i - 1].allocated {
                    self.segments[i - 1].len += self.segments[i].len;
                    self.segments.remove(i);
                }
            }
        }
    }

    pub fn can_grow_allocation(&self, allocation: u32) -> Option<u32> {
        let mut next = false;
        for segment in &self.segments {
            if next && !segment.allocated {
                return Some(segment.len);
            }
            if segment.offset == allocation {
                debug_assert!(segment.allocated);
                if segment.allocated {
                    next = true;
                }
            }
        }

        None
    }

    pub fn grow_storage(&mut self, extra_size: u32, allocated: bool) -> Range<u32> {
        let last_segment = self.segments.last().unwrap();
        let start = last_segment.offset + last_segment.len;
        self.segments.push(AllocatorSegment {
            offset: start,
            len: extra_size,
            allocated,
        });

        start .. (start + extra_size)
    }

    pub fn print(&self) {
        println!("---");
        for segment in &self.segments {
            println!("{{offset:{},len:{},allocated:{}}}", segment.offset, segment.len, segment.allocated);
        }
        println!("---");
    }
}

impl Allocator for SegmentAllocator {
    fn allocate(&mut self, size: u32) -> Result<Range<u32>, AllocError> {
        self.allocate(size)
    }
}

#[derive(Clone)]
pub struct BlockAllocator {
    blocks: Vec<bool>,
    free_blocks: u16,
    max_blocks: u16,
}

impl BlockAllocator {
    pub fn new(max_blocks: u16) -> Self {
        BlockAllocator {
            blocks: Vec::new(),
            free_blocks: 0,
            max_blocks,
        }
    }

    pub fn allocate(&mut self, n_blocks: u16) -> Result<Blocks, AllocError> {
        if self.free_blocks > n_blocks {
            let mut i = 0;
            let end = self.blocks.len() as i32 - n_blocks as i32;
            'outer: while i < end {
                let mut j = 0;
                while j < n_blocks {
                    if self.blocks[(i + j as i32) as usize] {
                        i += (j + 1) as i32;
                        continue 'outer;
                    }
                    j += 1;
                }

                let allocated = Blocks {
                    start: i as u16,
                    end: i as u16 + j,
                };

                for block in &mut self.blocks[allocated.range()] {
                    *block = true;
                }

                self.free_blocks -= n_blocks;

                return Ok(allocated);
            }
        }

        if n_blocks + (self.blocks.len() as u16) > self.max_blocks {
            return Err(AllocError::Oom);
        }

        let range_start = self.blocks.len() as u16;
        for _ in 0..n_blocks {
            self.blocks.push(true);
        }

        Ok(Blocks {
            start: range_start,
            end: (range_start + n_blocks as u16),
        })
    }

    pub fn deallocate(&mut self, blocks: Blocks) {
        for block in &mut self.blocks[blocks.range()] {
            assert!(*block, "Double-free!");
            *block = false;
        }

        self.free_blocks += blocks.count();

        if blocks.end == self.blocks.len() as u16 {
            while let Some(false) = self.blocks.last() {
                self.blocks.pop();
                self.free_blocks -= 1;
            }
        }
    }

    pub fn num_blocks(&self) -> u16 {
        self.blocks.len() as u16
    }

    pub fn num_free_blocks(&self) -> u16 {
        self.free_blocks
    }

    pub fn num_allocated_blocks(&self) -> u16 {
        self.num_blocks() - self.free_blocks
    }

    pub fn set_max_blocks(&mut self, max_blocks: u16) {
        self.max_blocks = max_blocks.min(self.num_blocks());
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Blocks {
    pub start: u16,
    pub end: u16
}

impl Blocks {
    pub fn count(&self) -> u16 {
        self.end - self.start
    }

    fn range(&self) -> Range<usize> {
        (self.start as usize) .. (self.end as usize)
    }
}

#[test]
fn test_allocator() {
    let mut alloc = SegmentAllocator::new(100);

    let a = alloc.allocate(10).unwrap();
    let b = alloc.allocate(20).unwrap();
    let c = alloc.allocate(30).unwrap();
    alloc.deallocate(b.start);
    assert_eq!(alloc.can_grow(a.start), Some(20));
    alloc.deallocate(a.start);
    let d = alloc.allocate(35).unwrap();
    let e = alloc.allocate(30).unwrap();
    assert_eq!(e.start, a.start);
    assert_eq!(alloc.allocate(10), Err(AllocError::Oom));
    let f = alloc.allocate(5).unwrap();
}

#[test]
fn block_allocator() {
    let mut alloc = BlockAllocator::new(10);

    assert_eq!(alloc.num_free_blocks(), 0);
    assert_eq!(alloc.num_allocated_blocks(), 0);
    assert_eq!(alloc.num_blocks(), 0);

    let a = alloc.allocate(1).unwrap();
    let b = alloc.allocate(2).unwrap();
    let c = alloc.allocate(1).unwrap();
    let d = alloc.allocate(4).unwrap();
    let e = alloc.allocate(2).unwrap();

    assert_eq!(
        &alloc.blocks[..],
        &[true, true, true, true, true, true, true, true, true, true]
    );

    assert_eq!(alloc.allocate(1), Err(AllocError::Oom));

    assert_eq!(alloc.num_free_blocks(), 0);
    assert_eq!(alloc.num_allocated_blocks(), 10);
    assert_eq!(alloc.num_blocks(), 10);

    alloc.deallocate(d);

    assert_eq!(
        &alloc.blocks[..],
        &[true, true, true, true, false, false, false, false, true, true]
    );

    assert_eq!(alloc.num_free_blocks(), 4);
    assert_eq!(alloc.num_allocated_blocks(), 6);
    assert_eq!(alloc.num_blocks(), 10);

    alloc.deallocate(b);

    assert_eq!(
        &alloc.blocks[..],
        &[true, false, false, true, false, false, false, false, true, true]
    );

    assert_eq!(alloc.num_free_blocks(), 6);
    assert_eq!(alloc.num_allocated_blocks(), 4);
    assert_eq!(alloc.num_blocks(), 10);

    let f = alloc.allocate(3).unwrap();

    assert_eq!(
        &alloc.blocks[..],
        &[true, false, false, true, true, true, true, false, true, true]
    );

    assert_eq!(alloc.num_free_blocks(), 3);
    assert_eq!(alloc.num_allocated_blocks(), 7);
    assert_eq!(alloc.num_blocks(), 10);

    alloc.deallocate(e);

    assert_eq!(
        &alloc.blocks[..],
        &[true, false, false, true, true, true, true]
    );

    assert_eq!(alloc.num_free_blocks(), 2);
    assert_eq!(alloc.num_allocated_blocks(), 5);
    assert_eq!(alloc.num_blocks(), 7);

    alloc.deallocate(a);
    alloc.deallocate(c);
    alloc.deallocate(f);

    assert_eq!(alloc.num_free_blocks(), 0);
    assert_eq!(alloc.num_allocated_blocks(), 0);
    assert_eq!(alloc.num_blocks(), 0);
}