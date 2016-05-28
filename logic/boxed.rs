use std::rc::Rc;
use std::slice;
use std::mem;
use std::fmt;
use std::ops;
use std::marker::PhantomData;
use libc::funcs::c95::stdlib::{ malloc, free };

type Mask = u64;
const DATA_32_MASK:      Mask = 0b0000000000000000000000000000000011111111111111111111111111111111;
const PTR_DATA_MASK:     Mask = 0b0000000000000000011111111111111111111111111111111111111111111111;
const TAG_16_MASK:       Mask = 0b1111111111111111000000000000000000000000000000000000000000000000;
const TYPE_MASK:         Mask = 0b1111111100000000000000000000000000000000000000000000000000000000;
const OWNED_BIT:         Mask = 0b0000000010000000000000000000000000000000000000000000000000000000;
const PTR_TYPE_BITS:     Mask = 0b1000000000000000000000000000000000000000000000000000000000000000;
const STRUCT_TYPE_BITS:  Mask = 0b1100000000000000000000000000000000000000000000000000000000000000;
const ARRAY_TYPE_BITS:   Mask = 0b1010000000000000000000000000000000000000000000000000000000000000;
const MAP_TYPE_BITS:     Mask = 0b1001000000000000000000000000000000000000000000000000000000000000;
const STR_TYPE_BITS:     Mask = 0b1000100000000000000000000000000000000000000000000000000000000000;
const FLOAT32_TYPE_BITS: Mask = 0b0000010000000000000000000000000000000000000000000000000000000000;
const INT32_TYPE_BITS:   Mask = 0b0000001000000000000000000000000000000000000000000000000000000000;

pub type ValueType = u8;
pub const VOID_VALUE:       ValueType = 0;
pub const FLOAT32_VALUE:    ValueType = (FLOAT32_TYPE_BITS >> 56) as u8;
pub const INT32_VALUE:      ValueType = (INT32_TYPE_BITS >> 56) as u8;
pub const POINTER_VALUE:    ValueType = (PTR_TYPE_BITS >> 56) as u8;
pub const ARRAY_PTR:        ValueType = (ARRAY_TYPE_BITS >> 56) as u8;
pub const STRUCT_PTR:       ValueType = (STRUCT_TYPE_BITS >> 56) as u8;
pub const STRING_PTR:       ValueType = (STR_TYPE_BITS >> 56) as u8;

pub const VOID: u64 = 0;

#[repr(C)]
#[derive(PartialEq)]
pub struct TaggedPtr<T> {
    payload: u64,
    _phantom: PhantomData<T>
}

impl<T> TaggedPtr<T> {
    pub fn new(ptr: *mut T, tag: u16) -> TaggedPtr<T> {
        unsafe {
            let cast: u64 = mem::transmute(ptr);
            return TaggedPtr {
                payload: cast | (tag as u64) << 48,
                _phantom: PhantomData
            }
        }
    }

    pub fn get(&self) -> &T { unsafe { mem::transmute(self.payload & PTR_DATA_MASK) } }
    pub fn tag(&self) -> u16 { unsafe { ((self.payload & TAG_16_MASK) >> 48) as u16 } }
}

#[repr(C)]
#[derive(Copy, PartialEq)]
pub struct Value {
    payload: u64,
}

/// An owned vector of Value objects
pub struct Array {
    data: *mut ArrayData,
}

/// Header of an array's buffer. The payload is placed directly after in memory.
pub struct ArrayData {
    len : u32,
    cap : u32,
    type_info: u64,
}

/// An owned Reference to a run-time defined structure based on Value objects.
pub struct Struct {
    data: *mut StructData,
}

/// Header of an structure's buffer. The payload is placed directly after in memory.
pub struct StructData {
    type_info: Rc<StructTypeInfo>
}

pub type StructMemberId = u32;

