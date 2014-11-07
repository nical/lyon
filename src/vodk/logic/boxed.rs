
use std::mem;

type Mask = u64;
const DATA_32_MASK:      Mask = 0b0000000000000000000000000000000011111111111111111111111111111111;
const PTR_DATA_MASK:     Mask = 0b0000000000000000011111111111111111111111111111111111111111111111;
const TYPE_MASK:         Mask = 0b1111111100000000000000000000000000000000000000000000000000000000;
const OWNED_BIT:         Mask = 0b0000000010000000000000000000000000000000000000000000000000000000;
const PTR_TYPE_BITS:     Mask = 0b1000000000000000000000000000000000000000000000000000000000000000;
const STRUCT_TYPE_BITS:  Mask = 0b0100000000000000000000000000000000000000000000000000000000000000;
const ARRAY_TYPE_BITS:   Mask = 0b1010000000000000000000000000000000000000000000000000000000000000;
const MAP_TYPE_BITS:     Mask = 0b1001000000000000000000000000000000000000000000000000000000000000;
const VOID_TYPE_BITS:    Mask = 0b0000000100000000000000000000000000000000000000000000000000000000;
const INT32_TYPE_BITS:   Mask = 0b0000001000000000000000000000000000000000000000000000000000000000;
const FLOAT32_TYPE_BITS: Mask = 0b0000010000000000000000000000000000000000000000000000000000000000;
const BOOLEAN_TYPE_BITS: Mask = 0b0000100000000000000000000000000000000000000000000000000000000000;

pub type ValueType = u8;
pub const VOID_VALUE:       ValueType = (VOID_TYPE_BITS >> 56) as u8;
pub const POINTER_VALUE:    ValueType = (PTR_TYPE_BITS >> 56) as u8;
pub const FLOAT32_VALUE:    ValueType = (FLOAT32_TYPE_BITS >> 56) as u8;
pub const INT32_VALUE:      ValueType = (INT32_TYPE_BITS >> 56) as u8;
pub const BOOLEAN_VALUE:    ValueType = (BOOLEAN_TYPE_BITS >> 56) as u8;
pub const ARRAY_PTR:        ValueType = (ARRAY_TYPE_BITS >> 56) as u8;
pub const STRUCT_PTR:       ValueType = (STRUCT_TYPE_BITS >> 56) as u8;

#[repr(C)]
#[deriving(Clone, Show, PartialEq)]
pub struct Value {
    payload: u64,
}


impl Value {
    pub fn void() -> Value {
        Value {
            payload: VOID_TYPE_BITS
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
        Value {
            payload: BOOLEAN_TYPE_BITS | if val { 1 } else { 0 }
        }
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
        return self.get_type() == FLOAT32_VALUE ;
    }

    pub fn is_int32(&self) -> bool {
        return self.get_type() == INT32_VALUE ;
    }

    pub fn is_pointer(&self) -> bool {
        return self.get_type() == POINTER_VALUE;
    }

    pub fn is_void(&self) -> bool {
        return self.payload & VOID_TYPE_BITS != 0;
    }

    pub fn is_boolean(&self) -> bool {
        return self.get_type() == BOOLEAN_VALUE;
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

    pub unsafe fn get_boolean_unchecked(&self) -> bool {
        return self.payload & 0x1 != 0;
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

    pub fn get_boolean(&self) -> Option<bool> {
        if self.is_boolean() {
            return unsafe { Some(self.get_boolean_unchecked()) };
        }
        return None;
    }

    pub fn get_bytes(&self) -> [u8, ..8] {
        return unsafe { mem::transmute(self.payload) };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn fuzzy_eq(a: f32, b: f32) -> bool { return (a - b).abs() < 0.000001; }

    #[test]
    fn int_value() {
        let val = -0xeffffff as i32;
        let a = Value::int32(val);
        assert_eq!(a.get_type(), INT32_VALUE);
        assert!(a.is_int32());
        assert!(!a.is_float32());
        assert!(!a.is_boolean());
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
        assert!(!a.is_boolean());
        assert!(!a.is_void());
        assert!(!a.is_pointer());
        assert!(fuzzy_eq(a.get_float32().unwrap(), 42.0));
    }

    #[test]
    fn boolean_value() {
        let t = Value::boolean(true);
        assert_eq!(t, Value::boolean(true));
        assert_eq!(t.get_type(), BOOLEAN_VALUE);
        assert!(t.is_boolean());
        assert!(!t.is_float32());
        assert!(!t.is_int32());
        assert!(!t.is_void());
        assert!(!t.is_pointer());
        assert_eq!(t.get_boolean(), Some(true));

        let f = Value::boolean(false);
        assert_eq!(f, Value::boolean(false));
        assert_eq!(f.get_type(), BOOLEAN_VALUE);
        assert!(f.is_boolean());
        assert!(!f.is_float32());
        assert!(!f.is_int32());
        assert!(!f.is_void());
        assert!(!f.is_pointer());
        assert_eq!(f.get_boolean(), Some(false));
    }

    #[test]
    fn void_value() {
        let a = Value::void();
        assert_eq!(a, Value::void());
        assert_eq!(a.get_type(), VOID_VALUE);
        assert!(a.is_void());
        assert!(!a.is_boolean());
        assert!(!a.is_float32());
        assert!(!a.is_int32());
        assert!(!a.is_pointer());
    }

}

