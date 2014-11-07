use std::mem;

pub type ByteCode = u8;

pub const OP_TYPE_MASK: ByteCode    = 0b11000000;
pub const OP_MASK: ByteCode         = 0b00111111;

pub const OP_TYPE_INT32: ByteCode   = 0b10000000;
pub const OP_TYPE_FLOAT32: ByteCode = 0b01000000;

// binary ops betwee 0 and 64
pub const OP_NULL:  ByteCode = 0;

pub const OP_ADD:   ByteCode = 1;
pub const OP_SUB:   ByteCode = 2;
pub const OP_MUL:   ByteCode = 3;
pub const OP_DIV:   ByteCode = 4;

pub const OP_EQ:    ByteCode = 5;
pub const OP_LT:    ByteCode = 6;
pub const OP_GT:    ByteCode = 7;
pub const OP_LTE:   ByteCode = 8;
pub const OP_GTE:   ByteCode = 9;

pub const OP_MOVE:  ByteCode = 10; // op | src
pub const OP_CONST: ByteCode = 11; // op | payload...
// unary ops
pub const OP_NOT:   ByteCode = 12;
pub const OP_INC:   ByteCode = 13;

pub const OP_JMP:   ByteCode = 20;
pub const OP_BRANCH:ByteCode = 21; // op | cond | jmp
pub const OP_CALL:  ByteCode = 22;
pub const OP_EXIT:  ByteCode = 23;

pub const OP_PHI:  ByteCode = 30;

pub const OP_DBG:   ByteCode = 42;
