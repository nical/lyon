
pub type ByteCode = u8;

// binary ops betwee 0 and 64
pub const OP_NULL:  ByteCode = 0;

pub const OP_ADD:   ByteCode = 1;
pub const OP_SUB:   ByteCode = 2;
pub const OP_MUL:   ByteCode = 3;
pub const OP_DIV:   ByteCode = 4;
pub const OP_MOD:   ByteCode = 5;
pub const OP_EQ:    ByteCode = 6;
pub const OP_LT:    ByteCode = 7;
pub const OP_GT:    ByteCode = 8;
pub const OP_LTE:   ByteCode = 9;
pub const OP_GTE:   ByteCode = 10;

pub const OP_ADD_F32:   ByteCode = 11;
pub const OP_SUB_F32:   ByteCode = 12;
pub const OP_MUL_F32:   ByteCode = 13;
pub const OP_DIV_F32:   ByteCode = 14;
pub const OP_MOD_F32:   ByteCode = 15;
pub const OP_EQ_F32:    ByteCode = 16;
pub const OP_LT_F32:    ByteCode = 17;
pub const OP_GT_F32:    ByteCode = 18;
pub const OP_LTE_F32:   ByteCode = 19;
pub const OP_GTE_F32:   ByteCode = 20;

pub const OP_ADD_I32:   ByteCode = 21;
pub const OP_SUB_I32:   ByteCode = 22;
pub const OP_MUL_I32:   ByteCode = 23;
pub const OP_DIV_I32:   ByteCode = 24;
pub const OP_MOD_I32:   ByteCode = 25;
pub const OP_EQ_I32:    ByteCode = 26;
pub const OP_LT_I32:    ByteCode = 27;
pub const OP_GT_I32:    ByteCode = 28;
pub const OP_LTE_I32:   ByteCode = 29;
pub const OP_GTE_I32:   ByteCode = 30;

// unary ops
pub const OP_NOT:   ByteCode = 42;
pub const OP_INC:   ByteCode = 43;

pub const OP_MOVE:  ByteCode = 50; // op | src
pub const OP_CONST_BOXED: ByteCode = 51; // op | payload...
pub const OP_CONST_F32: ByteCode = 52; // op | payload...
pub const OP_CONST_I32: ByteCode = 53; // op | payload...

pub const OP_JMP:   ByteCode = 60;
pub const OP_BRANCH:ByteCode = 61; // op | cond | jmp
pub const OP_CALL:  ByteCode = 62;
pub const OP_EXIT:  ByteCode = 63;

pub const OP_PHI:  ByteCode = 70;

pub const OP_DBG:  ByteCode = 100;
