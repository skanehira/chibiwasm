use super::error::Error;
use anyhow::{bail, Result};

pub trait Iunop {
    fn clz(&self) -> Result<Self>
    where
        Self: Sized;
    fn ctz(&self) -> Result<Self>
    where
        Self: Sized;
    fn extend8_s(&self) -> Result<Self>
    where
        Self: Sized;
    fn extend16_s(&self) -> Result<Self>
    where
        Self: Sized;
}

pub trait Ibinop {
    fn add(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn sub(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn mul(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn div_s(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn div_u(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn rem_s(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn rem_u(&self, rhs: Self) -> Result<Self>
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
    fn shr_s(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn shr_u(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn rotl(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn rotr(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
}

pub trait Irelop {
    fn equal(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn not_equal(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn lt_s(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn lt_u(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn gt_s(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn gt_u(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn le_s(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn le_u(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn ge_s(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn ge_u(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
}

pub trait Itestop {
    fn eqz(&self) -> Result<i32>
    where
        Self: Sized;
}

macro_rules! iunop {
    () => {
        fn clz(&self) -> Result<Self> {
            Ok(self.leading_zeros() as Self)
        }
        fn ctz(&self) -> Result<Self> {
            Ok(self.trailing_zeros() as Self)
        }
    };
    (i32) => {
        impl Iunop for i32 {
            iunop!();
            fn extend8_s(&self) -> Result<Self> {
                Ok(self << 24 >> 24)
            }
            fn extend16_s(&self) -> Result<Self> {
                Ok(self << 16 >> 16)
            }
        }
    };
    (i64) => {
        impl Iunop for i64 {
            iunop!();
            fn extend8_s(&self) -> Result<Self> {
                Ok(self << 56 >> 56)
            }
            fn extend16_s(&self) -> Result<Self> {
                Ok(self << 48 >> 48)
            }
        }
    };
}

macro_rules! ibinop {
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
        fn rem_s(&self, rhs: Self) -> Result<Self> {
            if rhs == 0 {
                bail!(Error::IntegerDivideByZero);
            }
            Ok(self.wrapping_rem(rhs) as Self)
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
        fn shr_s(&self, rhs: Self) -> Result<Self> {
            Ok((*self).wrapping_shr(rhs as u32))
        }
        fn rotl(&self, rhs: Self) -> Result<Self> {
            Ok((*self).rotate_left(rhs as u32))
        }
        fn rotr(&self, rhs: Self) -> Result<Self> {
            Ok((*self).rotate_right(rhs as u32))
        }
    };
    (i32) => {
        impl Ibinop for i32 {
            ibinop!();
            fn div_u(&self, rhs: Self) -> Result<Self> {
                if rhs == 0 {
                    bail!(Error::IntegerDivideByZero);
                }
                Ok(u32::wrapping_div(*self as u32, rhs as u32) as Self)
            }
            fn rem_u(&self, rhs: Self) -> Result<Self> {
                if rhs == 0 {
                    bail!(Error::IntegerDivideByZero);
                }
                Ok((*self as u32).wrapping_rem(rhs as u32) as Self)
            }
            fn shr_u(&self, rhs: Self) -> Result<Self> {
                Ok((*self as u32).wrapping_shr(rhs as u32) as Self)
            }
        }
    };
    (i64) => {
        impl Ibinop for i64 {
            ibinop!();
            fn div_u(&self, rhs: Self) -> Result<Self> {
                if rhs == 0 {
                    bail!(Error::IntegerDivideByZero);
                }
                Ok(u64::wrapping_div(*self as u64, rhs as u64) as Self)
            }
            fn rem_u(&self, rhs: Self) -> Result<Self> {
                if rhs == 0 {
                    bail!(Error::IntegerDivideByZero);
                }
                Ok((*self as u64).wrapping_rem(rhs as u64) as Self)
            }
            fn shr_u(&self, rhs: Self) -> Result<Self> {
                Ok((*self as u64).wrapping_shr(rhs as u32) as Self)
            }
        }
    };
}

macro_rules! irelop {
    () => {
        fn equal(&self, rhs: Self) -> Result<i32> {
            Ok((*self == rhs) as i32)
        }
        fn not_equal(&self, rhs: Self) -> Result<i32> {
            Ok((*self != rhs) as i32)
        }
        fn lt_s(&self, rhs: Self) -> Result<i32> {
            Ok((*self < rhs) as i32)
        }
        fn gt_s(&self, rhs: Self) -> Result<i32> {
            Ok((*self > rhs) as i32)
        }
        fn le_s(&self, rhs: Self) -> Result<i32> {
            Ok((*self <= rhs) as i32)
        }
        fn ge_s(&self, rhs: Self) -> Result<i32> {
            Ok((*self >= rhs) as i32)
        }
    };
    (i32) => {
        impl Irelop for i32 {
            irelop!();
            fn lt_u(&self, rhs: Self) -> Result<i32> {
                Ok((*self as u32).lt(&(rhs as u32)) as i32)
            }
            fn gt_u(&self, rhs: Self) -> Result<i32> {
                Ok((*self as u32).gt(&(rhs as u32)) as i32)
            }
            fn le_u(&self, rhs: Self) -> Result<i32> {
                Ok((*self as u32).le(&(rhs as u32)) as i32)
            }
            fn ge_u(&self, rhs: Self) -> Result<i32> {
                Ok((*self as u32).ge(&(rhs as u32)) as i32)
            }
        }
    };
    (i64) => {
        impl Irelop for i64 {
            irelop!();
            fn lt_u(&self, rhs: Self) -> Result<i32> {
                Ok((*self as u64).lt(&(rhs as u64)) as i32)
            }
            fn gt_u(&self, rhs: Self) -> Result<i32> {
                Ok((*self as u64).gt(&(rhs as u64)) as i32)
            }
            fn le_u(&self, rhs: Self) -> Result<i32> {
                Ok((*self as u64).le(&(rhs as u64)) as i32)
            }
            fn ge_u(&self, rhs: Self) -> Result<i32> {
                Ok((*self as u64).ge(&(rhs as u64)) as i32)
            }
        }
    };
}

macro_rules! itestop {
    () => {
        impl Itestop for i32 {
            fn eqz(&self) -> Result<i32> {
                Ok((*self == 0) as i32)
            }
        }

        impl Itestop for i64 {
            fn eqz(&self) -> Result<i32> {
                Ok((*self == 0) as i32)
            }
        }
    };
}

iunop!(i32);
iunop!(i64);
ibinop!(i32);
ibinop!(i64);
irelop!(i32);
irelop!(i64);
itestop!();
