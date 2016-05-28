use vodk_id::{ Id, FromIndex };
use vodk_id::id_list::IdCheck;
use std::marker::PhantomData;

pub type Index = u16;

// We use a magic value to
pub fn is_valid<T>(id: Id<T, Index>) -> bool { id.handle != ::std::u16::MAX }

pub struct MagicValueMax<T> {
    _marker: PhantomData<T>,
}

impl<T> IdCheck<Id<T, Index>> for MagicValueMax<T> {
    fn none() -> Id<T, Index> { return FromIndex::from_index(::std::u16::MAX as usize); }
}
