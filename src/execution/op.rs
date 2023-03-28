use super::value::StackAccess as _;
use super::value::StackValue;
use super::{runtime::Runtime, value::Value};
use anyhow::{bail, Context as _, Result};

pub fn local_get(runtime: &mut Runtime, idx: usize) -> Result<()> {
    let value = runtime
        .current_frame()?
        .locals
        .get(idx)
        .context("not found local variable")?;
    runtime.stack.push(StackValue::Value(value.clone()));
    Ok(())
}

pub fn local_set(runtime: &mut Runtime, idx: usize) -> Result<()> {
    let value: Value = runtime.stack.pop1()?;
    let frame = runtime.current_frame_mut()?;
    frame.locals.insert(idx, value.into());
    if frame.locals.len() > idx + 1 {
        frame.locals.remove(idx + 1);
    }
    Ok(())
}

pub fn popcnt(runtime: &mut Runtime) -> Result<()> {
    let value = runtime
        .stack
        .pop1()
        .context("not found value in the stack")?;

    match value {
        Value::I32(v) => {
            let value: Value = v.count_ones().into();
            runtime.stack.push(value.into());
        }
        Value::I64(v) => {
            let value: Value = (v.count_ones() as i64).into();
            runtime.stack.push(value.into());
        }
        _ => bail!("unexpected value"),
    }
    Ok(())
}

pub fn push<T: Into<StackValue>>(runtime: &mut Runtime, value: T) -> Result<()> {
    runtime.stack.push(value.into());
    Ok(())
}

pub fn i64extend_32s(runtime: &mut Runtime) -> Result<()> {
    let value = runtime.stack.pop1()?;
    match value {
        Value::I64(v) => {
            let result = v << 32 >> 32;
            let value: Value = result.into();
            runtime.stack.push(value.into());
        }
        _ => bail!("unexpected value type"),
    }
    Ok(())
}

macro_rules! impl_binary_operation {
    ($($op: ident),*) => {
        $(
            pub fn $op(runtime: &mut Runtime) -> Result<()> {
                let (r, l): (Value, Value) = runtime.stack.pop_rl()?;
                let value = l.$op(&r)?;
                runtime.stack.push(value.into());
                Ok(())
            }
        )*
    };
}

macro_rules! impl_unary_operation {
    ($($op: ident),*) => {
        $(
            pub fn $op(runtime: &mut Runtime) -> Result<()> {
                let value: Value = runtime.stack.pop1()?;
                let value = value.$op()?;
                runtime.stack.push(value.into());
                Ok(())
            }
         )*
    };
}

impl_unary_operation!(
    eqz, // itestop
    clz, ctz, extend8_s, extend16_s, // iunop
    abs, neg, sqrt, ceil, floor, trunc, nearest // funop
);

impl_binary_operation!(
    add, sub, mul, // binop
    div_s, div_u, rem_s, rem_u, and, or, xor, shl, shr_u, shr_s, rotl, rotr, // ibinop
    min, max, div, copysign, // fbinop
    equal, not_equal, // relop
    lt_s, lt_u, gt_s, gt_u, le_s, le_u, ge_s, ge_u, // irelop
    flt, fgt, fle, fge // frelop
);
