
use boxed;
use ssa::bytecode::*;

use std::mem;

#[deriving(Clone, Show)]
pub enum InterpreterError {
    InvalidOpCode(u8),
    IncompatibleTypes(ByteCode, boxed::ValueType, boxed::ValueType),
    InvalidRegister,
    UnknowError
}

type ProgramCounter = uint;

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

pub struct Interpreter {
    // TODO: don't separate registers.
    registers: [boxed::Value, ..64],
    functions: [unsafe fn(a: boxed::Value, b: boxed::Value) -> boxed::Value, ..30],
}

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {
            registers: [boxed::Value::void(), ..64],
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

    pub fn get_register(&self, code: &[u8],  offset: uint) -> boxed::Value {
        return self.registers[code[offset + 1] as uint];
    }

    pub fn exec(
        &mut self,
        script: &Script,
    ) -> Result<(), InterpreterError> {
        unsafe {
        let mut pc: ProgramCounter = 0;
        let code = script.bytecode.as_slice();

        loop {
            let op = code[pc];
            match op {
                OP_NULL => { pc += 1; }
                OP_EXIT => { return Ok(()); }
                OP_JMP => { pc = code[pc+1] as uint; }
                OP_ADD...OP_GTE_I32 => {
                    self.registers[code[pc+1] as uint] = self.functions[(op - OP_ADD) as uint](
                        self.get_register(code, code[pc+2] as uint),
                        self.get_register(code, code[pc+3] as uint)
                    );
                    pc += 4;
                }
                OP_CONST_I32 => {
                    self.registers[code[pc+1] as uint] = boxed::Value::int32(*unpack::<i32>(&code[pc+2]));
                    pc += 6;
                }
                OP_CONST_F32 => {
                    self.registers[code[pc+1] as uint] = boxed::Value::float32(*unpack::<f32>(&code[pc+2]));
                    pc += 6;
                }
                OP_CONST_BOXED => {
                    self.registers[code[pc+1] as uint] = *unpack::<boxed::Value>(&code[pc+2]);
                    pc += 10;
                }
                OP_BRANCH => {
                    let cond = self.get_register(code, code[pc+1] as uint).to_int32();
                    if cond != 0 {
                        pc = code[pc+2] as uint;
                    } else {
                        pc += 3;
                    }
                }
                OP_DBG => {
                    let addr = code[pc+1] as uint;
                    let val = self.get_register(code, addr);
                    println!(" -- dbg: {}", val);
                    pc += 2;
                }
                _ => { panic!(); }
            }
        }
    } // unsafe
    }
}

fn unpack<T>(ptr: &u8) -> &T {
    unsafe {
        let casted_ptr: &T = mem::transmute(ptr);
        return casted_ptr;
    }
}

pub struct Script {
    pub bytecode: Vec<u8>,
}

#[cfg(test)]
mod test {
    use ssa::bytecode::*;
    use super::*;
    use ssa::emitter::*;

    #[test]
    fn simple_addition() {
        let mut code: Vec<u8> = Vec::new();

        let mut emitter = Emitter::new();
        let a = emitter.int32_constant(42, register(0));
        let b = emitter.int32_constant(8, register(1));
        let res = emitter.operator(OP_ADD, a, b, register(2));
        emitter.debug(res);
        emitter.exit();
        let script = emitter.end();

        let mut interpreter = Interpreter::new();
        interpreter.exec(&script);
    }
}