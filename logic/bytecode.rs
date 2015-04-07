use std::mem;
use nanboxed;

pub type ByteCode = u8;

// binary ops
pub const OP_NULL:  ByteCode = 0;
pub const OP_ADD:   ByteCode = 1;
pub const OP_SUB:   ByteCode = 2;
pub const OP_MUL:   ByteCode = 3;
pub const OP_DIV:   ByteCode = 4;
pub const OP_EQ:    ByteCode = 5;
pub const OP_GT:    ByteCode = 6;
pub const OP_GTE:   ByteCode = 7;

pub const OP_MOVE:  ByteCode = 10; // op | src | dst
pub const OP_CONST: ByteCode = 11; // op | dst | payload...
// unary ops
pub const OP_NOT:   ByteCode = 12;
pub const OP_INC:   ByteCode = 13;

pub const OP_JMP:   ByteCode = 20;
pub const OP_BRANCH:ByteCode = 21; // op | cond | jmp
pub const OP_CALL:  ByteCode = 22;
pub const OP_EXIT:  ByteCode = 23;

pub const OP_DBG:   ByteCode = 42;

pub struct Script {
    pub byte_code: Vec<u8>,
}

pub type OpCodeSize = usize;
pub const BINARY_OP_SIZE: OpCodeSize = 4;   // op | src1 | src2 | dst
pub const UNARY_OP_SIZE: OpCodeSize = 3;    // op | src1  dst
pub const MOVE_OP_SIZE: OpCodeSize = 3;     // op | src | dst
pub const CONST_OP_SIZE: OpCodeSize = 10;   // op | dst | val(8 bytes)

pub struct ByteCodeBuilder {
    byte_code: Vec<u8>,
}

impl ByteCodeBuilder {
    pub fn new() -> ByteCodeBuilder {
        ByteCodeBuilder {
            byte_code: Vec::new(),
        }
    }

    pub fn end(&mut self) -> Script {
        self.byte_code.push(OP_EXIT);
        let mut script = Script {
            byte_code: Vec::new(),
        };
        mem::swap(&mut self.byte_code, &mut script.byte_code);
        return script;
    }

    pub fn exit(&mut self) {
        self.byte_code.push(OP_EXIT);
    }

    pub fn add(&mut self, a: u8, b: u8, dst: u8) {
        self.byte_code.push(OP_ADD);
        self.byte_code.push(a);
        self.byte_code.push(b);
        self.byte_code.push(dst);
    }

    pub fn sub(&mut self, a: u8, b: u8, dst: u8) {
        self.byte_code.push(OP_SUB);
        self.byte_code.push(a);
        self.byte_code.push(b);
        self.byte_code.push(dst);
    }

    pub fn mul(&mut self, a: u8, b: u8, dst: u8) {
        self.byte_code.push(OP_MUL);
        self.byte_code.push(a);
        self.byte_code.push(b);
        self.byte_code.push(dst);
    }

    pub fn div(&mut self, a: u8, b: u8, dst: u8) {
        self.byte_code.push(OP_DIV);
        self.byte_code.push(a);
        self.byte_code.push(b);
        self.byte_code.push(dst);
    }

    pub fn mov(&mut self, from: u8, to: u8) {
        self.byte_code.push(OP_DIV);
        self.byte_code.push(from);
        self.byte_code.push(to);
    }

    pub fn jump(&mut self, to: u8) {
        self.byte_code.push(OP_JMP);
        self.byte_code.push(to);
    }

    pub fn constant(&mut self, to: u8, val: nanboxed::Value) {
        self.byte_code.push(OP_CONST);
        self.byte_code.push(to);
        let val_bytes = val.get_bytes();

        println!("pack val {:?}", val);
        println!("val bytes {:?}", &val.get_bytes()[..]);

        self.byte_code.push(val_bytes[0]);
        self.byte_code.push(val_bytes[1]);
        self.byte_code.push(val_bytes[2]);
        self.byte_code.push(val_bytes[3]);
        self.byte_code.push(val_bytes[4]);
        self.byte_code.push(val_bytes[5]);
        self.byte_code.push(val_bytes[6]);
        self.byte_code.push(val_bytes[7]);
    }
}
