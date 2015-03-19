pub type Type = u32;

pub const BASE_TYPE_MASK: Type = 0xFF;
pub const FLOAT:   Type = 1 << 0;
pub const INT:     Type = 1 << 1;
pub const UNSIGNED:Type = 1 << 2;
pub const BOOL:    Type = 1 << 3;
pub const STRING:  Type = 1 << 4;

pub const TYPE_SIZE_MASK: Type = 0xFF00;
pub const SIZE_8:  Type = 8   << 8;
pub const SIZE_16: Type = 16  << 8;
pub const SIZE_32: Type = 32  << 8;
pub const SIZE_64: Type = 64  << 8;
pub const SIZE_128:Type = 128 << 8;

pub const NUM_COMPONENTS_MASK: Type = 0xFF0000;
pub const X2:      Type = 1 << 16;
pub const X3:      Type = 2 << 16;
pub const X4:      Type = 3 << 16;
pub const X8:      Type = 7 << 16;
pub const X16:     Type = 16 << 16;

pub const F32:     Type = FLOAT | SIZE_32;
pub const F64:     Type = FLOAT | SIZE_64;
pub const I8:      Type = INT | SIZE_8;
pub const I16:     Type = INT | SIZE_16;
pub const I32:     Type = INT | SIZE_32;
pub const I64:     Type = INT | SIZE_64;
pub const U8:      Type = INT | UNSIGNED | SIZE_8;
pub const U16:     Type = INT | UNSIGNED | SIZE_16;
pub const U32:     Type = INT | UNSIGNED | SIZE_32;
pub const U64:     Type = INT | UNSIGNED | SIZE_64;
pub const VEC2:    Type = F32 | X2;
pub const VEC3:    Type = F32 | X3;
pub const VEC4:    Type = F32 | X4;
pub const MAT4:    Type = F32 | X16;

pub fn base_type(t: Type) -> Type { t & BASE_TYPE_MASK }

pub fn scalar_type_of(t: Type) -> Type { t & (BASE_TYPE_MASK|TYPE_SIZE_MASK) }

pub fn num_components(t: Type) -> u32 { ((t & NUM_COMPONENTS_MASK) >> 16) + 1}

pub fn size_of(t: Type) -> u32 { num_components(t) * ((t & TYPE_SIZE_MASK) >> 8) }
