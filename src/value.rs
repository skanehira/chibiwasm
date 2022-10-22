use crate::{
    instruction::Instruction,
    types::{FuncType, ValueType},
};
use std::fmt::Display;

// https://webassembly.github.io/spec/core/exec/runtime.html#syntax-val
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32(n) => {
                write!(f, "{}", n)
            }
            Self::I64(n) => {
                write!(f, "{}", n)
            }
            Self::F32(n) => {
                write!(f, "{}", n)
            }
            Self::F64(n) => {
                write!(f, "{}", n)
            }
        }
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Self::I32(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Self::I64(v)
    }
}

#[derive(Debug)]
pub struct Function {
    pub func_type: FuncType,
    pub body: Vec<Instruction>,
}
