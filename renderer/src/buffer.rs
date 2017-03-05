use std;
use std::fmt;
use std::cmp;
use std::hash;
use std::marker::PhantomData;

pub struct Id<T> {
    handle: u16,
    _marker: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn new(handle: u16) -> Self { Id { handle: handle, _marker: PhantomData  } }
    pub fn index(&self) -> usize { self.handle as usize }
    pub fn to_i32(&self) -> i32 { self.handle as i32 }
    pub fn to_u16(&self) -> u16 { self.handle }
    pub fn as_range(&self) -> IdRange<T> { IdRange::new(self.handle, 1) }
}

impl<T> Copy for Id<T> {}

impl<T> Clone for Id<T> { fn clone(&self) -> Self { *self } }

impl<T> ::std::cmp::PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool { self.handle == other.handle }
    fn ne(&self, other: &Self) -> bool { self.handle != other.handle }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "#{}", self.handle) }
}

impl<T> hash::Hash for Id<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) { self.handle.hash(state); }
}

#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub struct IdRange<T> {
    first: Id<T>,
    count: u16,
}

impl<T> IdRange<T> {
    pub fn new(first: u16, count: u16) -> Self {
        IdRange {
            first: Id::new(first),
            count: count,
        }
    }

    pub fn empty() -> Self { IdRange::new(0, 0) }

    pub fn first(&self) -> Id<T> { self.first }

    pub fn first_index(&self) -> usize { self.first.index() }

    pub fn count(&self) -> usize { self.count as usize }

    pub fn get(&self, n: u16) -> Id<T> {
        assert!(n < self.count, "Shape id out of range.");
        Id::new(self.first.handle + n)
    }

    pub fn is_empty(&self) -> bool { self.count == 0 }

    pub fn contains(&self, id: Id<T>) -> bool {
        id.handle >= self.first.handle && id.handle < self.first.handle + self.count
    }

    pub fn intersection(&self, other: Self) -> Self {
        let first = cmp::max(self.first.handle, other.first.handle);
        let end = cmp::min(self.first.handle + self.count, other.first.handle + other.count);
        let count = if end > first { end - first } else { 0 };
        return IdRange::new(first, count);
    }

    pub fn including_id(&self, id: Id<T>) -> Self {
        if id.handle < self.first.handle {
            return IdRange {
                first: id,
                count: self.count + self.first.handle - id.handle,
            }
        }

        if id.handle >= self.first.handle + self.count {
            return IdRange {
                first: self.first,
                count: id.handle - self.first.handle + 1,
            }
        }

        return IdRange {
            first: self.first,
            count: self.count,
        };
    }
}

pub struct BufferId<T> {
    handle: u32,
    _marker: PhantomData<T>,
}
impl<T> BufferId<T> {
    pub fn new(handle: u32) -> Self { BufferId { handle: handle, _marker: PhantomData  } }
    pub fn index(&self) -> usize { self.handle as usize }
}

impl<T> Copy for BufferId<T> {}

impl<T> Clone for BufferId<T> { fn clone(&self) -> Self { *self } }

impl<T> ::std::cmp::PartialEq for BufferId<T> {
    fn eq(&self, other: &Self) -> bool { self.handle == other.handle }
    fn ne(&self, other: &Self) -> bool { self.handle != other.handle }
}

impl<T> fmt::Debug for BufferId<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Buffer#{}", self.handle) }
}

impl<T> hash::Hash for BufferId<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) { self.handle.hash(state); }
}

#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub struct BufferRange<T> {
    pub buffer: BufferId<T>,
    pub range: IdRange<T>,
}

pub struct CpuBuffer<T> {
    data: Vec<T>,
    allocator: SimpleBufferAllocator,
    dirty: bool, // TODO: Track dirty ranges
}

impl<T: Default+Copy> CpuBuffer<T> {
    pub fn new(size: u16) -> Self {
        CpuBuffer {
            data: vec![Default::default(); size as usize],
            allocator: SimpleBufferAllocator::new(size),
            dirty: true,
        }
    }

