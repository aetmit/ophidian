use frontend::analysis::hir::ConversionKind;
use num_enum::TryFromPrimitive;

#[repr(u8)]
#[derive(TryFromPrimitive)]
pub enum OpCode {
    // halt the vm with exit code that we assume
    // the last opcode has pushed onto the stack
    Halt = 0x00,

    // i for integer
    // e.g. iadd is integer addition
    // the following opcodes pop 2 values of the type of the opcode off
    // the stack, apply the operation and pushes the result
    I32Add = 0x01,
    I32Sub = 0x02,
    I32Mul = 0x03,
    I32Div = 0x04,

    // integer negation
    I32Negate = 0x05,

    // load a constant from the constant pool
    // requires an operand that is 3 bytes as the index
    // encoded in little endian
    LoadConst = 0x06,

    // print an integer from the top of the stack
    I32Print = 0x07,

    // pops a value from the stack and discards it
    Pop = 0x08,

    // takes a 3 byte operand and indexes into local pool and pushes the integer value at index operand
    I32LoadLocal = 0x09,

    // takes a 3 byte operand, then pops an int off the stack then adds the value into local pool at index
    I32StoreLocal = 0x0A,

    // print a boolean from the top of the stack
    BPrint = 0x0B,

    // takes a 3 byte operand and indexes into local pool and pushes the boolean value at index operand
    BLoadLocal = 0x0C,

    // takes a 3 byte operand, then pops an bool off the stack then adds the value into local pool at index
    BStoreLocal = 0x0D,

    // integer equality
    I32Equal = 0x0E,
    I32NEqual = 0x0F,

    // boolean equality
    BEqual = 0x10,
    BNEqual = 0x11,

    // integer comparison
    I32Less = 0x12,
    I32LessEq = 0x13,
    I32Greater = 0x14,
    I32GreaterEq = 0x15,

    // jumps
    // these jumps take a FOUR byte operand (not to be confused with the 3 bytes used for other opcodes)
    Jmp = 0x16,
    JmpTrue = 0x17,
    JmpFalse = 0x18,

    // duplicate the value at the top of the stack
    Dup = 0x19,

    // double arithmetic operations
    // works by popping 2 values off the stack, applying the operation then pushing the result
    F64Add = 0x1A,
    F64Sub = 0x1B,
    F64Mul = 0x1C,
    F64Div = 0x1D,

    F64Negate = 0x1E,

    F64Print = 0x1F,

    F64LoadLocal = 0x20,
    F64StoreLocal = 0x21,

    F64Equal = 0x22,
    F64NEqual = 0x23,

    F64Less = 0x24,
    F64LessEq = 0x25,
    F64Greater = 0x26,
    F64GreaterEq = 0x27,

    // convert f64 a.k.a. double to i32 a.k.a. int
    I32ToF64 = 0x28,

    // call a function
    // takes a 3 byte operand and uses that to index into a function lookup table which provides
    // metadata needed to execute the function
    Call = 0x29,

    // returns from a function that DOESN'T return anything
    Return = 0x2A,
    // returns from a function that DOES return exactly ONE value
    ReturnVal = 0x2B,
}

impl From<ConversionKind> for OpCode {
    fn from(value: ConversionKind) -> Self {
        match value {
            ConversionKind::IntToDouble => {
                OpCode::I32ToF64
            }
        }        
    }
}
