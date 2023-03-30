#![allow(unused)]

use super::indices::*;
use super::module::ModuleInst;
use super::{float::*, integer::*};
use crate::binary::instruction::*;
use crate::binary::types::ExportDesc;
use crate::binary::types::FuncType;
use anyhow::{Context as _, Result};
use std::fmt::Display;
use std::i64;
use std::mem::size_of;
use std::rc::Rc;

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

#[derive(Debug, Clone)]
pub struct Label {
    pub arity: usize, // argument or result? arity
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub arity: usize,       // result arity
    pub locals: Vec<Value>, // local variables
    pub labels: Vec<Label>,
}

// https://www.w3.org/TR/wasm-core-1/#stack%E2%91%A0
#[derive(Debug, Clone)]
pub enum StackValue {
    Value(Value),
    Label(Label),
    Frame(Frame),
}

// trait for stack access
pub trait StackAccess {
    fn pop1<T: From<StackValue> + std::fmt::Debug>(&mut self) -> Result<T>;
    fn pop_rl<T: From<StackValue> + std::fmt::Debug>(&mut self) -> Result<(T, T)>;
}

impl StackAccess for Vec<StackValue> {
    fn pop1<T: From<StackValue> + std::fmt::Debug>(&mut self) -> Result<T> {
        let value = self.pop().context("no value in the stack")?.into();
        Ok(value)
    }

    fn pop_rl<T: From<StackValue> + std::fmt::Debug>(&mut self) -> Result<(T, T)> {
        let r = self.pop1()?;
        let l = self.pop1()?;
        Ok((r, l))
    }
}

impl From<StackValue> for Value {
    fn from(value: StackValue) -> Self {
        match value {
            StackValue::Value(v) => v,
            _ => panic!("unexpected value: {:?}", value),
        }
    }
}

impl From<StackValue> for i32 {
    fn from(value: StackValue) -> Self {
        match value {
            StackValue::Value(v) => v.into(),
            _ => panic!("unexpected value: {:?}", value),
        }
    }
}

impl From<Value> for StackValue {
    fn from(value: Value) -> Self {
        Self::Value(value)
    }
}

impl From<Frame> for StackValue {
    fn from(value: Frame) -> Self {
        Self::Frame(value)
    }
}

impl From<Label> for StackValue {
    fn from(value: Label) -> Self {
        Self::Label(value)
    }
}

impl From<i32> for StackValue {
    fn from(value: i32) -> Self {
        Self::Value(value.into())
    }
}

impl From<i64> for StackValue {
    fn from(value: i64) -> Self {
        Self::Value(value.into())
    }
}

impl From<u32> for StackValue {
    fn from(value: u32) -> Self {
        Self::Value(value.into())
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

impl From<Value> for i32 {
    fn from(value: Value) -> Self {
        match value {
            Value::I32(v) => v,
            _ => panic!("value type is not i32: {:?}", value),
        }
    }
}

#[derive(Debug)]
pub enum ExternalVal {
    Func(FuncIdx),
    Table(TableIdx),
    Memory(MemoryIdx),
    Global(GlobalIdx),
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

#[derive(Debug)]
pub enum State {
    Continue,     // continue to next instruction
    Return,       // return from current frame
    Break(usize), // jump to the label
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
                    _ => panic!("unexpected value. left: {self} right: {rhs}")
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
                    _ => panic!("unexpected value. left: {self} right: {rhs}")
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
                    _ => panic!("unexpected value. left: {self} right: {rhs}")
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
                    _ => panic!("unexpected value. left: {self} right: {rhs}")
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
                    _ => panic!("unexpected value. left: {self} right: {rhs}")
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
                    _ => panic!("unexpected value. left: {self} right: {rhs}")
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
                    _ => panic!("unexpected value. {self}")
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
                    _ => panic!("unexpected value. {self}")
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
                    _ => panic!("unexpected value. {self}")
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

pub trait Numberic {
    fn read(buf: &[u8], addr: usize) -> Self;
    fn write(buf: &mut [u8], addr: usize, value: Self);
}

impl Numberic for i32 {
    fn read(buf: &[u8], addr: usize) -> Self {
        // TODO: Change to a non-copying approach.
        let mut bytes = [0u8; size_of::<i32>()];
        for i in 0..bytes.len() {
            bytes[i] = buf[addr + i];
        }
        i32::from_le_bytes(bytes)
    }

    fn write(buf: &mut [u8], addr: usize, value: Self) {
        let bytes = value.to_le_bytes();
        buf[addr..addr + size_of::<i32>()].copy_from_slice(&bytes);
    }
}