pub struct StructMemberInfo {
    name: Option<String>,
    value_type: Option<ValueType>, // TODO - richer type info like TypeInfo*
    default_value: Option<Value>,
}

pub struct StructTypeInfo {
    members: Vec<StructMemberInfo>,
}

impl Array {
    pub fn new(cap: u32) -> Array { Array { data: ArrayData::allocate(cap) } }

    pub fn len(&self) -> u32 { self.data().len }

    pub fn get(&self, idx: u32) -> &Value { self.data().get(idx) }

    pub fn get_mut(&mut self, idx: u32) -> &mut Value { self.mut_data().get_mut(idx) }

    pub fn push(&mut self, value: Value) -> u32 { self.mut_data().push(value) }

    pub fn into_value(self) -> Value {
        unsafe {
            let cast: u64 = mem::transmute(self.data);
            let val = Value {
                payload: cast | ARRAY_TYPE_BITS | OWNED_BIT
            };
            mem::forget(self);
            return val;
        }
    }

    fn data(&self) -> &ArrayData { unsafe { mem::transmute(self.data) } }
    fn mut_data(&mut self) -> &mut ArrayData { unsafe { mem::transmute(self.data) } }
}

impl Drop for Array {
    fn drop(&mut self) {
        ArrayData::deallocate(self.data);
    }
}

impl Struct {
    pub fn new(type_info: Rc<StructTypeInfo>) -> Struct {
        Struct { data: StructData::allocate(type_info) }
    }

    pub fn get(&self, id: StructMemberId) -> &Value { self.data().get(id) }

    pub fn set(&mut self, id: StructMemberId, val: Value) { self.mut_data().set(id, val); }

    pub fn type_info(&self) -> &StructTypeInfo { self.data().type_info() }

    pub fn into_value(self) -> Value {
        unsafe {
            let cast: u64 = mem::transmute(self.data);
            let val = Value {
                payload: cast | STRUCT_TYPE_BITS | OWNED_BIT
            };
            mem::forget(self);
            return val;
        }
    }

    fn data(&self) -> &StructData { unsafe { mem::transmute(self.data) } }

    fn mut_data(&mut self) -> &mut StructData { unsafe { mem::transmute(self.data) } }
}

impl Drop for Struct {
    fn drop(&mut self) {
        StructData::deallocate(self.data);
    }
}

impl Value {
    pub fn void() -> Value {
        Value {
            payload: VOID
        }
    }

    pub fn int32(val: i32) -> Value {
        let data: u32 = unsafe { mem::transmute(val) };
        Value {
            payload: data as u64 | INT32_TYPE_BITS
        }
    }

    pub fn float32(val: f32) -> Value {
        let data: u32 = unsafe { mem::transmute(val) };
        Value {
            payload: data as u64 | FLOAT32_TYPE_BITS
        }
    }

    pub fn boolean(val: bool) -> Value {
        Value::int32(if val { 1 } else { 0 })
    }

    pub fn array(values: &[Value]) -> Value {
        let ptr = ArrayData::allocate(values.len() as u32);
        for &val in values {
            unsafe { (*ptr).push(val); }
        }
        let cast: u64 = unsafe { mem::transmute(ptr) };
        return Value {
            payload: cast | ARRAY_TYPE_BITS | OWNED_BIT
        }
    }

    pub fn default(typ: ValueType) -> Value {
        return match typ {
            FLOAT32_VALUE => Value::float32(0.0),
            INT32_VALUE => Value::int32(0),
            ARRAY_PTR => Value::array(&[]),
            STRUCT_PTR => Value::void(),
            _ => Value::void(),
        };
    }

    pub fn borrowed_ptr<T>(val: *mut T) -> Value {
        Value {
            payload: unsafe { mem::transmute(val) }
        }
    }

    pub fn owned_ptr<T>(val: *mut T) -> Value {
        let cast: u64 = unsafe { mem::transmute(val) };
        Value {
            payload: cast | OWNED_BIT
        }
    }

