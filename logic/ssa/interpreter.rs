
use boxed;
use ssa::bytecode::*;

use std::mem;

#[derive(Copy, Clone, Debug)]
pub enum InterpreterError {
    InvalidOpCode(u8),
    IncompatibleTypes(ByteCode, boxed::ValueType, boxed::ValueType),
    InvalidRegister,
    UnknowError
}

type ProgramCounter = usize;

unsafe fn int32_add(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::int32(a.get_int32_unchecked() + b.get_int32_unchecked())
}
unsafe fn int32_sub(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::int32(a.get_int32_unchecked() - b.get_int32_unchecked())
}
unsafe fn int32_mul(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::int32(a.get_int32_unchecked() * b.get_int32_unchecked())
}
unsafe fn int32_div(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::int32(a.get_int32_unchecked() / b.get_int32_unchecked())
}
unsafe fn int32_mod(a: boxed::Value, m: boxed::Value) -> boxed::Value {
    let va = a.get_int32_unchecked();
    let vm = m.get_int32_unchecked();
    return boxed::Value::int32((va % vm + vm) % vm);
}
unsafe fn int32_cmp_eq(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.get_int32_unchecked() == b.get_int32_unchecked())
}
unsafe fn int32_cmp_lt(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.get_int32_unchecked() < b.get_int32_unchecked())
}
unsafe fn int32_cmp_gt(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.get_int32_unchecked() > b.get_int32_unchecked())
}
unsafe fn int32_cmp_lte(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.get_int32_unchecked() <= b.get_int32_unchecked())
}
unsafe fn int32_cmp_gte(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.get_int32_unchecked() >= b.get_int32_unchecked())
}


unsafe fn float32_add(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::float32(a.get_float32_unchecked() + b.get_float32_unchecked())
}
unsafe fn float32_sub(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::float32(a.get_float32_unchecked() - b.get_float32_unchecked())
}
unsafe fn float32_mul(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::float32(a.get_float32_unchecked() * b.get_float32_unchecked())
}
unsafe fn float32_div(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::float32(a.get_float32_unchecked() / b.get_float32_unchecked())
}
unsafe fn float32_mod(a: boxed::Value, m: boxed::Value) -> boxed::Value {
    let va = a.get_float32_unchecked();
    let vm = m.get_float32_unchecked();
    return boxed::Value::float32((va % vm + vm) % vm);
}
unsafe fn float32_cmp_eq(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a == b)
}
unsafe fn float32_cmp_lt(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.get_float32_unchecked() < b.get_float32_unchecked())
}
unsafe fn float32_cmp_gt(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.get_float32_unchecked() > b.get_float32_unchecked())
}
unsafe fn float32_cmp_lte(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.get_float32_unchecked() <= b.get_float32_unchecked())
}
unsafe fn float32_cmp_gte(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.get_float32_unchecked() >= b.get_float32_unchecked())
}

unsafe fn boxed_add(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    if a.get_type() == boxed::INT32_VALUE && b.get_type() == boxed::INT32_VALUE {
        return int32_add(a, b);
    }
    return boxed::Value::float32(a.to_float32() + b.to_float32());
}
unsafe fn boxed_sub(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    if a.get_type() == boxed::INT32_VALUE && b.get_type() == boxed::INT32_VALUE {
        return int32_add(a, b);
    }
    return boxed::Value::float32(a.to_float32() - b.to_float32());
}
unsafe fn boxed_mul(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    if a.get_type() == boxed::INT32_VALUE && b.get_type() == boxed::INT32_VALUE {
        return int32_mul(a, b);
    }
    return boxed::Value::float32(a.to_float32() * b.to_float32());
}
unsafe fn boxed_div(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    if a.get_type() == boxed::INT32_VALUE && b.get_type() == boxed::INT32_VALUE {
        return int32_div(a, b);
    }
    return boxed::Value::float32(a.to_float32() / b.to_float32());
}
unsafe fn boxed_mod(a: boxed::Value, m: boxed::Value) -> boxed::Value {
    let va = a.to_float32();
    let vm = m.to_float32();
    return boxed::Value::float32((va % vm + vm) % vm);
}
unsafe fn boxed_cmp_eq(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a == b)
}
unsafe fn boxed_cmp_lt(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.to_float32() < b.to_float32())
}
unsafe fn boxed_cmp_gt(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.to_float32() > b.to_float32())
}
unsafe fn boxed_cmp_lte(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.to_float32() <= b.to_float32())
}
unsafe fn boxed_cmp_gte(a: boxed::Value, b: boxed::Value) -> boxed::Value {
    boxed::Value::boolean(a.to_float32() >= b.to_float32())
}

