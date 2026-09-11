use macros::Constructor;

use crate::chunk::Chunk;
use crate::opcodes::OpCode;
use crate::stack::Stack;
use crate::value::{Value, ValueKind};

pub type VMExitCode = i32;

struct CallFrame {
    return_ip: *const u8,
    base: usize,
}

impl CallFrame {
    pub fn new(return_ip: *const u8, base: usize) -> Self {
        Self { return_ip, base }
    }
}

#[derive(Debug, Clone, Copy, Constructor)]
pub struct RuntimeFunction {
    start: *const u8,
    arity: usize,
    local_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeFunctionId(pub usize);

// safe wrapper that ties pointer lifetime to the chunk
// struct Reader<'a> {
// ptr: *const u8,
// _marker: std::marker::PhantomData<&'a u8>,
// }

// impl<'a> Reader<'a> {
//     pub fn new(chunk: &'a Chunk) -> Self {
//         Self { ptr: chunk.bytecode.as_ptr(), _marker: std::marker::PhantomData }
//     }

//     #[inline(always)]
//     pub unsafe fn read_byte(&mut self) -> u8 {
//         let byte = unsafe { *self.ptr };
//         self.ptr = unsafe {
//             self.ptr.add(1)
//         };
//         byte
//     }
// }

pub struct VirtualMachine {
    // stack that bytecode operates on
    stack: Stack<Value>,
    // pointer to instruction to be executed
    ip: *const u8,

    // call stack
    frames: Stack<CallFrame>,

