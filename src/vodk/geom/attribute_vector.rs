use halfedge::Id;
use std::default::Default;
use std::slice;
use std::marker::PhantomData;
use std::ops;

pub struct AttributeVector<T, ID:Copy> {
    data: Vec<T>,
    _idtype: PhantomData<ID>
}

impl<T, ID:Copy> AttributeVector<T, ID> {

    pub fn new() -> AttributeVector<T, ID> {
        AttributeVector {
            data: Vec::new(),
            _idtype: PhantomData
        }
    }

    pub fn with_capacity(size: u16) -> AttributeVector<T, ID> {
        AttributeVector {
            data: Vec::with_capacity(size as usize),
            _idtype: PhantomData
        }
    }

    pub fn from_vec(vec: Vec<T>) -> AttributeVector<T, ID> {
        AttributeVector {
            data: vec,
            _idtype: PhantomData
        }
    }

    pub fn into_vec(self) -> Vec<T> { self.data }

    pub fn len(&self) -> usize { self.data.len() }

    pub fn iter<'l>(&'l self) -> slice::Iter<'l, T> { self.data.iter() }

    pub fn iter_mut<'l>(&'l mut self) -> slice::IterMut<'l, T> { self.data.iter_mut() }
}

impl<T:Default, ID:Copy> AttributeVector<T, ID> {
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

    pub fn with_length(size: u16) -> AttributeVector<T, ID> {
        let mut result: AttributeVector<T,ID> = AttributeVector::new();
        result.resize(size);
        return result;        
    }
}

impl<T, ID:Copy> ops::Index<Id<ID>> for AttributeVector<T, ID> {
    type Output = T;
    fn index<'l>(&'l self, id: &Id<ID>) -> &'l T { &self.data[id.as_index()] }
}

impl<T, ID:Copy> ops::IndexMut<Id<ID>> for AttributeVector<T, ID> {
    fn index_mut<'l>(&'l mut self, id: &Id<ID>) -> &'l mut T { &mut self.data[id.as_index()] }
}

pub struct AttributeSlice<'l, T:'l, ID:Copy> {
    slice: &'l[T],
    _idtype: PhantomData<ID>
}

impl<'l, T:'l, ID:Copy> Copy for AttributeSlice<'l, T, ID> {} 

impl<'l, T:'l, ID:Copy> AttributeSlice<'l, T, ID> {
    pub fn new(slice: &'l[T]) -> AttributeSlice<'l, T, ID> {
        AttributeSlice {
            slice: slice,
            _idtype: PhantomData
        }
    }
}

impl<'l, T:'l, ID:Copy> ops::Index<Id<ID>> for AttributeSlice<'l, T, ID> {
    type Output = T;
    fn index<'a>(&'a self, id: &Id<ID>) -> &'a T { &self.slice[id.as_index()] }
}


pub struct MutAttributeSlice<'l, T:'l, ID:Copy> {
    slice: &'l mut[T],
    _idtype: PhantomData<ID>
}

impl<'l, T:'l, ID:Copy> MutAttributeSlice<'l, T, ID> {
    pub fn new(slice: &'l mut[T]) -> MutAttributeSlice<'l, T, ID> {
        MutAttributeSlice {
            slice: slice,
            _idtype: PhantomData
        }
    }
}

impl<'l, T:'l, ID:Copy> ops::Index<Id<ID>> for MutAttributeSlice<'l, T, ID> {
    type Output = T;
    fn index<'a>(&'a self, id: &Id<ID>) -> &'a T { &self.slice[id.as_index()] }
}

impl<'l, T:'l, ID:Copy> ops::IndexMut<Id<ID>> for MutAttributeSlice<'l, T, ID> {
    fn index_mut<'a>(&'a mut self, id: &Id<ID>) -> &'a mut T { &mut self.slice[id.as_index()] }
}

