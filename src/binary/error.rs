use super::instruction::Opcode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid count of memory, must be 1")]
    InvalidMemoryCountError,
    #[error("invalid count of table, must be 1")]
    InvalidTableCountError,
    #[error("invalid elemtype of table, must be funcref, got {0}")]
    InvalidElmTypeError(u8),
    #[error("invalid init expr instruction in expressions, got {0:?}")]
    InvalidInitExprOpcodeError(Opcode),
    #[error("invalid end instruction in expressions, got {0:?}")]
    InvalidInitExprEndOpcodeError(Opcode),
}
