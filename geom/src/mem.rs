use std::ptr::{ write_bytes };
use std::mem::{forget, transmute, size_of};

/// An Owned buffer that can be recycled into a Vec to help with avoiding allocations.
///
/// # Examples
///
/// ```
/// use geom::mem::*;
///
/// let mut v = vec![1u16, 2, 3, 4, 5];
/// let mut storage = recycle_vec(v);
/// // v is now gone into the void.
///
/// let mut v: Vec<f32> = create_vec_from(storage);
///
/// assert_eq!(v.len(), 0);
/// assert!(v.capacity() > 0);
///
/// ```


/// Holds memory that was allocated by a Vec and can be reused by another Vec.
///
/// This is useful to avoid reallocating temporary vectors.
pub struct Allocation {
    ptr: *mut u8,
    size: usize,
}

pub fn pre_allocate(size: usize) -> Allocation {
    let alloc: Vec<u8> = Vec::with_capacity(size);
    return recycle_vec(alloc);
}

/// Consumes a Vec and creates an allocation that holds the Vec's memory.
///
/// The vector is cleared and its data is dropped before the it is consumed.
pub fn recycle_vec<T>(mut vector: Vec<T>) -> Allocation {
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

    /// Returns the size of the buffer in bytes.
    pub fn size(&self) -> usize { self.size }

    /// Fills the buffer with the byte pattern.
    pub fn fill(&mut self, pattern: u8) {
        if self.size == 0 { return; }
        unsafe {
            write_bytes(self.ptr, pattern, self.size);
        }
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
    let alloc = recycle_vec(v);
    assert_eq!(drop_count, 5);
}