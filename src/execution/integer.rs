use super::error::Error;
use crate::{ibinop, irelop, itestop, iunop};
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

iunop!(i32);
iunop!(i64);
ibinop!(i32);
ibinop!(i64);
irelop!(i32);
irelop!(i64);
itestop!();
