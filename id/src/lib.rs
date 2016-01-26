use std::marker::PhantomData;
use std::ops::Add;
use std::hash::{ Hash, Hasher };

pub mod id_vector;
pub mod sparse_id_vector;
pub mod id_list;


// --------------------------------------------------------------------------------------------- Id

pub struct Id<T, H> {
    pub handle: H,
    pub _marker: PhantomData<T>
}

impl<T, H: std::fmt::Display> std::fmt::Debug for Id<T, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Id#{}", self.handle)
    }
}

impl<T, H:Copy> Copy for Id<T, H> {}

impl<T, H:Copy> Clone for Id<T, H> { fn clone(&self) -> Id<T, H> { *self } }

impl<T, H:PartialEq> PartialEq for Id<T, H> {
    fn eq(&self, other: &Id<T,H>) -> bool { self.handle.eq(&other.handle) }
}

impl<T, H:Copy+Eq> Eq for Id<T, H> {}

impl<T, H:Copy> Id<T, H> {
    pub fn new(idx: H) -> Id<T, H> { Id { handle: idx, _marker: PhantomData } }
}

impl<T, H:IntegerHandle> Identifier for Id<T, H> {
    type Handle = H;
}

impl<T, H:ToIndex> ToIndex for Id<T, H> {
    fn to_index(&self) -> usize { self.handle.to_index() }
}

impl<T, H:Copy+FromIndex> FromIndex for Id<T, H> {
    fn from_index(idx: usize) -> Id<T, H> { Id::new(FromIndex::from_index(idx)) }
}

impl<T, Handle: Hash> Hash for Id<T, Handle> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}


// ---------------------------------------------------------------------------------------- IdRange

pub struct IdRange<T, H> {
    pub first: Id<T, H>,
    pub count: H,
}

impl<T, H: std::fmt::Display> std::fmt::Debug for IdRange<T, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Id#[{}..{}]", self.first.handle, self.count)
    }
}

impl<T, H:Copy> Copy for IdRange<T, H> {}

impl<T, H:Copy> Clone for IdRange<T, H> { fn clone(&self) -> IdRange<T, H> { *self } }

impl<T, H:PartialEq> PartialEq for IdRange<T, H> {
    fn eq(&self, other: &IdRange<T,H>) -> bool { self.first.eq(&other.first) && self.count.eq(&other.count) }
}

impl<T, H:Copy+Eq> Eq for IdRange<T, H> {}

impl<T, H:IntegerHandle> IdRange<T, H> {
    pub fn len(self) -> usize { self.count.to_index() }

    pub fn is_empty(self) -> bool { self.len() == 0 }

    pub fn get(self, i: H) -> Id<T, H> {
        debug_assert!(i < self.count);
        return Id { handle: self.first.handle + i, _marker: PhantomData };
    }

    /// Return a range with the front element popped, or None if the range is empty.
    pub fn shrinked_left(self) -> Option<IdRange<T, H>> {
        if self.is_empty() {
            return None;
        }
        return Some(IdRange{
            count: FromIndex::from_index(self.count.to_index() - 1),
            first: FromIndex::from_index(self.first.to_index() + 1),
        });
    }

    /// Return a range with the back element popped, or None if the range is empty.
    pub fn shrinked_right(self) -> Option<IdRange<T, H>> {
        if self.is_empty() {
            return None;
        }
        return Some(IdRange{
            first: self.first,
            count: FromIndex::from_index(self.count.to_index() - 1),
        });
    }
}

impl<T, H:IntegerHandle> Iterator for IdRange<T, H> {
    type Item = Id<T, H>;
    fn next(&mut self) -> Option<Id<T, H>> {
        if self.count.to_index() == 0 {
            return None;
        }
        let res = self.first;
        self.count = FromIndex::from_index(self.count.to_index() - 1);
        self.first = FromIndex::from_index(self.first.to_index() + 1);
        return Some(res);
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        return (self.count.to_index(), Some(self.count.to_index()));
    }

    fn count(self) -> usize { self.count.to_index() }
}

pub struct ReverseIdRange<T, H> {
    range: IdRange<T, H>,
}

impl<T, H: std::fmt::Display> std::fmt::Debug for ReverseIdRange<T, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ReverseId#[{}..{}]", self.range.first.handle, self.range.count)
    }
}

impl<T, H:Copy> Copy for ReverseIdRange<T, H> {}

