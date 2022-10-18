use crate::{
    instruction::Instruction,
    types::{FuncType, ValueType},
};
use std::fmt::Display;

// https://webassembly.github.io/spec/core/exec/runtime.html#syntax-val
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    Num(Number),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Num(number) => {
                write!(f, "{}", number)
            }
            Value::Num(_) => todo!(),
        }
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Self::Num(Number::I32(v))
    }
}

impl std::ops::Add for Value {
    type Output = Value;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Num(Number::I32(a)), Self::Num(Number::I32(b))) => {
                Value::Num(Number::I32(a + b))
            }
            _ => unimplemented!("cannot add values"),
        }
    }
}

impl std::ops::Sub for Value {
    type Output = Value;
    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Num(Number::I32(a)), Self::Num(Number::I32(b))) => {
                Value::Num(Number::I32(a - b))
            }
            _ => unimplemented!("cannot sub values"),
        }
    }
}

impl std::ops::Mul for Value {
    type Output = Value;
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Num(Number::I32(a)), Self::Num(Number::I32(b))) => {
                Value::Num(Number::I32(a * b))
            }
            _ => unimplemented!("cannot mul values"),
        }
    }
}

impl std::ops::Div for Value {
    type Output = Value;
    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Num(Number::I32(a)), Self::Num(Number::I32(b))) => {
                Value::Num(Number::I32(a / b))
            }
            _ => unimplemented!("cannot mul values"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Number {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::I32(v) => write!(f, "{}", v),
            Number::I64(v) => write!(f, "{}", v),
            Number::F32(v) => write!(f, "{}", v),
            Number::F64(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Debug)]
pub struct Function {
    pub func_type: FuncType,
    pub body: Vec<Instruction>,
}
