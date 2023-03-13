use crate::{error::Error, instruction::Instruction, types::FuncType};
use anyhow::{bail, Result};
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

pub trait IntegerNumberic {
    fn add(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn sub(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn mul(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn equal(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn equalz(&self) -> Result<i32>
    where
        Self: Sized;
    fn div_u(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn div_s(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn clz(&self) -> Result<Self>
    where
        Self: Sized;
    fn ctz(&self) -> Result<Self>
    where
        Self: Sized;
    fn not_equal(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn lts(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn ltu(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn gts(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn gtu(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn les(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn leu(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn ges(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn geu(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn rems(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn remu(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn and(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn or(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn xor(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn shl(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn shru(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn shrs(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn rtol(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn rtor(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn extend8_s(&self) -> Result<Self>
    where
        Self: Sized;
    fn extend16_s(&self) -> Result<Self>
    where
        Self: Sized;
}

macro_rules! impl_numberic {
    () => {
        fn add(&self, rhs: Self) -> Result<Self> {
            Ok(self.wrapping_add(rhs))
        }
        fn sub(&self, rhs: Self) -> Result<Self> {
            Ok(self.wrapping_sub(rhs))
        }
        fn mul(&self, rhs: Self) -> Result<Self> {
            Ok(self.wrapping_mul(rhs))
        }
        fn div_s(&self, rhs: Self) -> Result<Self> {
            if rhs == 0 {
                bail!(Error::IntegerDivideByZero);
            }
            match self.checked_div(rhs) {
                Some(v) => Ok(v),
                None => bail!(Error::DivisionOverflow),
            }
        }
        fn rems(&self, rhs: Self) -> Result<Self> {
            if rhs == 0 {
                bail!(Error::IntegerDivideByZero);
            }
            Ok(self.wrapping_rem(rhs) as Self)
        }
        fn clz(&self) -> Result<Self> {
            Ok(self.leading_zeros() as Self)
        }
        fn ctz(&self) -> Result<Self> {
            Ok(self.trailing_zeros() as Self)
        }
        fn equal(&self, rhs: Self) -> Result<i32> {
            Ok((*self == rhs) as i32)
        }
        fn equalz(&self) -> Result<i32> {
            Ok((*self == 0) as i32)
        }
        fn not_equal(&self, rhs: Self) -> Result<i32> {
            Ok((*self != rhs) as i32)
        }
        fn lts(&self, rhs: Self) -> Result<i32> {
            Ok((*self < rhs) as i32)
        }
        fn gts(&self, rhs: Self) -> Result<i32> {
            Ok((*self > rhs) as i32)
        }
        fn les(&self, rhs: Self) -> Result<i32> {
            Ok((*self <= rhs) as i32)
        }
        fn ges(&self, rhs: Self) -> Result<i32> {
            Ok((*self >= rhs) as i32)
        }
        fn and(&self, rhs: Self) -> Result<Self> {
            Ok((*self & rhs) as Self)
        }
        fn or(&self, rhs: Self) -> Result<Self> {
            Ok((*self | rhs) as Self)
        }
        fn xor(&self, rhs: Self) -> Result<Self> {
            Ok((*self ^ rhs) as Self)
        }
        fn shl(&self, rhs: Self) -> Result<Self> {
            Ok((*self).wrapping_shl(rhs as u32))
        }
        fn shrs(&self, rhs: Self) -> Result<Self> {
            Ok((*self).wrapping_shr(rhs as u32))
        }
        fn rtol(&self, rhs: Self) -> Result<Self> {
            Ok((*self).rotate_left(rhs as u32))
        }
        fn rtor(&self, rhs: Self) -> Result<Self> {
            Ok((*self).rotate_right(rhs as u32))
        }
    };
}

impl IntegerNumberic for i32 {
    impl_numberic!();
    fn div_u(&self, rhs: Self) -> Result<Self> {
        if rhs == 0 {
            bail!(Error::IntegerDivideByZero);
        }
        Ok(u32::wrapping_div(*self as u32, rhs as u32) as Self)
    }
    fn ltu(&self, rhs: Self) -> Result<i32> {
        Ok((*self as u32).lt(&(rhs as u32)) as i32)
    }
    fn gtu(&self, rhs: Self) -> Result<i32> {
        Ok((*self as u32).gt(&(rhs as u32)) as i32)
    }
    fn leu(&self, rhs: Self) -> Result<i32> {
        Ok((*self as u32).le(&(rhs as u32)) as i32)
    }
    fn geu(&self, rhs: Self) -> Result<i32> {
        Ok((*self as u32).ge(&(rhs as u32)) as i32)
    }
    fn remu(&self, rhs: Self) -> Result<Self> {
        if rhs == 0 {
            bail!(Error::IntegerDivideByZero);
        }
        Ok((*self as u32).wrapping_rem(rhs as u32) as Self)
    }
    fn shru(&self, rhs: Self) -> Result<Self> {
        Ok((*self as u32).wrapping_shr(rhs as u32) as Self)
    }
    fn extend8_s(&self) -> Result<Self> {
        Ok(self << 24 >> 24)
    }
    fn extend16_s(&self) -> Result<Self> {
        Ok(self << 16 >> 16)
    }
}

impl IntegerNumberic for i64 {
    impl_numberic!();
    fn div_u(&self, rhs: Self) -> Result<Self> {
        if rhs == 0 {
            bail!(Error::IntegerDivideByZero);
        }
        Ok(u64::wrapping_div(*self as u64, rhs as u64) as Self)
    }
    fn ltu(&self, rhs: Self) -> Result<i32> {
        Ok((*self as u64).lt(&(rhs as u64)) as i32)
    }
    fn gtu(&self, rhs: Self) -> Result<i32> {
        Ok((*self as u64).gt(&(rhs as u64)) as i32)
    }
    fn leu(&self, rhs: Self) -> Result<i32> {
        Ok((*self as u64).le(&(rhs as u64)) as i32)
    }
    fn geu(&self, rhs: Self) -> Result<i32> {
        Ok((*self as u64).ge(&(rhs as u64)) as i32)
    }
    fn remu(&self, rhs: Self) -> Result<Self> {
        if rhs == 0 {
            bail!(Error::IntegerDivideByZero);
        }
        Ok((*self as u64).wrapping_rem(rhs as u64) as Self)
    }
    fn shru(&self, rhs: Self) -> Result<Self> {
        Ok((*self as u64).wrapping_shr(rhs as u32) as Self)
    }
    fn extend8_s(&self) -> Result<Self> {
        Ok(self << 56 >> 56)
    }
    fn extend16_s(&self) -> Result<Self> {
        Ok(self << 48 >> 48)
    }
}

macro_rules! impl_compare {
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

macro_rules! impl_binary {
    ($($op: ident),*) => {
        $(
            pub fn $op(&self, rhs: &Self) -> Result<Self> {
                match (self, rhs) {
                    (Value::I32(l), Value::I32(r)) => Ok(Value::I32(l.$op(*r)?)),
                    (Value::I64(l), Value::I64(r)) => Ok(Value::I64(l.$op(*r)?)),
                    _ => unimplemented!("unimplemented for Value")
                        //(Value::F32(l), Value::F32(r)) => Ok(Value::F32(l.div_s(*r)?)),
                        //(Value::F64(l), Value::F64(r)) => Ok(Value::F64(l.div_s(*r)?)),
                }
            }
        )*
    };
}

macro_rules! impl_unary {
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

impl Value {
    impl_unary!(clz, ctz, extend8_s, extend16_s);
    impl_binary!(
        add, sub, mul, div_u, div_s, rems, remu, and, or, xor, shl, shru, shrs, rtol, rtor
    );
    impl_compare!(equal, not_equal, lts, ltu, gts, gtu, les, leu, ges, geu);

    pub fn equalz(&self) -> Result<Self> {
        match self {
            Value::I32(l) => Ok(Value::I32(l.equalz()?)),
            Value::I64(l) => Ok(Value::I32(l.equalz()?)),
            _ => unimplemented!("unimplemented for Value"),
        }
    }
}
