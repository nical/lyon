use std;
use std::fmt;
use std::hash;
use std::marker::PhantomData;

pub struct Id<T> {
    handle: u16,
    _marker: PhantomData<T>,
}
impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> { fn clone(&self) -> Self { *self } }
impl<T> Id<T> {
    pub fn new(handle: u16) -> Self { Id { handle: handle, _marker: PhantomData  } }
    pub fn index(&self) -> usize { self.handle as usize }
    pub fn to_i32(&self) -> i32 { self.handle as i32 }
    pub fn as_range(&self) -> IdRange<T> { IdRange::new(*self, 1) }
}
impl<T> ::std::cmp::PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool { self.handle == other.handle }
    fn ne(&self, other: &Self) -> bool { self.handle != other.handle }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "#{}", self.handle) }
}

impl<T> hash::Hash for Id<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

#[derive(Copy, Clone)]
pub struct IdRange<T> {
    first: Id<T>,
    count: u16,
}

impl<T> IdRange<T> {
    pub fn new(first: Id<T>, count: u16) -> Self {
        IdRange {
            first: first,
            count: count,
        }
    }
    pub fn first(&self) -> Id<T> { self.first }
    pub fn first_index(&self) -> usize { self.first.index() }
    pub fn count(&self) -> usize { self.count as usize }
    pub fn get(&self, n: u16) -> Id<T> {
        assert!(n < self.count, "Shape id out of range.");
        Id::new(self.first.handle + n)
    }
}

pub struct CpuBuffer<T> {
    data: Vec<T>,
    allocator: SimpleBufferAllocator,
}

impl<T: Default+Copy> CpuBuffer<T> {
    pub fn new(size: u16) -> Self {
        CpuBuffer {
            data: vec![Default::default(); size as usize],
            allocator: SimpleBufferAllocator::new(size),
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
        self.allocator.alloc_range(count).map(|range|{ IdRange::new(Id::new(range.0), range.1) })
    }

    pub fn alloc_range(&mut self, count: u16) -> IdRange<T> {
        self.try_alloc_range(count).unwrap()
    }

    pub fn as_slice(&self) -> &[T] { &self.data[..] }

    pub fn len(&self) -> usize { self.data.len() }
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
