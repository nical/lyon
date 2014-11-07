
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

fn float32_add(a: f32, b: f32) -> f32 { a + b }
fn float32_sub(a: f32, b: f32) -> f32 { a - b }
fn float32_mul(a: f32, b: f32) -> f32 { a * b }
fn float32_div(a: f32, b: f32) -> f32 { a / b }

fn float32_cmp_eq(a: f32, b: f32) -> bool { a == b }
fn float32_cmp_lt(a: f32, b: f32) -> bool { a < b }
fn float32_cmp_gt(a: f32, b: f32) -> bool { a > b }
fn float32_cmp_lte(a: f32, b: f32) -> bool { a <= b }
fn float32_cmp_gte(a: f32, b: f32) -> bool { a >= b }

fn int32_add(a: i32, b: i32) -> i32 { a + b }
fn int32_sub(a: i32, b: i32) -> i32 { a - b }
fn int32_mul(a: i32, b: i32) -> i32 { a * b }
fn int32_div(a: i32, b: i32) -> i32 { a / b }
fn int32_mod(a: i32, m: i32) -> i32 { (a % m + m) % m }

fn int32_cmp_eq(a: i32, b: i32) -> bool { a == b }
fn int32_cmp_lt(a: i32, b: i32) -> bool { a < b }
fn int32_cmp_gt(a: i32, b: i32) -> bool { a > b }
fn int32_cmp_lte(a: i32, b: i32) -> bool { a <= b }
fn int32_cmp_gte(a: i32, b: i32) -> bool { a >= b }