impl<T, H:Copy> Clone for ReverseIdRange<T, H> { fn clone(&self) -> ReverseIdRange<T, H> { *self } }

impl<T, H:PartialEq> PartialEq for ReverseIdRange<T, H> {
    fn eq(&self, other: &ReverseIdRange<T,H>) -> bool { self.range.eq(&other.range) }
}

impl<T, H:Copy+Eq> Eq for ReverseIdRange<T, H> {}

impl<T, H:IntegerHandle> ReverseIdRange<T, H> {

    pub fn new(range: IdRange<T, H>) -> ReverseIdRange<T, H> { ReverseIdRange { range: range } }

    pub fn len(&self) -> usize { self.range.len() }

    pub fn is_empty(self) -> bool { self.len() == 0 }

    pub fn get(self, i: H) -> Id<T, H> { self.range.get(i) }
}

impl<T, H:IntegerHandle> Iterator for ReverseIdRange<T, H> {
    type Item = Id<T, H>;
    fn next(&mut self) -> Option<Id<T, H>> {
        if self.range.count.to_index() == 0 {
            return None;
        }
        self.range.count = FromIndex::from_index(self.range.count.to_index() - 1);
        return Some(FromIndex::from_index(self.range.first.to_index() + self.range.count.to_index()));
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }

    fn count(self) -> usize { self.range.count() }
}



// ------------------------------------------------------------------------------------------ GenId
// TODO: remove it or implement traits manually

#[derive(Copy, Clone)]
pub struct GenId<T, H:Copy, G> {
    pub id: Id<T, H>,
    pub gen: G,
}

impl<T, H: Copy+std::fmt::Display, G: std::fmt::Display> std::fmt::Debug for GenId<T, H, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "GenId#{}({})", self.id.handle, self.gen)
    }
}

impl<T, H:IntegerHandle, G: PartialEq> PartialEq for GenId<T, H, G> {
    fn eq(&self, other: &GenId<T, H, G>) -> bool { self.id == other.id && self.gen == other.gen }
}

impl<T, H:IntegerHandle, G> ToIndex for GenId<T, H, G> {
    fn to_index(&self) -> usize { self.id.to_index() }
}


// ----------------------------------------------------------------------------------------- traits

pub trait FromIndex {  fn from_index(idx: usize) -> Self; }

pub trait ToIndex { fn to_index(&self) -> usize; }

pub trait Generation { fn get_gen(&self) -> u32; }

pub trait IntegerHandle : Copy + Clone
                        + Add<Output=Self>
                        + PartialEq + PartialOrd
                        + FromIndex + ToIndex {}

pub trait Identifier: Copy + FromIndex + ToIndex + PartialEq {
    type Handle: IntegerHandle;
}


impl ToIndex for u8 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u16 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u32 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for u64 { fn to_index(&self) -> usize { *self as usize } }
impl ToIndex for usize { fn to_index(&self) -> usize { *self } }

impl FromIndex for u8 { fn from_index(idx: usize) -> u8 { idx as u8 } }
impl FromIndex for u16 { fn from_index(idx: usize) -> u16 { idx as u16 } }
impl FromIndex for u32 { fn from_index(idx: usize) -> u32 { idx as u32 } }
impl FromIndex for u64 { fn from_index(idx: usize) -> u64 { idx as u64 } }
impl FromIndex for usize { fn from_index(idx: usize) -> usize { idx } }

impl Generation for u8  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u16  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u32  { fn get_gen(&self) -> u32 { *self as u32 } }
impl Generation for u64  { fn get_gen(&self) -> u32 { *self as u32 } }

impl<T, H:Copy, G:Generation> Generation for GenId<T, H, G>  {
    fn get_gen(&self) -> u32 { self.gen.get_gen() }
}

impl IntegerHandle for u8 {}
impl IntegerHandle for u16 {}
impl IntegerHandle for u32 {}
impl IntegerHandle for u64 {}
impl IntegerHandle for usize {}


// ------------------------------------------------------------------------------------------ tests

#[test]
fn test_copy_id() {
    #[derive(Debug)]
    struct Foo;

    // check that Id is Copy
    let a: Id<Foo, u32> = Id::new(0);
    let b = a;
    let c = a;
    assert_eq!(b, c);

    // check that IdRange is Copy
    let r1 = IdRange {
        first: a,
        count: 10,
    };

    let r2 = r1;
    let r3 = r1;
    assert_eq!(r2, r3);
}