    pub fn get_type(&self) -> ValueType {
        return ((self.payload & TYPE_MASK) >> 56) as ValueType;
    }

    pub fn has_ownership(&self) -> bool {
        return self.payload & OWNED_BIT != 0;
    }

    pub fn is_float32(&self) -> bool {
        return self.payload & TYPE_MASK == FLOAT32_TYPE_BITS ;
    }

    pub fn is_int32(&self) -> bool {
        return self.payload & TYPE_MASK == INT32_TYPE_BITS ;
    }

    pub fn is_pointer(&self) -> bool {
        return self.payload & PTR_TYPE_BITS == PTR_TYPE_BITS;
    }

    pub fn is_array(&self) -> bool {
        return self.payload & TYPE_MASK == ARRAY_TYPE_BITS;
    }

    pub fn is_struct(&self) -> bool {
        return self.payload & TYPE_MASK == STRUCT_TYPE_BITS;
    }

    pub fn is_string(&self) -> bool {
        return self.payload & TYPE_MASK == STR_TYPE_BITS;
    }

    pub fn is_void(&self) -> bool {
        return self.payload == VOID;
    }

    unsafe fn get_array_unchecked(&self) -> *mut ArrayData {
        return mem::transmute(self.payload & PTR_DATA_MASK);
    }

    unsafe fn get_struct_unchecked(&self) -> *mut StructData {
        return mem::transmute(self.payload & PTR_DATA_MASK);
    }

    pub fn get_array(&self) -> Option<&ArrayData> {
        if !self.is_array() { return None; }
        return unsafe { Some(mem::transmute(self.get_array_unchecked())) }
    }

    pub fn get_mut_array(&mut self) -> Option<&ArrayData> {
        if !self.is_array() { return None; }
        return unsafe { Some(mem::transmute(self.get_array_unchecked())) }
    }

    pub unsafe fn get_pointer_unchecked<T>(&self) -> &T {
        return mem::transmute(self.payload & PTR_DATA_MASK);
    }

    pub unsafe fn get_float32_unchecked(&self) -> f32 {
        let data = (self.payload & DATA_32_MASK) as u32;
        return mem::transmute(data);
    }

    pub unsafe fn get_int32_unchecked(&self) -> i32 {
        let data = (self.payload & DATA_32_MASK) as u32;
        return mem::transmute(data);
    }

    pub fn get_pointer<'l, T>(&'l self) -> Option<&'l T> {
        if self.is_pointer() {
            return unsafe { Some(self.get_pointer_unchecked::<'l, T>()) };
        }
        return None;
    }

    pub fn get_float32(&self) -> Option<f32> {
        if self.is_float32() {
            return unsafe { Some(self.get_float32_unchecked()) };
        }
        if self.is_int32() {
            return unsafe { Some(self.get_int32_unchecked() as f32) };
        }
        return None;
    }

    pub fn get_int32(&self) -> Option<i32> {
        if self.is_int32() {
            return unsafe { Some(self.get_int32_unchecked()) };
        }
        return None;
    }

    pub fn get_bytes(&self) -> [u8; 8] {
        return unsafe { mem::transmute(self.payload) };
    }

    pub fn to_float32(&self) -> f32 {
        unsafe {
            return match self.get_type() {
                FLOAT32_VALUE => { self.get_float32_unchecked() }
                INT32_VALUE => { self.get_int32_unchecked() as f32 }
                _ => { 0.0f32 / 0.0f32 } // NaN
            }
        }
    }
    pub fn to_int32(&self) -> i32 {
        unsafe {
            return match self.get_type() {
                INT32_VALUE => { self.get_int32_unchecked() }
                FLOAT32_VALUE => { self.get_float32_unchecked() as i32 }
                _ => { 0 }
            }
        }
    }

    pub fn array_index(&self, idx: u32) -> Option<&Value> {
        if !self.is_array() {
            return None;
        }

        unsafe {
            return Some((*self.get_array_unchecked()).get(idx));
        }
    }

    pub fn array_len(&self) -> Option<u32> {
        if !self.is_array() {
            return None;
        }
        unsafe {
            return Some((*self.get_array_unchecked()).len());
        }
    }

    fn run_destructor(&mut self) {
        if !self.has_ownership() {
            return;
        }

        if self.is_array() {
            unsafe {
                ArrayData::deallocate(self.get_array_unchecked());
            }
        } else if self.is_struct() {
            unsafe {
                StructData::deallocate(self.get_struct_unchecked());
            }
        }

        self.payload = VOID;
    }
}

