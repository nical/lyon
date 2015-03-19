use std::mem;

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

// TODO:
// pub fn size_of_struct(t: &[Type]) -> u32

pub fn type_of<T: DataType>() -> Type {
    let wrapper: TypeResult<T> = DataType::data_type();
    return wrapper.data_type;
}

pub fn struct_type_of<T: StructDataType>() -> &'static[Type] {
    let wrapper: StructTypeResult<T> = StructDataType::data_type();
    return wrapper.data_type;
}

pub trait DataType {
    fn data_type() -> TypeResult<Self>;
}
pub struct TypeResult<T> { pub data_type: Type }

pub trait StructDataType {
    fn data_type() -> StructTypeResult<Self>;
}
pub struct StructTypeResult<T> { pub data_type: &'static[Type] }


const f32_data_type: &'static[Type] = &[F32];
const u32_data_type: &'static[Type] = &[U32];
const i32_data_type: &'static[Type] = &[I32];
const u16_data_type: &'static[Type] = &[U16];
const i16_data_type: &'static[Type] = &[I16];
const u8_data_type:  &'static[Type] = &[U8];
const i8_data_type:  &'static[Type] = &[I8];

impl StructDataType for f32 { fn data_type() -> StructTypeResult<f32> { StructTypeResult{data_type: f32_data_type } } }
impl StructDataType for u32 { fn data_type() -> StructTypeResult<u32> { StructTypeResult{data_type: u32_data_type } } }
impl StructDataType for i32 { fn data_type() -> StructTypeResult<i32> { StructTypeResult{data_type: i32_data_type } } }
impl StructDataType for u16 { fn data_type() -> StructTypeResult<u16> { StructTypeResult{data_type: u16_data_type } } }
impl StructDataType for i16 { fn data_type() -> StructTypeResult<i16> { StructTypeResult{data_type: i16_data_type } } }
impl StructDataType for u8  { fn data_type() -> StructTypeResult<u8>  { StructTypeResult{data_type: u8_data_type } } }
impl StructDataType for i8  { fn data_type() -> StructTypeResult<i8>  { StructTypeResult{data_type: i8_data_type } } }
impl DataType for f32 { fn data_type() -> TypeResult<f32> { TypeResult{data_type: F32 } } }
impl DataType for u32 { fn data_type() -> TypeResult<u32> { TypeResult{data_type: U32 } } }
impl DataType for i32 { fn data_type() -> TypeResult<i32> { TypeResult{data_type: I32 } } }
impl DataType for u16 { fn data_type() -> TypeResult<u16> { TypeResult{data_type: U16 } } }
impl DataType for i16 { fn data_type() -> TypeResult<i16> { TypeResult{data_type: I16 } } }
impl DataType for u8  { fn data_type() -> TypeResult<u8>  { TypeResult{data_type: U8 } } }
impl DataType for i8  { fn data_type() -> TypeResult<i8>  { TypeResult{data_type: I8 } } }

pub struct DynamicallyTypedSlice<'l> {
    data: &'l mut[u8],
    desc: &'l[Type],
    stride: usize,
}

impl<'l> DynamicallyTypedSlice<'l> {
    pub fn new<T>(data: &'l[T], desc: &'l[Type]) -> DynamicallyTypedSlice<'l> {
        DynamicallyTypedSlice {
            data: unsafe {
                mem::transmute((
                    data.as_ptr() as *mut T,
                    data.len() * mem::size_of::<T>()
                ))
            },
            desc: desc,
            stride: mem::size_of::<T>(),
        }
    }

    pub fn from_slice<T: StructDataType>(data: &'l[T]) -> DynamicallyTypedSlice<'l> {
        DynamicallyTypedSlice {
            data: unsafe {
                mem::transmute((
                    data.as_ptr() as *mut T,
                    data.len() * mem::size_of::<T>()
                ))
            },
            desc: struct_type_of::<T>(),
            stride: mem::size_of::<T>(),
        }
    }

    pub fn get_type<'l>(&'l self) -> &'l[Type] {
        self.desc.as_slice()
    }

    pub fn as_slice<T>(&'l self) -> &'l[T] {
        assert!(mem::size_of::<T>() == self.stride);
        return unsafe {
            mem::transmute((
                self.data.as_ptr() as *const T,
                self.data.len() / self.stride,
            ))
        };
    }

    pub fn as_mut_slice<T>(&'l mut self) -> &'l mut[T] {
        assert!(mem::size_of::<T>() == self.stride);
        return unsafe {
            mem::transmute((
                self.data.as_ptr() as *mut T,
                self.data.len() / self.stride,
            ))
        };
    }

    pub fn as_byte_slice(&'l self) -> &'l[u8] {
        return self.data.as_slice();
    }

    pub fn as_mut_byte_slice(&'l mut self) -> &'l mut[u8] {
        return self.data.as_mut_slice();
    }

    pub fn len(&self) -> usize { self.data.len() / self. stride as usize }

    pub fn byte_len(&self) -> usize { self.data.len() }
}
