use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AllocatorSegmentId(u16);

pub struct Allocator {
    segments: Vec<AllocatorSegment>,
}

struct AllocatorSegment {
    offset: u32,
    len: u32,
    allocated: bool,
}

impl Allocator {
    pub fn new(len: u32) -> Self {
        Allocator {
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

    pub fn can_grow(&self, allocation: u32) -> Option<u32> {
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

    pub fn print(&self) {
        println!("---");
        for segment in &self.segments {
            println!("{{offset:{},len:{},allocated:{}}}", segment.offset, segment.len, segment.allocated);
        }
        println!("---");
    }
}

#[test]
fn test_allocator() {
    let mut alloc = Allocator::new(100);

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
