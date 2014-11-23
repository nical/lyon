use ssa::bytecode;
use ssa::interpreter::Script;
use boxed;

use std::mem;

pub struct CodeOffset {
    offset: u8,
}

pub struct Register {
    index: u8,
}

pub fn register(index: u8) -> Register { Register { index: index } }

pub struct Emitter {
    code: Vec<u8>
}

impl Emitter {
    pub fn new() -> Emitter {
        Emitter {
            code: Vec::new(),
        }
    }

    pub fn end(&mut self) -> Script {
        let mut script = Script {
            bytecode: Vec::new(),
        };
        mem::swap(&mut self.code, &mut script.bytecode);
        return script;
    }

    pub fn exit(&mut self) {
        self.code.push(bytecode::OP_EXIT);
    }

    pub fn operator(&mut self, op: u8,  a: CodeOffset, b: CodeOffset, register: Register) -> CodeOffset {
        let offset = self.code.len() as u8;
        self.code.push(op);
        self.code.push(register.index);
        self.code.push(a.offset);
        self.code.push(b.offset);
        CodeOffset {
            offset: offset,
        }
    }

    pub fn debug(&mut self, a: CodeOffset) -> CodeOffset {
        let offset = self.code.len() as u8;
        self.code.push(bytecode::OP_DBG);
        self.code.push(a.offset);
        CodeOffset {
            offset: offset,
        }
    }

    pub fn null(&mut self) {
        self.code.push(bytecode::OP_NULL);
    }

    pub fn mov(&mut self, from: CodeOffset, to: Register) -> CodeOffset {
        let offset = self.code.len() as u8;
        self.code.push(bytecode::OP_MOVE);
        self.code.push(from.offset);
        self.code.push(to.index);
        CodeOffset {
            offset: offset,
        }
    }

    pub fn jump(&mut self, to: u8) {
        self.code.push(bytecode::OP_JMP);
        self.code.push(to);
    }

    pub fn branch(&mut self, cond: CodeOffset, if_branch: CodeOffset) {
        self.code.push(bytecode::OP_BRANCH);
        self.code.push(cond.offset);
        self.code.push(if_branch.offset);
    }

    pub fn boxed_constant(&mut self, val: boxed::Value, register: Register) -> CodeOffset {
        let offset = self.code.len() as u8;
        self.code.push(bytecode::OP_CONST_BOXED);
        self.code.push(register.index);
        let val_bytes = val.get_bytes();

        self.code.push(val_bytes[0]);
        self.code.push(val_bytes[1]);
        self.code.push(val_bytes[2]);
        self.code.push(val_bytes[3]);
        self.code.push(val_bytes[4]);
        self.code.push(val_bytes[5]);
        self.code.push(val_bytes[6]);
        self.code.push(val_bytes[7]);
        CodeOffset {
            offset: offset,
        }
    }

    pub fn int32_constant(&mut self, val: i32, register: Register) -> CodeOffset {
        let offset = self.code.len() as u8;
        self.code.push(bytecode::OP_CONST_I32);
        self.code.push(register.index);
        unsafe {
            let val_bytes: &[u8, ..4] = mem::transmute(&val);

            self.code.push(val_bytes[0]);
            self.code.push(val_bytes[1]);
            self.code.push(val_bytes[2]);
            self.code.push(val_bytes[3]);
        }
        CodeOffset {
            offset: offset,
        }
    }

    pub fn float32_constant(&mut self, val: f32, register: Register) -> CodeOffset {
        let offset = self.code.len() as u8;
        self.code.push(bytecode::OP_CONST_I32);
        self.code.push(register.index);
        unsafe {
            let val_bytes: &[u8, ..4] = mem::transmute(&val);

            self.code.push(val_bytes[0]);
            self.code.push(val_bytes[1]);
            self.code.push(val_bytes[2]);
            self.code.push(val_bytes[3]);
        }
        CodeOffset {
            offset: offset,
        }
    }
}