type Register = u64;
unsafe fn as_boxed(reg: *mut Register) -> *mut boxed::Value { mem::transmute(reg) }
unsafe fn as_unboxed<T>(reg: *mut Register) -> *mut T { mem::transmute(reg) }

/*
pub struct Interpreter {
    // TODO: don't separate registers.
    registers: [boxed::Value; 64],
    functions: [unsafe fn(a: boxed::Value, b: boxed::Value) -> boxed::Value; 30],
}

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {
            registers: unsafe { mem::transmute([boxed::VOID; 64]) },
            functions: [
                boxed_add,
                boxed_sub,
                boxed_mul,
                boxed_div,
                boxed_mod,
                boxed_cmp_eq,
                boxed_cmp_lt,
                boxed_cmp_gt,
                boxed_cmp_lte,
                boxed_cmp_gte,

                float32_add,
                float32_sub,
                float32_mul,
                float32_div,
                float32_mod,
                float32_cmp_eq,
                float32_cmp_lt,
                float32_cmp_gt,
                float32_cmp_lte,
                float32_cmp_gte,

                int32_add,
                int32_sub,
                int32_mul,
                int32_div,
                int32_mod,
                int32_cmp_eq,
                int32_cmp_lt,
                int32_cmp_gt,
                int32_cmp_lte,
                int32_cmp_gte,
            ],
        }
    }

    pub fn get_register(&self, code: &[u8],  offset: usize) -> boxed::Value {
        return self.registers[code[offset + 1] as usize];
    }

    pub fn exec(
        &mut self,
        script: &Script,
    ) -> Result<(), InterpreterError> {
        unsafe {
        let mut pc: ProgramCounter = 0;
        let code = &script.bytecode[..];

        loop {
            let op = code[pc];
            match op {
                OP_NULL => { pc += 1; }
                OP_EXIT => { return Ok(()); }
                OP_JMP => { pc = code[pc+1] as usize; }
                OP_ADD...OP_GTE_I32 => {
                    self.registers[code[pc+1] as usize] = self.functions[(op - OP_ADD) as usize](
                        self.get_register(code, code[pc+2] as usize),
                        self.get_register(code, code[pc+3] as usize)
                    );
                    pc += 4;
                }
                OP_CONST_I32 => {
                    self.registers[code[pc+1] as usize] = boxed::Value::int32(*unpack::<i32>(&code[pc+2]));
                    pc += 6;
                }
                OP_CONST_F32 => {
                    self.registers[code[pc+1] as usize] = boxed::Value::float32(*unpack::<f32>(&code[pc+2]));
                    pc += 6;
                }
                OP_CONST_BOXED => {
                    self.registers[code[pc+1] as usize] = *unpack::<boxed::Value>(&code[pc+2]);
                    pc += 10;
                }
                OP_BRANCH => {
                    let cond = self.get_register(code, code[pc+1] as usize).to_int32();
                    if cond != 0 {
                        pc = code[pc+2] as usize;
                    } else {
                        pc += 3;
                    }
                }
                OP_DBG => {
                    let addr = code[pc+1] as usize;
                    let val = self.get_register(code, addr);
                    println!(" -- dbg: {:?}", val);
                    pc += 2;
                }
                _ => { panic!(); }
            }
        }
    } // unsafe
    }
}
*/
fn unpack<T>(ptr: &u8) -> &T {
    unsafe {
        let casted_ptr: &T = mem::transmute(ptr);
        return casted_ptr;
    }
}

pub struct Script {
    pub bytecode: Vec<u8>,
}

pub type Word = u32;


fn _f32_max(a: f32, b: f32) -> f32 { if a > b { a } else { b } }
fn _f32_min(a: f32, b: f32) -> f32 { if a > b { b } else { a } }
fn _i32_max(a: i32, b: i32) -> i32 { if a > b { a } else { b } }
fn _i32_min(a: i32, b: i32) -> i32 { if a > b { b } else { a } }

unsafe fn as_word<T>(a: &T) -> &Word { mem::transmute(a) }
unsafe fn as_f32(a: *mut Word) -> *mut f32 { mem::transmute(a) }
unsafe fn as_i32(a: *mut Word) -> *mut i32 { mem::transmute(a) }
fn bool_word(a: bool) -> Word { if a { 1 } else {0} }
fn word_bool(a: Word) -> bool { a != 0 }


