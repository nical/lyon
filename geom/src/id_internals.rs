use vodk_id::Id;

pub type Index = u16;

// We use a magic value to 
pub fn is_valid<T>(id: Id<T, Index>) -> bool { id.handle != ::std::u16::MAX }