impl Clone for Value {
    fn clone(&self) -> Value {
        if self.payload & PTR_TYPE_BITS == 0 {
            return Value {
                payload: self.payload
            };
        }

        if self.is_array() {
            unsafe {
                let cast: u64 = mem::transmute(ArrayData::clone(self.get_array_unchecked()));
                return Value {
                    payload: cast | ARRAY_TYPE_BITS | OWNED_BIT
                };
            }
        }

        panic!("TODO");
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("boxed::Value{ ");
        unsafe {
            match self.get_type() {
                FLOAT32_VALUE => {
                    // TODO
                    //fmt.write_le_f32(self.get_float32_unchecked());
                    fmt.write_str(" float32 }");
                }
                INT32_VALUE => {
                    // TODO
                    //fmt.write_int(self.get_int32_unchecked() as isize);
                    fmt.write_str(" int32 }");
                }
                VOID_VALUE => {
                    fmt.write_str(" void }");
                }
                STRUCT_PTR => {
                    fmt.write_str("*struct }");
                }
                ARRAY_PTR => {
                    fmt.write_str("*array }");
                }
                _ => {
                    fmt.write_str("boxed::Value{ ? }");
                }
            }
        }
        return Ok(());
    }
}

impl ArrayData {
    pub fn len(&self) -> u32 { self.len }

    fn Capacity(&self) -> u32 { self.cap }

    pub fn get_slice(&self) -> &[Value] {
        unsafe {
            let self_ptr: *const ArrayData = self;
            let ptr: *const Value = mem::transmute(self_ptr.offset(1));
            return slice::from_raw_parts(ptr, self.len as usize);
        }
    }

    pub fn get_mut_slice(&mut self) -> &mut[Value] {
        unsafe {
            let self_ptr: *mut ArrayData = self;
            let ptr: *mut Value = mem::transmute(self_ptr.offset(1));
            return slice::from_raw_parts_mut(ptr, self.len as usize);
        }
    }

    pub fn get(&self, idx: u32) -> &Value {
        return &self.get_slice()[idx as usize];
    }

    pub fn get_mut(&mut self, idx: u32) -> &mut Value {
        return &mut self.get_mut_slice()[idx as usize];
    }

    pub fn push(&mut self, val: Value) -> u32 {
        if self.cap - self.len > 0 {
            let idx = self.len;
            unsafe {
                *self.unsafe_get_mut(idx) = val
            }
            self.len += 1;
            return idx;
        }
        panic!("TODO");
    }

    unsafe fn unsafe_get_mut(&mut self, idx: u32) -> &mut Value {
        let self_ptr: *mut ArrayData = self;
        unsafe {
            let first_ptr: *mut Value = mem::transmute(self_ptr.offset(1));
            return mem::transmute(first_ptr.offset(idx as isize));
        }
    }

    fn allocate(cap: u32) -> *mut ArrayData {
        unsafe {
            let ptr = malloc((cap as usize * mem::size_of::<Value>() + mem::size_of::<ArrayData>()) as u64);
            let header_ptr: *mut ArrayData = mem::transmute(ptr);
            *header_ptr = ArrayData {
                len: 0, cap: cap, type_info: 0,
            };
            return header_ptr;
        }
    }