    functions: Vec<RuntimeFunction>,
    // reader: Reader<'chunk>,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            stack: Stack::new(),
            frames: Stack::new(),
            ip: std::ptr::null(),
            functions: Vec::new(),
            // reader: Reader::new(),
        }
    }

    pub fn execute(&mut self, chunk: &Chunk) -> VMExitCode {
        // TODO: jump to main function
        self.ip = chunk.bytecode_ptr();

        loop {
            // we unwrap here because we assume that the bytecode is correct
            // remember to add new match cases to OpCode::try_from() when adding
            // new opcodes
            let opcode = OpCode::try_from(self.read_byte())
                .expect("try checking the try_from() function for Opcode in opcode.rs");
            match opcode {
                OpCode::Halt => {
                    let exit_code = self.pop();
                    // behaviour is undefined if exit code is not an integer
                    // similar to c's `exit()` function from <stdlib.h>
                    return unsafe { exit_code.data.integer };
                }
                OpCode::I32Add => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer + b.data.integer };
                    let value = Value::new_int(res);
                    self.push(value);
                }
                OpCode::I32Sub => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer - b.data.integer };
                    let value = Value::new_int(res);
                    self.push(value);
                }
                OpCode::I32Mul => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer * b.data.integer };
                    let value = Value::new_int(res);
                    self.push(value);
                }
                OpCode::I32Div => {
                    // division by zero is undefined behaviour
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer / b.data.integer };
                    let value = Value::new_int(res);
                    self.push(value);
                }
                OpCode::I32Negate => {
                    let a = self.pop();
                    let res = unsafe { -a.data.integer };
                    let value = Value::new_int(res);
                    self.push(value);
                }
                OpCode::LoadConst => {
                    let b0 = self.read_byte();
                    let b1 = self.read_byte();
                    let b2 = self.read_byte();
                    let idx = decode_u24_le([b0, b1, b2]);
                    // we unwrap here because we assume the index provided is valid
                    let constant = chunk.constants.get(idx as usize).unwrap();

                    self.push(constant.clone());
                }
                OpCode::Pop => {
                    let _ = self.pop();
                }
                OpCode::I32Print => {
                    let v = self.pop();
                    let int = unsafe { v.data.integer };
                    println!("{}", int);
                }
                OpCode::I32LoadLocal => {
                    let b0 = self.read_byte();
                    let b1 = self.read_byte();
                    let b2 = self.read_byte();
                    let idx = decode_u24_le([b0, b1, b2]);
                    let value = self.stack[idx as usize + self.current_base()];
                    self.push(value);
                }
                OpCode::I32StoreLocal => {
                    let value = self.pop();
                    if value.kind == ValueKind::Uninitialized {
                        panic!("use of unitialized variable");
                    }
                    let b0 = self.read_byte();
                    let b1 = self.read_byte();
                    let b2 = self.read_byte();
                    let idx = decode_u24_le([b0, b1, b2]);
                    let base = self.current_base();
                    self.stack[base + idx as usize] = value;
                    self.push(value);
                }
                OpCode::BEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.boolean == b.data.boolean };
                    let v = Value::new_bool(res);
                    self.push(v);
                }
                OpCode::BLoadLocal => {
                    let b0 = self.read_byte();
                    let b1 = self.read_byte();
                    let b2 = self.read_byte();
                    let idx = decode_u24_le([b0, b1, b2]);
                    let value = self.stack[(idx as usize) + self.current_base()];
                    self.push(value);
                }
                OpCode::BNEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.boolean != b.data.boolean };
                    let v = Value::new_bool(res);
                    self.push(v);
                }
                OpCode::BPrint => {
                    let v = self.pop();
                    let boolean = unsafe { v.data.boolean };
                    println!("{}", boolean);
                }
                OpCode::BStoreLocal => {
                    let value = self.pop();
                    if value.kind == ValueKind::Uninitialized {
                        panic!("use of uninitialized variable");
                    }
                    let b0 = self.read_byte();
                    let b1 = self.read_byte();
                    let b2 = self.read_byte();
                    let idx = decode_u24_le([b0, b1, b2]);
                    let base = self.current_base();
                    self.stack[base + (idx as usize)] = value;
                    self.push(value);
                }
                OpCode::I32Equal => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer == b.data.integer };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::I32Greater => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer > b.data.integer };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::I32GreaterEq => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer >= b.data.integer };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::I32Less => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer < b.data.integer };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::I32LessEq => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer <= b.data.integer };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::I32NEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.integer != b.data.integer };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::JmpTrue => {
                    let value = self.pop();
                    let cond = unsafe { value.data.boolean };
                    let b0 = self.read_byte();
                    let b1 = self.read_byte();
                    let b2 = self.read_byte();
                    let b3 = self.read_byte();
                    if cond {
                        let offset = i32::from_le_bytes([b0, b1, b2, b3]);
                        unsafe {
                            self.jump(offset);
                        }
                    }
                }
                OpCode::Jmp => {
                    let b0 = self.read_byte();
                    let b1 = self.read_byte();
                    let b2 = self.read_byte();
                    let b3 = self.read_byte();
                    let offset = i32::from_le_bytes([b0, b1, b2, b3]);
                    unsafe {
                        self.jump(offset);
                    }
                }
                OpCode::JmpFalse => {
                    let value = self.pop();
                    let cond = unsafe { value.data.boolean };
                    let b0 = self.read_byte();
                    let b1 = self.read_byte();
                    let b2 = self.read_byte();
                    let b3 = self.read_byte();

                    if !cond {
                        let offset = i32::from_le_bytes([b0, b1, b2, b3]);
                        unsafe {
                            self.jump(offset);
                        }
                    }
                }
                OpCode::Dup => {
                    let v = self.pop();
                    self.push(v);
                    self.push(v);
                }
                OpCode::F64Add => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double + b.data.double };
                    let value = Value::new_double(res);
                    self.push(value);
                }
                OpCode::F64Div => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double / b.data.double };
                    let value = Value::new_double(res);
                    self.push(value);
                }
                OpCode::F64Equal => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double == b.data.double };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::F64Greater => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double > b.data.double };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::F64GreaterEq => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double >= b.data.double };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::F64Less => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double < b.data.double };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::F64LessEq => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double <= b.data.double };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::F64LoadLocal => {
                    let bytes = [self.read_byte(), self.read_byte(), self.read_byte()];
                    let idx = decode_u24_le(bytes);
                    let value = self.stack[self.current_base() + (idx as usize)];
                    self.push(value);
                }
                OpCode::F64Mul => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double * b.data.double };
                    let value = Value::new_double(res);
                    self.push(value);
                }
                OpCode::F64NEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double != b.data.double };
                    let value = Value::new_bool(res);
                    self.push(value);
                }
                OpCode::F64Negate => {
                    let a = self.pop();
                    let res = unsafe { -a.data.double };
                    let value = Value::new_double(res);
                    self.push(value);
                }
                OpCode::F64Print => {
                    let val = self.pop();
                    let double = unsafe { val.data.double };
                    println!("{}", double);
                }
                OpCode::F64StoreLocal => {
                    let value = self.pop();
                    if value.kind == ValueKind::Uninitialized {
                        panic!("use of unitialized variable");
                    }
                    let b0 = self.read_byte();
                    let b1 = self.read_byte();
                    let b2 = self.read_byte();
                    let idx = decode_u24_le([b0, b1, b2]);
                    let base = self.current_base();
                    self.stack[base + (idx as usize)] = value;
                    self.push(value);
                }
                OpCode::F64Sub => {
                    let b = self.pop();
                    let a = self.pop();
                    let res = unsafe { a.data.double - b.data.double };
                    let value = Value::new_double(res);
                    self.push(value);
                }
                OpCode::I32ToF64 => {
                    let a = self.pop();
                    let res = unsafe { a.data.integer as f64 };
                    let value = Value::new_double(res);
                    self.push(value);
                }
                OpCode::Call => {
                    let bytes = [self.read_byte(), self.read_byte(), self.read_byte()];
                    let idx = decode_u24_le(bytes);
                    let function = self.functions[idx as usize];
                    let base = self.stack.len() - function.arity;
                    self.frames.push(CallFrame::new(self.ip, base));

                    self.stack.extend(std::iter::repeat_n(
                        Value::UNINITIALIZED,
                        function.local_count,
                    ));

                    self.set_ip(function.start);
                }
                OpCode::Return => {
                    let frame = self.frames.pop();
                    self.stack.truncate(frame.base);
                    self.set_ip(frame.return_ip);
                }
                OpCode::ReturnVal => {
                    let frame = self.frames.pop();
                    let return_value = self.pop();
                    self.stack.truncate(frame.base);
                    self.push(return_value);
                    self.set_ip(frame.return_ip);
                }
            }
        }
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop()
    }

    fn read_byte(&mut self) -> u8 {
        unsafe {
            let byte = *self.ip;
            self.ip = self.ip.add(1);
            byte
        }
    }

    fn current_base(&self) -> usize {
        self.frames.top().expect("empty stack").base
    }

    fn set_ip(&mut self, pos: *const u8) {
        self.ip = pos;
    }

    unsafe fn jump(&mut self, offset: i32) {
        if offset >= 0 {
            self.ip = unsafe { self.ip.add(offset as usize) };
        } else {
            self.ip = unsafe { self.ip.sub((-offset) as usize) };
        }
    }
}

fn decode_u24_le(bytes: [u8; 3]) -> u32 {
    let padded = [bytes[0], bytes[1], bytes[2], 0];
    u32::from_le_bytes(padded)
}
