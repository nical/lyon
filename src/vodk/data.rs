
pub type Type = u32;

pub static BASE_TYPE_MASK: Type = 0xFF;
pub static FLOAT:   Type = 1 << 0;
pub static INT:     Type = 1 << 1;
pub static UNSIGNED:Type = 1 << 2;
pub static BOOL:    Type = 1 << 3;
pub static STRING:  Type = 1 << 4;

pub static TYPE_SIZE_MASK: Type = 0xFF00;
pub static SIZE_8:  Type = 1 << 8;
pub static SIZE_16: Type = 1 << 9;
pub static SIZE_32: Type = 1 << 10;
pub static SIZE_64: Type = 1 << 11;
pub static SIZE_128:Type = 1 << 12;

pub static NUM_COMPONENTS_MASK: Type = 0xFF0000;
pub static X2:      Type = 1 << 16;
pub static X3:      Type = 1 << 17;
pub static X4:      Type = 1 << 18;
pub static X8:      Type = 1 << 19;
pub static X16:     Type = 1 << 20;

pub static F32:     Type = FLOAT | SIZE_32;
pub static F64:     Type = FLOAT | SIZE_64;
pub static I8:      Type = INT | SIZE_8;
pub static I16:     Type = INT | SIZE_16;
pub static I32:     Type = INT | SIZE_32;
pub static I64:     Type = INT | SIZE_64;
pub static U8:      Type = INT | UNSIGNED | SIZE_8;
pub static U16:     Type = INT | UNSIGNED | SIZE_16;
pub static U32:     Type = INT | UNSIGNED | SIZE_32;
pub static U64:     Type = INT | UNSIGNED | SIZE_64;
pub static VEC2:    Type = F32 | X2;
pub static VEC3:    Type = F32 | X3;
pub static VEC4:    Type = F32 | X4;
pub static MAT4:    Type = F32 | X16;

