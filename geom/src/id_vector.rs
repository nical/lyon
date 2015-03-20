use halfedge::Id;
use std::default::Default;
use std::slice;
use std::marker::PhantomData;
use std::ops;

pub struct IdVector<T, ID:Copy> {
    data: Vec<T>,
    _idtype: PhantomData<ID>
}

impl<T, ID:Copy> IdVector<T, ID> {

    pub fn new() -> IdVector<T, ID> {
        IdVector {
            data: Vec::new(),
            _idtype: PhantomData
        }
    }

    pub fn with_capacity(size: u16) -> IdVector<T, ID> {
        IdVector {
            data: Vec::with_capacity(size as usize),
            _idtype: PhantomData
        }
    }

    pub fn from_vec(vec: Vec<T>) -> IdVector<T, ID> {
        IdVector {
            data: vec,
            _idtype: PhantomData
        }
    }

    pub fn into_vec(self) -> Vec<T> { self.data }

    pub fn len(&self) -> usize { self.data.len() }

    pub fn iter<'l>(&'l self) -> slice::Iter<'l, T> { self.data.iter() }

    pub fn iter_mut<'l>(&'l mut self) -> slice::IterMut<'l, T> { self.data.iter_mut() }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl<T:Default, ID:Copy> IdVector<T, ID> {
    pub fn resize(&mut self, size: u16) {
        let d = size as i32 - self.data.len() as i32;
        if d > 0 {
            self.data.reserve(d as usize);
            for _ in 0 .. d {
                self.data.push(Default::default());
            }
        } else {
            for _ in 0 .. -d {
                self.data.pop();
            }
        }
    }

    pub fn with_length(size: u16) -> IdVector<T, ID> {
        let mut result: IdVector<T,ID> = IdVector::new();
        result.resize(size);
        return result;
    }
}

impl<T, ID:Copy> ops::Index<Id<ID>> for IdVector<T, ID> {
    type Output = T;
    fn index<'l>(&'l self, id: &Id<ID>) -> &'l T { &self.data[id.as_index()] }
}

impl<T, ID:Copy> ops::IndexMut<Id<ID>> for IdVector<T, ID> {
    fn index_mut<'l>(&'l mut self, id: &Id<ID>) -> &'l mut T { &mut self.data[id.as_index()] }
}

pub struct IdSlice<'l, T:'l, ID:Copy> {
    slice: &'l[T],
    _idtype: PhantomData<ID>
}

impl<'l, T:'l, ID:Copy> Copy for IdSlice<'l, T, ID> {}

impl<'l, T:'l, ID:Copy> IdSlice<'l, T, ID> {
    pub fn new(slice: &'l[T]) -> IdSlice<'l, T, ID> {
        IdSlice {
            slice: slice,
            _idtype: PhantomData
        }
    }

    pub fn as_slice<'a>(&'a self) -> &'a[T] { self.slice }

    pub fn iter<'a>(&'a self) -> slice::Iter<'a, T> { self.slice.iter() }
}

impl<'l, T:'l, ID:Copy> ops::Index<Id<ID>> for IdSlice<'l, T, ID> {
    type Output = T;
    fn index<'a>(&'a self, id: &Id<ID>) -> &'a T { &self.slice[id.as_index()] }
}


pub struct MutIdSlice<'l, T:'l, ID:Copy> {
    slice: &'l mut[T],
    _idtype: PhantomData<ID>
}

impl<'l, T:'l, ID:Copy> MutIdSlice<'l, T, ID> {
    pub fn new(slice: &'l mut[T]) -> MutIdSlice<'l, T, ID> {
        MutIdSlice {
            slice: slice,
            _idtype: PhantomData
        }
    }

    pub fn iter<'a>(&'a self) -> slice::Iter<'a, T> { self.slice.iter() }
    pub fn iter_mut<'a>(&'a mut self) -> slice::IterMut<'a, T> { self.slice.iter_mut() }
}

impl<'l, T:'l, ID:Copy> ops::Index<Id<ID>> for MutIdSlice<'l, T, ID> {
    type Output = T;
    fn index<'a>(&'a self, id: &Id<ID>) -> &'a T { &self.slice[id.as_index()] }
}

impl<'l, T:'l, ID:Copy> ops::IndexMut<Id<ID>> for MutIdSlice<'l, T, ID> {
    fn index_mut<'a>(&'a mut self, id: &Id<ID>) -> &'a mut T { &mut self.slice[id.as_index()] }
}

