use std::mem;

pub type ValueType = u8;
pub const VOID_VALUE:       ValueType = 0b0000;
pub const POINTER_VALUE:    ValueType = 0b1000;
pub const NUMBER_VALUE:     ValueType = 0b0100;
pub const BOOLEAN_VALUE:    ValueType = 0b0010;
pub const ARRAY_PTR:        ValueType = 0b1001;
pub const STRUCT_PTR:       ValueType = 0b0011;

pub type Mask = u64;
pub const SIGN_MASK:        Mask = 0b1000000000000000000000000000000000000000000000000000000000000000;
pub const EXPONENT_MASK:    Mask = 0b0111111111110000000000000000000000000000000000000000000000000000;
pub const SIGNIFICAND_MASK: Mask = 0b0000000000001111111111111111111111111111111111111111111111111111;

pub const NAN_MASK:         Mask = 0b1111111111111000000000000000000000000000000000000000000000000000;
pub const TAG_MASK:         Mask = 0b0000000000000111100000000000000000000000000000000000000000000000;
pub const VAL_MASK:         Mask = 0b0000000000000000011111111111111111111111111111111111111111111111;

pub const PTR_TAG_BIT:      Mask = ((POINTER_VALUE as Mask) << TAG_OFFSET) as Mask;
pub const BOOL_TAG_BIT:     Mask = ((BOOLEAN_VALUE as Mask) << TAG_OFFSET) as Mask;
pub const VOID_TAG_BIT:     Mask = ((VOID_VALUE as Mask) << TAG_OFFSET) as Mask;

pub type ByteOffset = usize;
pub const TAG_OFFSET: ByteOffset = 47;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Value {
    payload: u64,
}

pub struct Structure {
    foo: u32,
}

#[derive(Copy, Clone, Debug)]
pub enum UnpackedValue {
    NumberValue(f64),
    PointerValue(*mut Structure),
    BooleanValue(bool),
    VoidValue,
}

impl Value {
    pub fn number(val: f64) -> Value {
        unsafe {
            Value {
                payload: mem::transmute(val)
            }
        }
    }

    pub fn boolean(val: bool) -> Value {
        Value {
            payload: NAN_MASK | BOOL_TAG_BIT | if val { 1 } else { 0 }
        }
    }

    pub fn ptr(val: *mut Structure) -> Value {
        Value {
            payload: NAN_MASK | PTR_TAG_BIT | (val as u64)
        }
    }

    pub fn void() -> Value {
        Value {
            payload: NAN_MASK | VOID_TAG_BIT
        }
    }

    pub fn get_type(&self) -> ValueType {
        return if self.is_number() { NUMBER_VALUE }
               else { ((self.payload & TAG_MASK) >> TAG_OFFSET) as u8 };
    }

    pub fn is_number(&self) -> bool {
        return self.payload & NAN_MASK != NAN_MASK;
    }

    pub fn is_ptr(&self) -> bool {
        return self.get_type() == POINTER_VALUE;
    }

    pub fn is_void(&self) -> bool {
        return self.get_type() == VOID_VALUE;
    }

    pub fn is_boolean(&self) -> bool {
        return self.get_type() == BOOLEAN_VALUE;
    }

    pub fn get_ptr(&self) -> Option<*mut Structure> {
        if self.is_ptr() {
            unsafe {
                return Some(mem::transmute(self.payload & VAL_MASK));
            }
        }
        return None;
    }

    pub fn get_number(&self) -> Option<f64> {
        if self.is_number() {
            unsafe {
                return Some(mem::transmute(self.payload));
            }
        }
        return None;
    }

    pub fn get_boolean(&self) -> Option<bool> {
        if self.is_boolean() {
            return Some(self.payload & VAL_MASK != 0);
        }
        return None;
    }

    pub fn get_unpacked(&self) -> UnpackedValue {
        match self.get_number() {
            Some(num) => { return UnpackedValue::NumberValue(num); }
            _ => {}
        }

        match self.get_ptr() {
            Some(p) => { return UnpackedValue::PointerValue(p); }
            _ => {}
        }

        match self.get_boolean() {
            Some(b) => { return UnpackedValue::BooleanValue(b); }
            _ => {}
        }

        if self.is_void() { return UnpackedValue::VoidValue; }

        panic!("invalid value");
    }

    pub fn get_bytes(&self) -> [u8; 8] {
        return unsafe { mem::transmute(self.payload) };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn fuzzy_eq(a: f64, b: f64) -> bool { return (a - b).abs() < 0.000001; }

    #[test]
    fn float_value() {
        let a = Value::number(42.0);
        assert_eq!(a.get_type(), NUMBER_VALUE);
        assert!(a.is_number());
        assert!(!a.is_boolean());
        assert!(!a.is_void());
        assert!(!a.is_ptr());
        match a.get_unpacked() {
            UnpackedValue::NumberValue(num) => { assert!(fuzzy_eq(num, 42.0)) }
            _ => {}
        }
        assert!(fuzzy_eq(a.get_number().unwrap(), 42.0));
    }

    #[test]
    fn boolean_value() {
        let t = Value::boolean(true);
        assert_eq!(t, Value::boolean(true));
        assert_eq!(t.get_type(), BOOLEAN_VALUE);
        assert!(t.is_boolean());
        assert!(!t.is_number());
        assert!(!t.is_void());
        assert!(!t.is_ptr());
        assert_eq!(t.get_boolean(), Some(true));
        match t.get_unpacked() {
            UnpackedValue::BooleanValue(true) => {}
            _ => { panic!(); }
        }

        let f = Value::boolean(false);
        assert_eq!(f, Value::boolean(false));
        assert!(f.is_boolean());
        assert!(!f.is_number());
        assert!(!f.is_void());
        assert!(!f.is_ptr());
        assert_eq!(f.get_type(), BOOLEAN_VALUE);
        assert_eq!(f.get_boolean(), Some(false));
        match f.get_unpacked() {
            UnpackedValue::BooleanValue(false) => {}
            _ => { panic!(); }
        }
    }

    #[test]
    fn void_value() {
        let a = Value::void();
        assert_eq!(a, Value::void());
        assert_eq!(a.get_type(), VOID_VALUE);
        assert!(a.is_void());
        assert!(!a.is_boolean());
        assert!(!a.is_number());
        assert!(!a.is_ptr());
        match a.get_unpacked() {
            UnpackedValue::VoidValue => {}
            _ => { panic!(); }
        }
    }

}

