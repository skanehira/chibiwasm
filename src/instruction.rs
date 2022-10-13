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
    I32Eq,
    I32Const,
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
            0x46 => Self::I32Eq,
            0x41 => Self::I32Const,
            _ => bail!("invalid opcode: {:x}", value),
        };
        Ok(op)
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Unreachable,
    Nop,
    LocalGet(u32),
    Call(u32),
    I32Sub,
    I32Add,
    I32Eq,
    I32Const(i32),
}
