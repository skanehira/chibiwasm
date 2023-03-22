use super::error::Error;
use super::{runtime::Runtime, value::Value};
use anyhow::{bail, Context as _, Result};

pub fn pop_rl(runtime: &mut Runtime) -> Result<(Value, Value)> {
    let r = runtime
        .value_stack
        .pop()
        .ok_or_else(|| Error::StackPopError)?;
    let l = runtime
        .value_stack
        .pop()
        .ok_or_else(|| Error::StackPopError)?;
    Ok((r, l))
}

pub fn local_get(runtime: &mut Runtime, idx: usize) -> Result<()> {
    let value = runtime
        .current_frame()?
        .local_stack
        .get(idx)
        .context("not found local variable")?;
    runtime.value_stack.push(value.clone());
    Ok(())
}

pub fn popcnt(runtime: &mut Runtime) -> Result<()> {
    let value = runtime.stack_pop()?;
    match value {
        Value::I32(v) => runtime.value_stack.push(v.count_ones().into()),
        Value::I64(v) => runtime.value_stack.push((v.count_ones() as i64).into()),
        _ => bail!("unexpected value"),
    }
    Ok(())
}

pub fn push<T: Into<Value>>(runtime: &mut Runtime, value: T) -> Result<()> {
    runtime.value_stack.push(value.into());
    Ok(())
}

pub fn i64extend_32s(runtime: &mut Runtime) -> Result<()> {
    let value = runtime
        .value_stack
        .pop()
        .ok_or_else(|| Error::StackPopError)?;
    match value {
        Value::I64(v) => {
            let result = v << 32 >> 32;
            runtime.value_stack.push(result.into());
        }
        _ => bail!("unexpected value type"),
    }
    Ok(())
}

macro_rules! impl_binary_operation {
    ($($op: ident),*) => {
        $(
            pub fn $op(runtime: &mut Runtime) -> Result<()> {
                let (r, l) = pop_rl(runtime)?;
                runtime.value_stack.push(l.$op(&r)?);
                Ok(())
            }
        )*
    };
}

macro_rules! impl_unary_operation {
    ($($op: ident),*) => {
        $(
            pub fn $op(runtime: &mut Runtime) -> Result<()> {
                let value = runtime.value_stack.pop().ok_or_else(|| Error::StackPopError)?;
                runtime.value_stack.push(value.$op()?);
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
