
pub type ByteCode = u8;

pub const OP_NULL:  ByteCode = 0;
pub const OP_BRANCH:ByteCode = 1; // op | cond | jmp
pub const OP_JMP:   ByteCode = 2;
pub const OP_CALL:  ByteCode = 3;
pub const OP_EXIT:  ByteCode = 4;
pub const OP_ABORT: ByteCode = 5;
pub const OP_DBG:   ByteCode = 6;

pub const OP_CONST_32: ByteCode = 10; // op | payload...
pub const OP_CONST_64: ByteCode = 11; // op | payload...

pub const OP_3: u8 = 16;

pub const OP_F32_ADD:       ByteCode = OP_3 + 0;
pub const OP_F32_SUB:       ByteCode = OP_3 + 1;
pub const OP_F32_MUl:       ByteCode = OP_3 + 2;
pub const OP_F32_DIV:       ByteCode = OP_3 + 3;
pub const OP_F32_MOD:       ByteCode = OP_3 + 4;
pub const OP_F32_MIN:       ByteCode = OP_3 + 5;
pub const OP_F32_MAX:       ByteCode = OP_3 + 6;
pub const OP_F32_CMP_EQ:    ByteCode = OP_3 + 7;
pub const OP_F32_CMP_LT:    ByteCode = OP_3 + 8;
pub const OP_F32_CMP_GT:    ByteCode = OP_3 + 9;
pub const OP_F32_CMP_LTE:   ByteCode = OP_3 + 10;
pub const OP_F32_CMP_GTE:   ByteCode = OP_3 + 11;
pub const OP_I32_ADD:       ByteCode = OP_3 + 12;
pub const OP_I32_SUB:       ByteCode = OP_3 + 13;
pub const OP_I32_MUl:       ByteCode = OP_3 + 14;
pub const OP_I32_DIV:       ByteCode = OP_3 + 15;
pub const OP_I32_MOD:       ByteCode = OP_3 + 16;
pub const OP_I32_MIN:       ByteCode = OP_3 + 17;
pub const OP_I32_MAX:       ByteCode = OP_3 + 18;
pub const OP_I32_CMP_EQ:    ByteCode = OP_3 + 19;
pub const OP_I32_CMP_LT:    ByteCode = OP_3 + 20;
pub const OP_I32_CMP_GT:    ByteCode = OP_3 + 21;
pub const OP_I32_CMP_LTE:   ByteCode = OP_3 + 22;
pub const OP_I32_CMP_GTE:   ByteCode = OP_3 + 23;

pub const OP_BXD_CAST:      ByteCode = OP_3 + 24; // op(src, ty, dst)
pub const OP_ARRAY_INDEX:   ByteCode = OP_3 + 25; // op(arr, idx, dst:ptr)
pub const OP_DEREF:         ByteCode = OP_3 + 26; // op(src:ptr, dst)

pub const OP_3_SENTINEL:    ByteCode = OP_I32_CMP_GTE;

pub const OP_2: u8 = 64;

pub const OP_I32_F32_CAST:  ByteCode = OP_2 + 0;
pub const OP_F32_I32_CAST:  ByteCode = OP_2 + 1;
pub const OP_BOOL_NOT:      ByteCode = OP_2 + 2;
pub const OP_SWAP_WORD:     ByteCode = OP_2 + 3;
pub const OP_CP_WORD:       ByteCode = OP_2 + 4;
pub const OP_BXD_TYPE:      ByteCode = OP_2 + 5;

pub const OP_2_SENTINEL:    ByteCode = OP_BXD_TYPE;

pub const OP_1: u8 = 96;
pub const OP_CLEAR_WORD:    ByteCode = OP_1 + 0;
pub const OP_1_SENTINEL: u8 = OP_CLEAR_WORD;


// op_f32_add,
// op_f32_sub,
// op_f32_mul,
// op_f32_div,
// op_f32_mod,
// op_f32_min,
// op_f32_max,
// op_f32_cmp_eq,
// op_f32_cmp_lt,
// op_f32_cmp_gt,
// op_f32_cmp_lte,
// op_f32_cmp_gte,
// op_i32_add,
// op_i32_sub,
// op_i32_mul,
// op_i32_div,
// op_i32_mod,
// op_i32_min,
// op_i32_max,
// op_i32_cmp_eq,
// op_i32_cmp_lt,
// op_i32_cmp_gt,
// op_i32_cmp_lte,
// op_i32_cmp_gte,
// op_bool_and,
// op_bool_or,
//
// op_i32_f32_cast,
// op_f32_i32_cast,
// op_bool_not,
// op_swap_words,
// op_cp_word,

// unary ops
