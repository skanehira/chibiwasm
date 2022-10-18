use anyhow::bail;

// https://webassembly.github.io/spec/core/binary/instructions.html#expressions
#[derive(Debug)]
pub enum Opcode {
    Unreachable,
    Nop,
    LocalGet,
    Call,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32Eq,
    I32Eqz,
    I32Ne,
    I32LtS,
    I32LtU,
    I32Const,
    Return,
    If,
    Else,
    End,
    Void,
}

impl TryFrom<u8> for Opcode {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let op = match value {
            0x00 => Self::Unreachable,
            0x01 => Self::Nop,
            0x20 => Self::LocalGet,
            0x10 => Self::Call,
            0x6a => Self::I32Add,
            0x6b => Self::I32Sub,
            0x6c => Self::I32Mul,
            0x6D => Self::I32DivS,
            0x6E => Self::I32DivU,
            0x45 => Self::I32Eqz,
            0x46 => Self::I32Eq,
            0x47 => Self::I32Ne,
            0x48 => Self::I32LtS,
            0x49 => Self::I32LtU,
            0x41 => Self::I32Const,
            0x0f => Self::Return,
            0x04 => Self::If,
            0x05 => Self::Else,
            0x0b => Self::End,
            0x40 => Self::Void,
            _ => bail!("invalid opcode: {:x}", value),
        };
        Ok(op)
    }
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
    I32Const(i32),
    Return,
    If,
    Else,
    End,
    Void,
}
