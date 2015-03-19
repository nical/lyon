use std::ptr::{Unique, write_bytes};
use std::mem::{forget, transmute, size_of};

pub struct VecStorage {
    ptr: Unique<u8>,
    cap: usize,
}

impl VecStorage {
  pub fn from_vec<T>(mut vector: Vec<T>) -> VecStorage {
    vector.clear();
    let capacity = vector.capacity() * size_of::<T>();
    unsafe {
        let p = vector.as_mut_ptr();
        forget(vector);
    
        return VecStorage {
          ptr: Unique::new(transmute(p)),
          cap: capacity,
        };
    }
  }
  
  pub fn into_vec<T>(self) -> Vec<T> {
    unsafe {
        let vector = Vec::from_raw_parts(transmute(self.ptr.get()), 0,
        self.cap / size_of::<T>());
        forget(self);
        return vector;
    }
  }
  
  pub fn capacity(&self) -> usize { self.cap }

  pub fn zero_out(&mut self) {
    if self.cap == 0 { return; }
    unsafe {
        write_bytes(self.ptr.get_mut(), 0, self.cap);
    }
  }
}
