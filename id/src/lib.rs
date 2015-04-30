extern crate num;

use std::marker::PhantomData;
use num::traits::{ One, Zero, Bounded };
use std::ops::{ Add, Sub };

pub mod id_vector;
pub mod id_list;

pub trait FromIndex {
    fn from_index(idx: usize) -> Self;
}

pub trait ToIndex {
    fn to_index(&self) -> usize;
}

pub trait Generation {
    fn get_gen(&self) -> u32;
}

pub trait IntegerHandle : Copy
                        + Sub<Output=Self>
                        + Add<Output=Self>
                        + PartialEq + PartialOrd
                        + Bounded
                        + FromIndex + ToIndex {}

pub trait Identifier: Copy + FromIndex + ToIndex + PartialEq {
    type Handle: IntegerHandle;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Id<T, H> {
    pub handle: H,
    pub _marker: PhantomData<T>
}

#[derive(Copy, Clone, Debug)]
pub struct GenId<T, H, G> {
    pub handle: H,
    pub gen: G,
    pub _marker: PhantomData<T>
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IdRange<T, H> {
    pub first: Id<T, H>,
    pub count: H,
}

#[derive(Clone)]
pub struct IdRangeIterator<T, H> {
    range: IdRange<T, H>
}

impl<T: Copy, H: IntegerHandle> IdRange<T, H> {
    pub fn iter(self) -> IdRangeIterator<T, H> {
        return IdRangeIterator::new(self);
    }

    pub fn get(self, i: H) -> Id<T, H> {
        debug_assert!(i < self.count);
        return Id { handle: self.first.handle + i, _marker: PhantomData };
    }
}

impl<T:Copy, H:IntegerHandle+One+Zero> Iterator for IdRangeIterator<T, H> {
    type Item = Id<T, H>;
    fn next(&mut self) -> Option<Id<T, H>> {
        if self.range.count == num::zero() {
            return None;
        }
        let res = self.range.first;
        self.range.count = self.range.count - num::one();
        self.range.first = FromIndex::from_index(self.range.first.to_index() + 1);
        return Some(res);
    }
}

impl<T:Copy, H:Copy> IdRangeIterator<T, H> {
    pub fn new(range: IdRange<T, H>) -> IdRangeIterator<T, H> {
        IdRangeIterator { range: range }
    }
}

impl<T:Copy, H:Copy> Id<T, H> {
    pub fn new(idx: H) -> Id<T, H> { Id { handle: idx, _marker: PhantomData } }
}

impl<T: Copy+PartialEq, H: IntegerHandle> Identifier for Id<T, H> {
    type Handle = H;
}

impl<T, H: PartialEq, G: PartialEq> PartialEq for GenId<T, H, G> {
    fn eq(&self, other: &GenId<T, H, G>) -> bool { self.handle == other.handle && self.gen == other.gen }
    fn ne(&self, other: &GenId<T, H, G>) -> bool { self.handle != other.handle || self.gen != other.gen }
}

impl<T, H:IntegerHandle, G> ToIndex for GenId<T, H, G> {
    fn to_index(&self) -> usize { self.handle.to_index() }
}

impl ToIndex for u8 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u16 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u32 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u64 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for usize { fn to_index(&self) -> usize { *self } }

impl<T: Copy, H:IntegerHandle> ToIndex for Id<T, H> {
    fn to_index(&self) -> usize { self.handle.to_index() }
}

impl FromIndex for u8 { fn from_index(idx: usize) -> u8 { idx as u8 } }
impl FromIndex for u16 { fn from_index(idx: usize) -> u16 { idx as u16 } }
impl FromIndex for u32 { fn from_index(idx: usize) -> u32 { idx as u32 } }
impl FromIndex for u64 { fn from_index(idx: usize) -> u64 { idx as u64 } }
impl FromIndex for usize { fn from_index(idx: usize) -> usize { idx } }

impl<T: Copy, H:IntegerHandle> FromIndex for Id<T, H> {
    fn from_index(idx: usize) -> Id<T, H> { Id::new(FromIndex::from_index(idx)) }
}

impl Generation for u8  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u16  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u32  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u64  { fn get_gen(&self) -> u32 { *self as u32 } }

impl<T, H, G: Generation> Generation for GenId<T, H, G>  { fn get_gen(&self) -> u32 { self.gen.get_gen() } }

impl IntegerHandle for u8 {}
impl IntegerHandle for u16 {}
impl IntegerHandle for u32 {}
impl IntegerHandle for u64 {}
impl IntegerHandle for usize {}