unsafe fn op_f32_add(a: *mut Word, b: *mut Word, result: *mut Word) { *as_f32(result) = *as_f32(a) + *as_f32(b); }
unsafe fn op_f32_sub(a: *mut Word, b: *mut Word, result: *mut Word) { *as_f32(result) = *as_f32(a) - *as_f32(b); }
unsafe fn op_f32_mul(a: *mut Word, b: *mut Word, result: *mut Word) { *as_f32(result) = *as_f32(a) * *as_f32(b); }
unsafe fn op_f32_div(a: *mut Word, b: *mut Word, result: *mut Word) { *as_f32(result) = *as_f32(a) / *as_f32(b); }
unsafe fn op_f32_mod(a: *mut Word, b: *mut Word, result: *mut Word) { *as_f32(result) = *as_f32(a) % *as_f32(b); }
unsafe fn op_f32_min(a: *mut Word, b: *mut Word, result: *mut Word) { *as_f32(result) = _f32_min(*as_f32(a), *as_f32(b)); }
unsafe fn op_f32_max(a: *mut Word, b: *mut Word, result: *mut Word) { *as_f32(result) = _f32_max(*as_f32(a), *as_f32(b)); }
unsafe fn op_f32_cmp_eq(a: *mut Word, b: *mut Word, result: *mut Word)  { *as_i32(result) = if *as_f32(a) == *as_f32(b) { 1 } else { 0 }; }
unsafe fn op_f32_cmp_lt(a: *mut Word, b: *mut Word, result: *mut Word)  { *as_i32(result) = if *as_f32(a) <  *as_f32(b) { 1 } else { 0 }; }
unsafe fn op_f32_cmp_gt(a: *mut Word, b: *mut Word, result: *mut Word)  { *as_i32(result) = if *as_f32(a) >  *as_f32(b) { 1 } else { 0 }; }
unsafe fn op_f32_cmp_lte(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = if *as_f32(a) <= *as_f32(b) { 1 } else { 0 }; }
unsafe fn op_f32_cmp_gte(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = if *as_f32(a) >= *as_f32(b) { 1 } else { 0 }; }
unsafe fn op_i32_add(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = *as_i32(a) + *as_i32(b); }
unsafe fn op_i32_sub(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = *as_i32(a) - *as_i32(b); }
unsafe fn op_i32_mul(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = *as_i32(a) * *as_i32(b); }
unsafe fn op_i32_div(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = *as_i32(a) / *as_i32(b); }
unsafe fn op_i32_mod(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = *as_i32(a) % *as_i32(b); }
unsafe fn op_i32_min(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = _i32_min(*as_i32(a), *as_i32(b)); }
unsafe fn op_i32_max(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = _i32_max(*as_i32(a), *as_i32(b)); }
unsafe fn op_i32_cmp_eq(a: *mut Word, b: *mut Word, result: *mut Word)  { *as_i32(result) = if *as_i32(a) == *as_i32(b) { 1 } else { 0 }; }
unsafe fn op_i32_cmp_lt(a: *mut Word, b: *mut Word, result: *mut Word)  { *as_i32(result) = if *as_i32(a) <  *as_i32(b) { 1 } else { 0 }; }
unsafe fn op_i32_cmp_gt(a: *mut Word, b: *mut Word, result: *mut Word)  { *as_i32(result) = if *as_i32(a) >  *as_i32(b) { 1 } else { 0 }; }
unsafe fn op_i32_cmp_lte(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = if *as_i32(a) <= *as_i32(b) { 1 } else { 0 }; }
unsafe fn op_i32_cmp_gte(a: *mut Word, b: *mut Word, result: *mut Word) { *as_i32(result) = if *as_i32(a) >= *as_i32(b) { 1 } else { 0 }; }
unsafe fn op_bool_and(a: *mut Word, b: *mut Word, result: *mut Word) { *result = bool_word(word_bool(*a) && word_bool(*b)); }
unsafe fn op_bool_or(a: *mut Word, b: *mut Word, result: *mut Word) { *result = bool_word(word_bool(*a) || word_bool(*b)); }

