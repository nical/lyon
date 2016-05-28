use std::ptr::{ write_bytes };
use std::mem::{forget, transmute, size_of};

/// An Owned buffer that can be recycled into a data structures such as Vec to
/// help with avoiding allocations.
///
/// # Examples
///
/// ```
/// use buffer::*;
///
/// let mut v = vec![1u16, 2, 3, 4, 5];
/// let mut storage = vec::recycle(v);
/// // v is now gone into the void.
///
/// let mut v: Vec<f32> = vec::new_vec(storage);
///
/// assert_eq!(v.len(), 0);
/// assert!(v.capacity() > 0);
///
/// ```
///
/// ```
/// use buffer::*;
/// use std::mem::swap;
///
/// struct Processor {
///   buffer: Allocation,
/// }
///
/// impl Processor {
///   fn process(&mut self, input: u32) {
///     let mut alloc = Allocation::empty();
///     swap(&mut alloc, &mut self.buffer);
///     let mut intermediate_thing: Vec<&u32> = vec::new_vec(alloc);
///     // .. do work using intermediate_thing;
///     self.buffer = vec::recycle(intermediate_thing);
///   }
/// }
/// let mut v = vec![1u16, 2, 3, 4, 5];
/// let mut storage = vec::recycle(v);
/// // v is now gone into the void.
///
/// let mut v: Vec<f32> = vec::new_vec(storage);
///
/// assert_eq!(v.len(), 0);
/// assert!(v.capacity() > 0);
///
/// ```


/// Holds memory a contiguous block of memory that can be recycled by data structures..
///
/// This is useful to avoid reallocating temporary vectors.
pub struct Allocation {
    ptr: *mut u8,
    size: usize,
}

pub mod vec {
    use super::Allocation;
    use std::mem::{forget, transmute, size_of};

    pub fn recycle<T>(mut vector: Vec<T>) -> Allocation {
        vector.clear();
        let size = vector.capacity() * size_of::<T>();
        unsafe {
            let p = vector.as_mut_ptr();
            forget(vector);
            return Allocation {
                ptr: transmute(p),
                size: size,
            };
        }
    }

    pub fn new_vec<T>(alloc: Allocation) -> Vec<T> {
        unsafe {
            let vector = Vec::from_raw_parts(transmute(alloc.ptr), 0, alloc.size / size_of::<T>());
            forget(alloc);
            return vector;
        }
    }
}

/// Creates a Vec using this allocation.
///
/// The length of the new vector is 0 and the size is self.size() / size_of::<T>().
pub fn create_vec_from<T>(alloc: Allocation) -> Vec<T> {
    unsafe {
        let vector = Vec::from_raw_parts(transmute(alloc.ptr), 0, alloc.size / size_of::<T>());
        forget(alloc);
        return vector;
    }
}


impl Allocation {
    /// Creates an empty Allocation.
    pub fn empty() -> Allocation {
        unsafe {
            Allocation {
                ptr: transmute(0 as usize),
                size: 0
            }
        }
    }

    /// Creates an allocation of size bytes
    pub fn allocate_bytes(size: usize) -> Allocation
    {
        let alloc: Vec<u8> = Vec::with_capacity(size);
        return vec::recycle(alloc);
    }

    /// Returns the size of the buffer in bytes.
    pub fn size(&self) -> usize { self.size }

    /// Fills the buffer with the byte pattern.
    pub fn fill(&mut self, pattern: u8) {
        if self.size == 0 { return; }
        unsafe {
            write_bytes(self.ptr, pattern, self.size);
        }
    }

    /// Creates an allocation directly from raw components.
    pub unsafe fn from_raw_parts(ptr: *mut u8, size: usize) -> Allocation {
        Allocation {
            ptr: ptr,
            size: size,
        }
    }

    /// Consume the allocation and returns its raw parts.
    pub unsafe fn into_raw_parts(self) -> (*mut u8, usize) {
        let result = (self.ptr, self.size);
        forget(self);
        return result;
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe {
            // let a vector acquire the buffer and drop it
            let _ : Vec<u8> = Vec::from_raw_parts(transmute(self.ptr), 0, self.size);
        }
    }
}

#[cfg(test)]
struct A { drop: *mut u32 }
#[cfg(test)]
impl Drop for A {
    fn drop(&mut self) {
        unsafe { *self.drop += 1; }
    }
}

#[test]
fn test_alloc_drop() {
    let mut drop_count: u32 = 0;
    let mut v = Vec::new();
    v.push(A { drop: &mut drop_count });
    v.push(A { drop: &mut drop_count });
    v.push(A { drop: &mut drop_count });
    v.push(A { drop: &mut drop_count });
    v.push(A { drop: &mut drop_count });
    assert_eq!(drop_count, 0);
    let alloc = vec::recycle(v);
    assert_eq!(drop_count, 5);
    assert!(alloc.size() >= 5*size_of::<A>());
}

#[test]
fn test_alloc_simple() {
    let size_of = 4;
    let num_elems = 1337;
    let alloc = Allocation::allocate_bytes(size_of*num_elems);
    assert!(alloc.size() == size_of*num_elems);
    let v: Vec<u32> = vec::new_vec(alloc);
    assert!(v.len() == 0);
    assert!(v.capacity() == num_elems);
    let alloc = vec::recycle(v);
    assert!(alloc.size() == size_of*num_elems);
}

