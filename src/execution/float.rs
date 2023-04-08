use crate::{fbinop, frelop, funop};
use anyhow::Result;

// Ref: https://www.w3.org/TR/wasm-core-1/#numeric-instructions%E2%91%A0
pub trait Funop {
    fn abs(&self) -> Result<Self>
    where
        Self: Sized;
    fn neg(&self) -> Result<Self>
    where
        Self: Sized;
    fn sqrt(&self) -> Result<Self>
    where
        Self: Sized;
    fn ceil(&self) -> Result<Self>
    where
        Self: Sized;
    fn floor(&self) -> Result<Self>
    where
        Self: Sized;
    fn trunc(&self) -> Result<Self>
    where
        Self: Sized;
    fn nearest(&self) -> Result<Self>
    where
        Self: Sized;
}

pub trait Fbinop {
    fn add(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn sub(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn mul(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn div(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn min(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn max(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
    fn copysign(&self, rhs: Self) -> Result<Self>
    where
        Self: Sized;
}

pub trait Frelop {
    fn equal(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn not_equal(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn flt(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn fgt(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn fle(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
    fn fge(&self, rhs: Self) -> Result<i32>
    where
        Self: Sized;
}

funop!(f32, f64);
fbinop!(f32, f64);
frelop!(f32, f64);