unsafe fn op_i32_f32_cast(a: *mut Word, result: *mut Word) { *as_f32(result) = *as_i32(a) as f32; }
unsafe fn op_f32_i32_cast(a: *mut Word, result: *mut Word) { *as_i32(result) = *as_f32(a) as i32; }
unsafe fn op_bool_not(a: *mut Word, result: *mut Word) { *result = bool_word(!word_bool(*a)); }
unsafe fn op_swap_words(a: *mut Word, b: *mut Word) { let tmp = *a; *a = *b; *b = tmp; }
unsafe fn op_cp_word(src: *mut Word, dst: *mut Word) { *dst = *src; }

unsafe fn op_clear_word(a: *mut Word) { *as_i32(a) = 0; }

pub type UnsafeFunctionPtr = *mut u64;
unsafe fn as_op_2_1(f: UnsafeFunctionPtr ) -> fn(*mut Word, *mut Word, *mut Word) { mem::transmute(f) }
unsafe fn as_op_1_1(f: UnsafeFunctionPtr ) -> fn(*mut Word, *mut Word) { mem::transmute(f) }
unsafe fn as_op_1(f: UnsafeFunctionPtr ) -> fn(*mut Word, *mut Word) { mem::transmute(f) }

struct ExecContext {
    ops_3: Vec<unsafe fn(*mut Word, *mut Word, *mut Word)>,
    ops_2: Vec<unsafe fn(*mut Word, *mut Word)>,
    ops_1: Vec<unsafe fn(*mut Word)>,
    registers: Vec<Word>,
}

impl ExecContext {
    pub fn new(num_registers: usize) -> ExecContext {
        ExecContext {
            ops_3: vec![
                op_f32_add,
                op_f32_sub,
                op_f32_mul,
                op_f32_div,
                op_f32_mod,
                op_f32_min,
                op_f32_max,
                op_f32_cmp_eq,
                op_f32_cmp_lt,
                op_f32_cmp_gt,
                op_f32_cmp_lte,
                op_f32_cmp_gte,
                op_i32_add,
                op_i32_sub,
                op_i32_mul,
                op_i32_div,
                op_i32_mod,
                op_i32_min,
                op_i32_max,
                op_i32_cmp_eq,
                op_i32_cmp_lt,
                op_i32_cmp_gt,
                op_i32_cmp_lte,
                op_i32_cmp_gte,
                op_bool_and,
                op_bool_or,
            ],
            ops_2: vec![
                op_i32_f32_cast,
                op_f32_i32_cast,
                op_bool_not,
                op_swap_words,
                op_cp_word,
            ],
            ops_1: vec![
                op_clear_word,
            ],
            registers: vec![0 ; num_registers],
        }
    }

    pub fn exec(&mut self, code: &[u8], start: ProgramCounter) -> Result<(), ()> {
        unsafe {
        let mut pc: ProgramCounter = start;
            loop {
                let op = code[pc];
                match op {
                    OP_NULL => { pc += 1; }
                    OP_EXIT => { return Ok(()); }
                    OP_JMP => { pc = code[pc+1] as usize; }
                    OP_3 ... OP_3_SENTINEL => {
                         self.ops_3[(op - OP_3) as usize](
                            &mut self.registers[pc+1],
                            &mut self.registers[pc+2],
                            &mut self.registers[pc+3]
                        );
                        pc += 4;
                    }
                    OP_2 ... OP_2_SENTINEL => {
                        self.ops_2[(op - OP_2) as usize](
                            &mut self.registers[pc+1],
                            &mut self.registers[pc+2]
                        );
                        pc += 3;
                    }
                    OP_1 ... OP_1_SENTINEL => {
                        self.ops_1[(op - OP_2) as usize](
                            &mut self.registers[pc+1],
                        );
                        pc += 1;
                    }
                    OP_CONST_32 => {
                        self.registers[pc+1] = *as_word(&code[pc+5]);
                        pc += 6;
                    }
                    _ => { return Err(()); }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use ssa::bytecode::*;
    use super::*;
    use ssa::emitter::*;

    #[test]
    fn simple_addition() {
        let mut code: Vec<u8> = Vec::new();
/*
        let mut emitter = Emitter::new();
        let a = emitter.int32_constant(42, register(0));
        let b = emitter.int32_constant(8, register(1));
        let res = emitter.operator(OP_ADD, a, b, register(2));
        emitter.debug(res);
        emitter.exit();
        let script = emitter.end();

        let mut interpreter = Interpreter::new();
        interpreter.exec(&script);
*/
    }
}