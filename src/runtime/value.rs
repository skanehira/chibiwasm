use super::{float::*, integer::*};
use crate::binary::instruction::*;
use crate::binary::section::ExportDesc;
use crate::binary::types::FuncType;
use anyhow::Result;
use std::fmt::Display;

// https://webassembly.github.io/spec/core/exec/runtime.html#syntax-val
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl Value {
    pub fn is_true(&self) -> bool {
        match *self {
            Value::I32(v) => 1 == v,
            Value::I64(v) => 1 == v,
            _ => {
                panic!("cannot call is_true() when value is f32 or f64");
            }
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32(n) => {
                write!(f, "{n}")
            }
            Self::I64(n) => {
                write!(f, "{n}")
            }
            Self::F32(n) => {
                write!(f, "{n}")
            }
            Self::F64(n) => {
                write!(f, "{n}")
            }
        }
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Self::I32(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Self::F32(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Self::I64(v)
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        let v: i32 = v.try_into().unwrap();
        Self::I32(v)
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        let v: i64 = v.try_into().unwrap();
        Self::I64(v)
    }
}

#[derive(Debug)]
pub struct Function {
    pub func_type: FuncType,
    pub body: Vec<Instruction>,
}

#[derive(Debug)]
pub enum ExternalVal {
    Func(u32),
    Table(u32),
    Memory(u32),
    Global(u32),
}

impl From<ExportDesc> for ExternalVal {
    fn from(value: ExportDesc) -> Self {
        match value {
            ExportDesc::Func(addr) => Self::Func(addr),
            ExportDesc::Table(addr) => Self::Table(addr),
            ExportDesc::Memory(addr) => Self::Memory(addr),
            ExportDesc::Global(addr) => Self::Global(addr),
        }
    }
}

macro_rules! binop {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self, rhs: &Self) -> Result<Self> {
                match (self, rhs) {
                    (Value::I32(l), Value::I32(r)) => Ok(Value::I32(l.$op(*r)?)),
                    (Value::I64(l), Value::I64(r)) => Ok(Value::I64(l.$op(*r)?)),
                    (Value::F32(l), Value::F32(r)) => Ok(Value::F32(l.$op(*r)?)),
                    (Value::F64(l), Value::F64(r)) => Ok(Value::F64(l.$op(*r)?)),
                    _ => unimplemented!("unimplemented for Value")
                }
            }
        )*
    };
}

macro_rules! ibinop {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self, rhs: &Self) -> Result<Self> {
                match (self, rhs) {
                    (Value::I32(l), Value::I32(r)) => Ok(Value::I32(l.$op(*r)?)),
                    (Value::I64(l), Value::I64(r)) => Ok(Value::I64(l.$op(*r)?)),
                    _ => unimplemented!("unimplemented for Value")
                }
            }
        )*
    };
}

macro_rules! fbinop {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self, rhs: &Self) -> Result<Self> {
                match (self, rhs) {
                    (Value::F32(l), Value::F32(r)) => Ok(Value::F32(l.$op(*r)?)),
                    (Value::F64(l), Value::F64(r)) => Ok(Value::F64(l.$op(*r)?)),
                    _ => unimplemented!("unimplemented for Value")
                }
            }
        )*
    };
}

macro_rules! relop {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self, rhs: &Self) -> Result<Self> {
                match (self, rhs) {
                    (Value::I32(l), Value::I32(r)) => Ok(Value::I32(l.$op(*r)?)),
                    (Value::I64(l), Value::I64(r)) => Ok(Value::I32(l.$op(*r)? as i32)),
                    (Value::F32(l), Value::F32(r)) => Ok(Value::I32(l.$op(*r)? as i32)),
                    (Value::F64(l), Value::F64(r)) => Ok(Value::I32(l.$op(*r)? as i32)),
                    _ => unimplemented!("unimplemented div_s for Value")
                }
            }
        )*
    };
}

macro_rules! irelop {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self, rhs: &Self) -> Result<Self> {
                match (self, rhs) {
                    (Value::I32(l), Value::I32(r)) => Ok(Value::I32(l.$op(*r)?)),
                    (Value::I64(l), Value::I64(r)) => Ok(Value::I32(l.$op(*r)? as i32)),
                    _ => unimplemented!("unimplemented div_s for Value")
                }
            }
        )*
    };
}

macro_rules! frelop {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self, rhs: &Self) -> Result<Self> {
                match (self, rhs) {
                    (Value::F32(l), Value::F32(r)) => Ok(Value::I32(l.$op(*r)? as i32)),
                    (Value::F64(l), Value::F64(r)) => Ok(Value::I32(l.$op(*r)? as i32)),
                    _ => unimplemented!("unimplemented div_s for Value")
                }
            }
        )*
    };
}

macro_rules! iunop {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self) -> Result<Self> {
                match self {
                    Value::I32(l) => Ok(Value::I32(l.$op()?)),
                    Value::I64(l) => Ok(Value::I64(l.$op()?)),
                    _ => unimplemented!("unimplemented for Value")
                }
            }
        )*
    };
}

macro_rules! itestop {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self) -> Result<Self> {
                match self {
                    Value::I32(l) => Ok(Value::I32(l.$op()?)),
                    Value::I64(l) => Ok(Value::I32(l.$op()? as i32)),
                    _ => unimplemented!("unimplemented for Value")
                }
            }
        )*
    };
}

macro_rules! funop {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self) -> Result<Self> {
                match self {
                    Value::F32(l) => Ok(Value::F32(l.$op()?)),
                    Value::F64(l) => Ok(Value::F64(l.$op()?)),
                    _ => unimplemented!("unimplemented for Value")
                }
            }
        )*
    };
}

impl Value {
    binop!(add, sub, mul);

    ibinop!(div_s, div_u, rem_s, rem_u, and, or, xor, shl, shr_s, shr_u, rotl, rotr);

    // TODO: add copysign
    fbinop!(min, max, div, copysign);

    relop!(equal, not_equal);

    frelop!(flt, fgt, fle, fge);
    irelop!(lt_s, lt_u, gt_s, gt_u, le_s, le_u, ge_s, ge_u);

    iunop!(clz, ctz, extend8_s, extend16_s);
    funop!(abs, neg, sqrt, ceil, floor, trunc, nearest);

    itestop!(eqz);
}