    fn deallocate(array: *mut ArrayData) {
        unsafe {
            for val in (*array).get_mut_slice() {
                val.run_destructor();
            }
            free(mem::transmute(array));
        }
    }

    fn clone(array: *mut ArrayData) -> *mut ArrayData {
        unsafe {
            let new_array = ArrayData::allocate((*array).len);
            for val in (*array).get_slice() {
                (*new_array).push(val.clone());
            }
            return new_array;
        }
    }
}

impl ops::Index<u32> for Array {
    type Output = Value;
    fn index<'l>(&'l self, id: u32) -> &'l Value { self.data().get(id) }
}

impl ops::IndexMut<u32> for Array {
    fn index_mut<'l>(&'l mut self, id: u32) -> &'l mut Value { self.mut_data().get_mut(id) }
}


impl StructData {
    pub fn members(&self) -> &[Value] {
        unsafe {
            return slice::from_raw_parts(self.payload(), self.type_info.members.len());
        }
    }

    fn mut_members(&mut self) -> &mut[Value] {
        unsafe {
            return slice::from_raw_parts_mut(self.mut_payload(), self.type_info.members.len());
        }
    }

    pub fn get(&self, id: StructMemberId) -> &Value {
        if id as usize >= self.type_info.members.len() {
            panic!("Struct member index out of bounds.");
        }
        unsafe {
            return mem::transmute(self.payload().offset(id as isize))
        }
    }

    pub fn set(&mut self, id: StructMemberId, val: Value) {
        if id as usize >= self.type_info.members.len() {
            panic!("Struct member index out of bounds.");
        }
        if val.get_type() != VOID_VALUE {
            if let Some(ty) = self.type_info.members[id as usize].value_type {
                if ty != val.get_type() {
                    panic!("Incompatible type in struct member assignment. Got {} expected {}", val.get_type(), ty);
                }
            }
        }
        unsafe {
            *self.mut_payload().offset(id as isize) = val;
        }
    }

    unsafe fn unsafe_get_mut(&mut self, idx: u32) -> &mut Value {
        return mem::transmute(self.mut_payload().offset(idx as isize));
    }

    unsafe fn payload(&self) -> *const Value {
        let self_ptr: *const StructData = self;
        return mem::transmute(self_ptr.offset(1));
    }

    unsafe fn mut_payload(&mut self) -> *mut Value {
        let self_ptr: *mut StructData = self;
        return mem::transmute(self_ptr.offset(1));
    }

    pub fn type_info(&self) -> &StructTypeInfo { &*self.type_info }

    fn allocate(ti: Rc<StructTypeInfo>) -> *mut StructData {
        unsafe {
            let ptr = malloc((ti.members.len() * mem::size_of::<Value>() + mem::size_of::<StructData>()) as u64);
            let header_ptr: *mut StructData = mem::transmute(ptr);
            *header_ptr = StructData {
                type_info: ti,
            };
            let mut i: isize = 0;
            for ref m in (*header_ptr).type_info().members.iter() {
                *(*header_ptr).mut_payload().offset(i) =
                    if let Some(ref val) = m.default_value { *val }
                    else { Value::void() };
                i += 1;
            }
            return header_ptr;
        }
    }

    fn deallocate(structure: *mut StructData) {
        unsafe {
            for val in (*structure).mut_members() {
                val.run_destructor();
            }
            free(mem::transmute(structure));
        }
    }
}

impl StructTypeInfo {
    pub fn member(&self, m: StructMemberId) -> &StructMemberInfo { &self.members[m as usize] }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::rc::Rc;

    fn fuzzy_eq(a: f32, b: f32) -> bool { return (a - b).abs() < 0.000001; }

    #[test]
    fn int_value() {
        let val = -0xeffffff as i32;
        let a = Value::int32(val);
        assert_eq!(a.get_type(), INT32_VALUE);
        assert!(a.is_int32());
        assert!(!a.is_float32());
        assert!(!a.is_void());
        assert!(!a.is_pointer());
        assert_eq!(a.get_int32().unwrap(), val);
    }