    pub fn try_alloc(&mut self) -> Option<Id<T>> {
        self.allocator.alloc().map(|idx|{ Id::new(idx) })
    }

    pub fn alloc(&mut self) -> Id<T> { self.try_alloc().unwrap() }

    pub fn push(&mut self, val: T) -> Id<T> {
        let id = self.alloc();
        self[id] = val;
        return id;
    }

    pub fn try_alloc_range(&mut self, count: u16) -> Option<IdRange<T>> {
        self.allocator.alloc_range(count).map(|range|{ IdRange::new(range.0, range.1) })
    }

    pub fn alloc_range(&mut self, count: u16) -> IdRange<T> {
        self.try_alloc_range(count).unwrap()
    }

    pub fn as_slice(&self) -> &[T] { &self.data[..] }

    pub fn len(&self) -> usize { self.data.len() }

    pub fn range(&self) -> IdRange<T> { IdRange::new(0, self.len() as u16) }

    pub fn sub_slice(&self, range: IdRange<T>) -> &[T] {
        let range = self.range().intersection(range);
        return &self.data[range.first_index()..(range.first_index() + range.count())]
    }

    pub fn flush_dirty_range(&mut self) -> IdRange<T> {
        if self.dirty {
            self.dirty = false;
            return self.range();
        }
        return IdRange::empty();
    }
}

impl<T> std::ops::Index<Id<T>> for CpuBuffer<T> {
    type Output = T;
    fn index(&self, id: Id<T>) -> &T {
        &self.data[id.index()]
    }
}

impl<T> std::ops::IndexMut<Id<T>> for CpuBuffer<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut T {
        &mut self.data[id.index()]
    }
}


pub struct SimpleBufferAllocator {
    back_index: u16,
    front_index: u16,
    len: u16,
}

impl SimpleBufferAllocator {
    pub fn new(len: u16) -> Self {
        SimpleBufferAllocator {
            back_index: len,
            front_index: 0,
            len: len,
        }
    }

    pub fn len(&self) -> u16 { self.len }

    pub fn available_size(&self) -> u16 { self.back_index - self.front_index }

    pub fn alloc_range_back(&mut self, len: u16) -> Option<(u16, u16)> {
        if self.available_size() < len {
            return None;
        }

        self.back_index -= len;

        return Some((self.back_index, len));
    }

    pub fn alloc_back(&mut self) -> Option<u16> {
        self.alloc_range_back(1).map(|range|{ range.0 })
    }

    pub fn alloc_range(&mut self, len: u16) -> Option<(u16, u16)> {
        if self.available_size() < len {
            return None;
        }

        let id = self.front_index;
        self.front_index += len;

        return Some((id, len));
    }

    pub fn alloc(&mut self) -> Option<u16> {
        self.alloc_range(1).map(|range|{ range.0 })
    }
}

pub struct TypedSimpleBufferAllocator<T> {
    alloc: SimpleBufferAllocator,
    _marker: PhantomData<T>,
}

impl<T> TypedSimpleBufferAllocator<T> {
    pub fn new(len: u16) -> Self {
        TypedSimpleBufferAllocator {
            alloc: SimpleBufferAllocator::new(len),
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> u16 { self.alloc.len() }

    pub fn alloc(&mut self) -> Option<Id<T>> {
        self.alloc.alloc().map(|id|{ Id::new(id) })
    }

    pub fn alloc_back(&mut self) -> Option<Id<T>> {
        self.alloc.alloc_back().map(|id|{ Id::new(id) })
    }

    pub fn alloc_range(&mut self, len: u16) -> Option<IdRange<T>> {
        self.alloc.alloc_range(len).map(|(first, count)|{
            IdRange::new(first, count)
        })
    }

    pub fn alloc_range_back(&mut self, len: u16) -> Option<IdRange<T>> {
        self.alloc.alloc_range_back(len).map(|(first, count)|{
            IdRange::new(first, count)
        })
    }
}
