use anyhow::bail;
use num_derive::FromPrimitive;

// https://webassembly.github.io/spec/core/binary/instructions.html#expressions
#[derive(Debug, FromPrimitive)]
#[repr(u8)]
pub enum Opcode {
    Unreachable = 0x00,
    Nop = 0x01,
    LocalGet = 0x20,
    Call = 0x10,
    I32Add = 0x6a,
    I32Sub = 0x6b,
    I32Mul = 0x6c,
    I32DivS = 0x6D,
    I32DivU = 0x6E,
    I32Eqz = 0x45,
    I32Eq = 0x46,
    I32Ne = 0x47,
    I32LtS = 0x48,
    I32LtU = 0x49,
    I32GtS = 0x4A,
    I32GtU = 0x4B,
    I32LeS = 0x4C,
    I32LeU = 0x4D,
    I32GeS = 0x4E,
    I32GeU = 0x4F,
    I32Const = 0x41,
    Return = 0x0f,
    If = 0x04,
    Else = 0x05,
    End = 0x0b,
    Void = 0x40,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Unreachable,
    Nop,
    LocalGet(u32),
    Call(u32),
    I32Sub,
    I32Add,
    I32Mul,
    I32DivS,
    I32DivU,
    I32Eq,
    I32Eqz,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeU,
    I32GeS,
    I32Const(i32),
    Return,
    If,
    Else,
    End,
    Void,
}