pub struct Interpreter {
    // TODO: don't separate registers.
    float32_registers: [f32, ..64],
    int32_registers: [i32, ..64],
    boolean_registers: [bool, ..64],
    boxed_registers: [boxed::Value, ..64],
    float_binary_functions: [fn(a: f32, b: f32) -> f32, ..4],
    float_cmp_functions: [fn(a: f32, b: f32) -> bool, ..5],
    int_binary_functions: [fn(a: i32, b: i32) -> i32, ..5],
    int_cmp_functions: [fn(a: i32, b: i32) -> bool, ..5],
}

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {
            float32_registers: [0.0, ..64],
            int32_registers: [0, ..64],
            boolean_registers: [false, ..64],
            boxed_registers: [boxed::Value::void(), ..64],
            float_binary_functions: [
                float32_add,
                float32_sub,
                float32_mul,
                float32_div,
            ],
            float_cmp_functions: [
                float32_cmp_eq,
                float32_cmp_lt,
                float32_cmp_gt,
                float32_cmp_lte,
                float32_cmp_gte,
            ],
            int_binary_functions: [
                int32_add,
                int32_sub,
                int32_mul,
                int32_div,
                int32_mod,
            ],
            int_cmp_functions: [
                int32_cmp_eq,
                int32_cmp_lt,
                int32_cmp_gt,
                int32_cmp_lte,
                int32_cmp_gte,
            ]
        }
    }

    pub fn exec(
        &mut self,
        script: &Script,
    ) -> Result<(), InterpreterError> {
        unsafe {
        let mut pc: ProgramCounter = 0;
        let code = &script.bytecode;

        loop {
            let op = code[pc];
            let operator = (op & OP_MASK) as uint;
            match operator as u8 {
                OP_NULL => { pc += 1; }
                OP_EXIT => { return Ok(()); }
                OP_JMP => { pc = code[pc+1] as uint; }
                OP_ADD...OP_DIV => {
                    let operator = operator - OP_ADD as uint;
                    match op & OP_TYPE_MASK {
                        OP_TYPE_INT32 => {
                            self.int32_registers[pc] = self.int_binary_functions[operator](
                                self.int32_registers[code[pc+1] as uint],
                                self.int32_registers[code[pc+2] as uint]
                            );
                        }
                        OP_TYPE_FLOAT32 => {
                            self.float32_registers[pc] = self.float_binary_functions[operator](
                                self.float32_registers[code[pc+1] as uint],
                                self.float32_registers[code[pc+2] as uint]
                            );
                        }
                        0 => {
                            let a = self.boxed_registers[code[pc+1] as uint];
                            let b = self.boxed_registers[code[pc+2] as uint];
                            if a.is_int32() && b.is_int32() {
                                self.boxed_registers[pc] = boxed::Value::int32(
                                    self.int_binary_functions[operator](
                                        a.get_int32_unchecked(),
                                        b.get_int32_unchecked()
                                    )
                                );
                            } else {
                                let fa = a.get_float32().unwrap();
                                let fb = b.get_float32().unwrap();
                                self.boxed_registers[pc] = boxed::Value::float32(
                                    self.float_binary_functions[operator](fa, fb)
                                );
                            }
                        }
                        _ => { fail!(); }
                    }
                    pc += 3;
                }
                OP_EQ...OP_GTE => {
                    let operator = operator - OP_EQ as uint;
                    match op & OP_TYPE_MASK {
                        OP_TYPE_INT32 => {
                            self.boolean_registers[pc] = self.int_cmp_functions[operator](
                                self.int32_registers[code[pc+1] as uint],
                                self.int32_registers[code[pc+2] as uint]
                            );
                        }
                        OP_TYPE_FLOAT32 => {
                            self.boolean_registers[pc] = self.float_cmp_functions[operator](
                                self.float32_registers[code[pc+1] as uint],
                                self.float32_registers[code[pc+2] as uint]
                            );
                        }
                        0 => {
                            let a = self.boxed_registers[code[pc+1] as uint];
                            let b = self.boxed_registers[code[pc+2] as uint];
                            if a.is_int32() && b.is_int32() {
                                self.boolean_registers[pc] = self.int_cmp_functions[operator](
                                    a.get_int32_unchecked(),
                                    b.get_int32_unchecked()
                                );
                            } else {
                                let fa = a.get_float32().unwrap();
                                let fb = b.get_float32().unwrap();
                                self.boolean_registers[pc] = self.float_cmp_functions[operator as uint](fa, fb);
                            }
                        }
                        _ => { fail!() }
                    }
                    pc += 3;
                }
                OP_CONST => {
                    match op & OP_TYPE_MASK {
                        OP_TYPE_INT32 => {
                            self.int32_registers[pc] = *unpack::<i32>(&code[pc+1]);
                            pc += 5;
                        }
                        OP_TYPE_FLOAT32 => {
                            self.float32_registers[pc] = *unpack::<f32>(&code[pc+1]);
                            pc += 5;
                        }
                        0 => {
                            self.boxed_registers[pc] = *unpack::<boxed::Value>(&code[pc+1]);
                            pc += 9;
                        }
                        _ => { fail!() }
                    }
                }
                OP_DBG => {
                    let addr = code[pc+1] as uint;  
                    match op & OP_TYPE_MASK {
                        OP_TYPE_INT32 => {
                            println!("dbg: int32 register {} = {}", addr, self.int32_registers[addr]);
                        }
                        OP_TYPE_FLOAT32 => {
                            println!("dbg: float32 register {} = {}", addr, self.float32_registers[addr]);
                        }
                        0 => {
                            println!("dbg: boxed register {} = {}", addr, self.boxed_registers[addr]);
                        }
                        _ => { fail!() }
                    }
                    pc += 2;
                }
                _ => { fail!(); }
            }
        }

        return Ok(());
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
    pub register_versions: Vec<u8>,
}

#[cfg(test)]
mod test {
    use ssa::bytecode::*;
    use super::*;

    #[test]
    fn simple_addition() {
        let mut code: Vec<u8> = Vec::new();
        /* 0*/code.push(OP_NULL);
        /* 1*/code.push(OP_CONST|OP_TYPE_INT32);
        /* 2*/code.push(42);
        /* 3*/code.push(0);
        /* 4*/code.push(0);
        /* 5*/code.push(0);
        /* 6*/code.push(OP_CONST|OP_TYPE_INT32);
        /* 7*/code.push(8);
        /* 8*/code.push(0);
        /* 9*/code.push(0);
        /*10*/code.push(0);
        /*11*/code.push(OP_ADD|OP_TYPE_INT32);
        /*12*/code.push(1);
        /*13*/code.push(6);
        /*14*/code.push(OP_DBG|OP_TYPE_INT32); // should print 50
        /*15*/code.push(11);
        /*16*/code.push(OP_EXIT);

        let script = Script {
            bytecode: code,
            register_versions: Vec::new(),
        };

        let mut interpreter = Interpreter::new();
        interpreter.exec(&script);
    }
}