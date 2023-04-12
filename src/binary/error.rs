#![allow(clippy::enum_variant_names)]
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid count of memory, must be 1")]
    InvalidMemoryCount,
    #[error("invalid count of table, must be 1")]
    InvalidTableCount,
    #[error("invalid elemtype of table, must be funcref, got {0}")]
    InvalidElmType(u8),
    #[error("invalid init expr instruction in expressions, got {0}")]
    InvalidInitExprOpcode(u8),
    #[error("invalid end instruction in expressions, got {0}")]
    InvalidInitExprEndOpcode(u8),
    #[error("invalid import kind at import section, got {0}")]
    InvalidImportKind(u8),
    #[error("invalid opecode: {0}")]
    InvalidOpcode(u8),
}
