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

macro_rules! funop {
    ($($ty: ty),+) => {
        $(
            impl Funop for $ty {
                fn abs(&self) -> Result<Self> {
                    Ok((*self).abs())
                }
                fn neg(&self) -> Result<Self> {
                    Ok(-(*self))
                }
                fn sqrt(&self) -> Result<Self> {
                    if (*self) == 0.0 {
                        return Ok(0.0);
                    }
                    Ok((*self).sqrt())
                }
                fn ceil(&self) -> Result<Self> {
                    Ok(num_traits::real::Real::ceil(*self))
                }
                fn floor(&self) -> Result<Self> {
                    Ok((*self).floor())
                }
                fn trunc(&self) -> Result<Self> {
                    Ok((*self).trunc())
                }
                fn nearest(&self) -> Result<Self> {
                    let abs = (*self).abs();
                    if (0.0..=0.5).contains(&abs) {
                        return Ok(0.0);
                    }
                    let rounded = (*self).round();
                    let value = match rounded as i64 % 2 {
                        r if r == 1 => self.floor().unwrap(),
                        r if r == -1 => self.ceil().unwrap(),
                        _ => rounded,
                    };
                    Ok(value)
                }
            }
        )+
    };
}

macro_rules! fbinop {
    ($($ty: ty),+) => {
        $(
            impl Fbinop for $ty {
                fn add(&self, rhs: Self) -> Result<Self> {
                    Ok((*self) + rhs)
                }
                fn div(&self, rhs: Self) -> Result<Self> {
                    Ok((*self) / rhs)
                }
                fn mul(&self, rhs: Self) -> Result<Self> {
                    Ok((*self) * (rhs))
                }
                fn sub(&self, rhs: Self) -> Result<Self> {
                    Ok((*self) - rhs)
                }
                fn min(&self, rhs: Self) -> Result<Self> {
                    Ok((*self).min(rhs))
                }
                fn max(&self, rhs: Self) -> Result<Self> {
                    Ok((*self).max(rhs))
                }
                fn copysign(&self, rhs: Self) -> Result<Self> {
                    Ok((*self).copysign(rhs))
                }
            }
        )+
    };
}

macro_rules! frelop {
    ($($ty: ty),+) => {
        $(
            impl Frelop for $ty {
                fn equal(&self, rhs: Self) -> Result<i32> {
                    Ok(((*self) == rhs) as i32)
                }
                fn not_equal(&self, rhs: Self) -> Result<i32> {
                    Ok(((*self) != rhs) as i32)
                }
                fn flt(&self, rhs: Self) -> Result<i32> {
                    Ok(((*self) < rhs) as i32)
                }
                fn fgt(&self, rhs: Self) -> Result<i32> {
                    Ok(((*self) > rhs) as i32)
                }
                fn fle(&self, rhs: Self) -> Result<i32> {
                    Ok(((*self) <= rhs) as i32)
                }
                fn fge(&self, rhs: Self) -> Result<i32> {
                    Ok(((*self) >= rhs) as i32)
                }
            }
        )+
    };
}

funop!(f32, f64);
fbinop!(f32, f64);
frelop!(f32, f64);
