
#[macro_export]
macro_rules! load {
    ($stack: expr, $store: expr, $ty: ty, $arg: expr) => {{
        let store = $store
            .lock()
            .ok()
            .with_context(|| Error::CanNotLockForThread(Resource::Store))?;
        let memory = store
            .memory
            .get(0)
            .with_context(|| Error::NotFoundMemory(0))?;
        let memory = memory
            .lock()
            .ok()
            .with_context(|| Error::CanNotLockForThread(Resource::Memory))?;
        let addr = $stack.pop1::<i32>()? as usize;
        let value = memory.load::<$ty>(addr, $arg)?;
        $stack.push(value.into());
    }};
    ($stack: expr, $store: expr, $ty: ty, $arg: expr, $tz: ty) => {{
        let addr = $stack.pop1::<i32>()? as usize;
        let store = $store
            .lock()
            .ok()
            .with_context(|| Error::CanNotLockForThread(Resource::Store))?;
        let memory = store
            .memory
            .get(0)
            .with_context(|| Error::NotFoundMemory(0))?;
        let memory = memory
            .lock()
            .ok()
            .with_context(|| Error::CanNotLockForThread(Resource::Memory))?;
        let value = memory.load::<$ty>(addr, $arg)? as $tz;
        $stack.push(value.into());
    }};
}

#[macro_export]
macro_rules! store {
    ($stack: expr, $store: expr, $ty: ty, $arg: expr) => {{
        let store = $store
            .lock()
            .ok()
            .with_context(|| Error::CanNotLockForThread(Resource::Store))?;
        let memory = store
            .memory
            .get(0)
            .with_context(|| Error::NotFoundMemory(0))?;
        let mut memory = memory
            .lock()
            .ok()
            .with_context(|| Error::CanNotLockForThread(Resource::Memory))?;
        let value = $stack.pop1::<$ty>()?;
        let addr = $stack.pop1::<i32>()? as usize;
        memory.write(addr, $arg, value)?;
    }};
    ($stack: expr, $store: expr, $ty: ty, $arg: expr, $tz: ty) => {{
        let store = $store
            .lock()
            .ok()
            .with_context(|| Error::CanNotLockForThread(Resource::Store))?;
        let memory = store
            .memory
            .get(0)
            .with_context(|| Error::NotFoundMemory(0))?;
        let mut memory = memory
            .lock()
            .ok()
            .with_context(|| Error::CanNotLockForThread(Resource::Memory))?;
        let value = $stack.pop1::<$ty>()? as $tz;
        let addr = $stack.pop1::<i32>()? as usize;
        memory.write(addr, $arg, value)?;
    }};
}

#[macro_export]
macro_rules! impl_binary_operation {
    ($($op: ident),*) => {
        $(
            pub fn $op(stack: &mut impl StackAccess) -> Result<()> {
                let (r, l): (Value, Value) = stack.pop_rl()?;
                let value = l.$op(&r)?;
                stack.push(value);
                Ok(())
            }
        )*
    };
}

#[macro_export]
macro_rules! impl_unary_operation {
    ($($op: ident),*) => {
        $(
            pub fn $op(stack: &mut impl StackAccess) -> Result<()> {
                let value: Value = stack.pop1()?;
                let value = value.$op()?;
                stack.push(value);
                Ok(())
            }
         )*
    };
}

#[macro_export]
macro_rules! impl_cvtop_operation {
    ($($op: ident),*) => {
        $(
            pub fn $op(stack: &mut impl StackAccess) -> Result<()> {
                let value: Value = stack.pop1()?;
                let value = value.$op()?;
                stack.push(value);
                Ok(())
            }
         )*
    };
}

#[macro_export]
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

#[macro_export]
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

#[macro_export]
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

#[macro_export]
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

#[macro_export]
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

#[macro_export]
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

#[macro_export]
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