    #[test]
    fn float_value() {
        let a = Value::float32(42.0);
        assert_eq!(a.get_type(), FLOAT32_VALUE);
        assert!(a.is_float32());
        assert!(!a.is_int32());
        assert!(!a.is_void());
        assert!(!a.is_pointer());
        assert!(fuzzy_eq(a.get_float32().unwrap(), 42.0));
    }

    #[test]
    fn void_value() {
        let a = Value::void();
        assert_eq!(a, Value::void());
        assert_eq!(a.get_type(), VOID_VALUE);
        assert!(a.is_void());
        assert!(!a.is_float32());
        assert!(!a.is_int32());
        assert!(!a.is_pointer());
    }

    #[test]
    fn array_value() {
        let mut a = Value::array(&[
            Value::int32(0),
            Value::int32(1),
            Value::int32(2),
            Value::int32(3),
            Value::array(&[
                Value::int32(0),
                Value::int32(1),
                Value::int32(2),
                Value::int32(3),
            ]),
        ]);

        assert_eq!(a.get_array().unwrap().len(), 5);
        assert_eq!(a.get_array().unwrap().len(), 5);
        assert_eq!(a.get_array().unwrap().get(0), &Value::int32(0));
        assert_eq!(a.get_array().unwrap().get(1), &Value::int32(1));
        assert_eq!(a.get_array().unwrap().get(2), &Value::int32(2));
        assert_eq!(a.get_array().unwrap().get(3), &Value::int32(3));

        a.run_destructor();

        let mut arr = Array::new(10);
        arr.push(Value::int32(0));
        arr.push(Value::int32(1));
        arr.push(Value::int32(2));
        arr.push(Value::int32(3));

        assert_eq!(arr[0], Value::int32(0));
    }

    #[test]
    fn struct_value() {
        let Vec3 = Rc::new(StructTypeInfo {
            members: vec![
                StructMemberInfo {
                    name: Some(String::from("x")),
                    value_type: Some(FLOAT32_VALUE),
                    default_value: Some(Value::float32(0.0))
                },
                StructMemberInfo {
                    name: Some(String::from("y")),
                    value_type: Some(FLOAT32_VALUE),
                    default_value: Some(Value::float32(0.0))
                },
                StructMemberInfo {
                    name: Some(String::from("z")),
                    value_type: Some(FLOAT32_VALUE),
                    default_value: Some(Value::float32(0.0))
                },
            ]
        });
        const X: StructMemberId = 0;
        const Y: StructMemberId = 1;
        const Z: StructMemberId = 2;

        let FooBar = Rc::new(StructTypeInfo {
            members: vec![
                StructMemberInfo {
                    name: Some(String::from("position")),
                    value_type: Some(STRUCT_PTR),
                    default_value: None
                },
                StructMemberInfo {
                    name: Some(String::from("speed")),
                    value_type: Some(STRUCT_PTR),
                    default_value: None
                }
            ]
        });


        let mut a = Struct::new(Vec3.clone());
        assert_eq!(a.get(X), &Value::float32(0.0));
        assert_eq!(a.get(Y), &Value::float32(0.0));
        assert_eq!(a.get(Z), &Value::float32(0.0));

        assert_eq!(a.type_info().member(X).name, Some(String::from("x")));
        assert_eq!(a.type_info().member(Y).name, Some(String::from("y")));
        assert_eq!(a.type_info().member(Z).name, Some(String::from("z")));

        a.set(X, Value::float32(42.0));
        assert_eq!(a.get(X), &Value::float32(42.0));

        let mut foobar = Struct::new(FooBar.clone());
        foobar.set(0, a.into_value());

        assert!(foobar.get(0).is_struct());
        assert!(foobar.get(1).is_void());

        foobar.set(1, Struct::new(Vec3.clone()).into_value());
    }
}
