
use nanboxed::*;
use bytecode::*;

use std::mem;

#[derive(Clone, Debug)]
pub enum InterpreterError {
    InvalidOpCode(u8),
    IncompatibleTypes(ByteCode, ValueType, ValueType),
    InvalidRegister,
    UnknowError
}

pub type InstructionPointer = usize;

pub fn exec(
    script: &Script,
    registers: &mut [Value],
    start: InstructionPointer
) -> Result<(), InterpreterError> {
    let mut pc = start;
    let code = &script.byte_code;
    loop {
        let op = code[pc];
        match op {
            OP_EXIT => { return Ok(()); }
            OP_NULL => { pc += 1 }
            OP_JMP => { pc = code[pc+1] as usize }
            OP_BRANCH => {
                let cond = code[pc+1] as usize;
                match registers[cond].get_boolean() {
                    Some(true) => { pc = code[pc+2] as usize; }
                    Some(false) => { pc += 3; }
                    None => { return Err(InterpreterError::IncompatibleTypes(op, registers[cond].get_type(), BOOLEAN_VALUE)); }
                }
            }
            OP_ADD | OP_SUB | OP_DIV | OP_MUL => {
                let a = code[pc+1] as usize;
                let b = code[pc+2] as usize;
                let dst = code[pc+3] as usize;

                let va = registers[a];
                let vb = registers[b];

                if va.get_type() != NUMBER_VALUE && vb.get_type() != NUMBER_VALUE {
                    return Err(InterpreterError::IncompatibleTypes(op, va.get_type(), vb.get_type()));
                }

                registers[dst] = match op {
                    OP_ADD => { Value::number(va.get_number().unwrap() + vb.get_number().unwrap()) }
                    OP_SUB => { Value::number(va.get_number().unwrap() - vb.get_number().unwrap()) }
                    OP_MUL => { Value::number(va.get_number().unwrap() * vb.get_number().unwrap()) }
                    OP_DIV => { Value::number(va.get_number().unwrap() / vb.get_number().unwrap()) }
                    OP_EQ  => { Value::boolean(va.get_number().unwrap() == vb.get_number().unwrap()) }
                    OP_GT  => { Value::boolean(va.get_number().unwrap() > vb.get_number().unwrap()) }
                    OP_GTE => { Value::boolean(va.get_number().unwrap() >= vb.get_number().unwrap()) }
                    _ => { panic!() }
                };

                pc += BINARY_OP_SIZE;
            }
            OP_NOT | OP_INC => {
                let a = code[pc+1] as usize;
                let dst = code[pc+2] as usize;

                let va = registers[a];

                registers[dst] = match op {
                    OP_INC => {
                        if va.get_type() != NUMBER_VALUE {
                            return Err(InterpreterError::IncompatibleTypes(op, va.get_type(), NUMBER_VALUE));
                        }
                        Value::number(va.get_number().unwrap() + 1.0)
                    }
                    OP_NOT => {
                        if va.get_type() == BOOLEAN_VALUE {
                            Value::boolean(!va.get_boolean().unwrap())
                        } else if va.get_type() == NUMBER_VALUE {
                            Value::boolean(va.get_number().unwrap() as i64 == 0)
                        } else {
                            return Err(InterpreterError::IncompatibleTypes(op, va.get_type(), BOOLEAN_VALUE));
                        }
                    }
                    _ => { panic!() }
                };

                pc += UNARY_OP_SIZE;
            }
            OP_CONST => {
                let dst = code[pc+1] as usize;
                let val = *unpack::<Value>(&code[pc+2]);
                registers[dst] = val;
                pc += 10;
            }
            OP_MOVE => {
                registers[code[pc+2] as usize] = registers[code[pc+1] as usize];
                pc += MOVE_OP_SIZE;
            }
            _ => {
                return Err(InterpreterError::InvalidOpCode(op));
            }
        }
    }
}

fn unpack<T>(ptr: &u8) -> &T {
    unsafe {
        let casted_ptr: &T = mem::transmute(ptr);
        return casted_ptr;
    }
}

#[cfg(test)]
mod tests {
    use nanboxed::*;
    use bytecode;
    use super::{exec};

    fn fuzzy_eq(a: f64, b: f64) -> bool { return (a - b).abs() < 0.000001; }

    #[test]
    fn empty_script() {
        let registers: &mut [Value] = &mut [];
        let mut builder = bytecode::ByteCodeBuilder::new();
        builder.exit();
        let script = builder.end();
        let status = exec(&script, registers, 0);
        assert!(status.is_ok());
    }

    #[test]
    fn empty_simple_addtion() {
        let registers = &mut [Value::void(); 3];
        let mut builder = bytecode::ByteCodeBuilder::new();
        builder.constant(0, Value::number(5.0));
        builder.constant(1, Value::number(10.0));
        builder.add(0, 1, 2);
        builder.exit();
        let script = builder.end();
        let status = exec(&script, registers, 0);
        match status {
            Ok(()) => {}
            Err(e) => { panic!("exec failed with error {:?}", e); }
        }
        let result = registers[2].get_number().unwrap();
        assert!(fuzzy_eq(result, 15.0));
    }
}
