#![allow(unused)]

use super::error::Error;
use super::module::{FuncInst, ModuleInst};
use super::op::*;
use super::store::Store;
use super::value::{ExternalVal, Frame, Label, StackAccess as _, State, Value};
use crate::binary::instruction::*;
use crate::binary::module::{Decoder, Module};
use crate::binary::types::{Block, BlockType, FuncType, ValueType};
use anyhow::{bail, Context as _, Result};
use log::{error, trace};
use std::fs;
use std::io::{Cursor, Read};
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct Runtime {
    pub store: Store,
    pub module: Rc<ModuleInst>,
    pub stack: Vec<Value>,
    pub label_stack: Vec<Label>,
    pub call_stack: Vec<Frame>,
}

impl Runtime {
    pub fn from_file(file: &str) -> Result<Self> {
        let file = fs::File::open(file)?;
        let mut decoder = Decoder::new(file);
        let mut module = decoder.decode()?;
        Ok(Self::instantiate(&mut module)?)
    }

    pub fn from_reader(reader: &mut impl Read) -> Result<Self> {
        let mut decoder = Decoder::new(reader);
        let mut module = decoder.decode()?;
        Ok(Self::instantiate(&mut module)?)
    }

    pub fn from_bytes<T: AsRef<[u8]>>(b: T) -> Result<Self> {
        let buf = Cursor::new(b);
        let mut decoder = Decoder::new(buf);
        let mut module = decoder.decode()?;
        Ok(Self::instantiate(&mut module)?)
    }

    // https://www.w3.org/TR/wasm-core-1/#instantiation%E2%91%A1
    pub fn instantiate(module: &mut Module) -> Result<Self> {
        let store = Store::new(module)?;
        let module = ModuleInst::allocate(&module);

        let runtime = Self {
            store,
            module: Rc::new(module),
            ..Default::default()
        };

        Ok(runtime)
    }

    pub fn current_frame(&self) -> Result<&Frame> {
        let frame = self
            .call_stack
            .last()
            .with_context(|| format!("call stack is empty",))?;
        Ok(frame)
    }

    pub fn current_frame_mut(&mut self) -> Result<&mut Frame> {
        let frame = self
            .call_stack
            .last_mut()
            .with_context(|| format!("call stack is emtpy"))?;
        Ok(frame)
    }

    fn push_label(&mut self, arity: usize) -> Result<()> {
        let label = Label { arity };
        let frame = self.current_frame_mut()?;
        frame.labels.push(label);
        Ok(())
    }

    fn pop_label(&mut self) -> Result<Label> {
        let frame = self.current_frame_mut()?;
        let label = frame
            .labels
            .pop()
            .with_context(|| format!("no label in the frame. frame: {:?}", frame))?;
        Ok(label)
    }

    fn push_frame(&mut self, arity: usize, locals: Vec<Value>) {
        let frame = Frame {
            arity,
            locals,
            labels: vec![],
        };
        self.call_stack.push(frame);
    }

    fn pop_frame(&mut self) -> Result<Frame> {
        self.call_stack
            .pop()
            .with_context(|| format!("no frame in the call stack, call stack: {:?}", self.stack))
    }

    pub fn call(&mut self, name: String, args: Vec<Value>) -> Result<Option<Value>> {
        trace!("call function: {}", name);
        let (idx, func) = self.resolve_by_name(name)?;
        if func.func_type.params.len() != args.len() {
            bail!("invalid argument length");
        }

        for arg in args {
            self.stack.push(arg.into());
        }

        let result = match self.invoke(idx) {
            Ok(value) => Ok(value),
            Err(e) => {
                self.stack = vec![]; // when traped, need to cleanup stack
                Err(e)
            }
        };
        trace!("stack when after call function: {:#?}", &self.stack);
        result
    }

    // https://www.w3.org/TR/wasm-core-1/#exec-invoke
    fn invoke(&mut self, idx: usize) -> Result<Option<Value>> {
        // 1. get function instance from store
        let func = self.resolve_by_idx(idx)?;

        // 2. push the arguments to frame local
        let bottom = self.stack.len() - func.func_type.params.len();
        let mut locals: Vec<_> = self
            .stack
            .split_off(bottom)
            .into_iter()
            .map(Into::into)
            .collect();

        for local in func.code.locals.iter() {
            match local {
                ValueType::I32 => locals.push(Value::I32(0)),
                ValueType::I64 => locals.push(Value::I64(0)),
                ValueType::F32 => locals.push(Value::F32(0.0)),
                ValueType::F64 => locals.push(Value::F64(0.0)),
            }
        }

        // 3. push a frame
        let arity = func.func_type.results.len();
        self.push_frame(arity, locals);

        // 4. execute instruction of function
        // TODO: check state
        trace!("call stack: {:?}", &self.call_stack.last());
        let _ = execute(self, &func.code.body)?;

        // 5. if the function has return value, pop it from stack
        let result = if arity > 0 {
            // NOTE: only returns one value now
            let value: Value = self.stack.pop1()?;
            Some(value)
        } else {
            None
        };

        // 6. pop current frame
        let _ = self.pop_frame()?;

        Ok(result)
    }

    fn resolve_by_idx(&mut self, idx: usize) -> Result<FuncInst> {
        let function = self
            .store
            .funcs
            .get(idx)
            .context(format!("not found function {idx}"))?;
        Ok((*function).clone())
    }

    fn resolve_by_name(&mut self, name: String) -> Result<(usize, FuncInst)> {
        let export_inst = self
            .module
            .exports
            .get(&name)
            .context(format!("not found function {name}"))?;
        let external_val = &export_inst.desc;

        let idx = match external_val {
            ExternalVal::Func(i) => i,
            _ => bail!("invalid export desc: {:?}", external_val),
        };
        let function = self
            .store
            .funcs
            .get(*idx as usize)
            .context("not found function {name}")?;
        Ok(((*idx) as usize, (*function).clone()))
    }
}

fn execute(runtime: &mut Runtime, insts: &Vec<Instruction>) -> Result<State> {
    for inst in insts {
        if !matches!(
            inst,
            Instruction::Block(_) | Instruction::If(_) | Instruction::Loop(_)
        ) {
            trace!("instruction: {:?}", &inst);
        }
        match inst {
            Instruction::Unreachable => bail!("unreachable"),
            Instruction::Nop | Instruction::End => {}
            Instruction::LocalGet(idx) => local_get(runtime, *idx as usize)?,
            Instruction::LocalSet(idx) => local_set(runtime, *idx as usize)?,
            Instruction::LocalTee(idx) => local_tee(runtime, *idx as usize)?,
            Instruction::GlobalGet(idx) => global_get(runtime, *idx as usize)?,
            Instruction::GlobalSet(idx) => global_set(runtime, *idx as usize)?,
            Instruction::I32Add | Instruction::I64Add => add(runtime)?,
            Instruction::I32Sub | Instruction::I64Sub => sub(runtime)?,
            Instruction::I32Mul | Instruction::I64Mul => mul(runtime)?,
            Instruction::I32Clz | Instruction::I64Clz => clz(runtime)?,
            Instruction::I32Ctz | Instruction::I64Ctz => ctz(runtime)?,
            Instruction::I32DivU | Instruction::I64DivU => div_u(runtime)?,
            Instruction::I32DivS | Instruction::I64DivS => div_s(runtime)?,
            Instruction::I32Eq | Instruction::I64Eq => equal(runtime)?,
            Instruction::I32Eqz | Instruction::I64Eqz => eqz(runtime)?,
            Instruction::I32Ne | Instruction::I64Ne => not_equal(runtime)?,
            Instruction::I32LtS | Instruction::I64LtS => lt_s(runtime)?,
            Instruction::I32LtU | Instruction::I64LtU => lt_u(runtime)?,
            Instruction::I32GtS | Instruction::I64GtS => gt_s(runtime)?,
            Instruction::I32GtU | Instruction::I64GtU => gt_u(runtime)?,
            Instruction::I32LeS | Instruction::I64LeS => le_s(runtime)?,
            Instruction::I32LeU | Instruction::I64LeU => le_u(runtime)?,
            Instruction::I32GeS | Instruction::I64GeS => ge_s(runtime)?,
            Instruction::I32GeU | Instruction::I64GeU => ge_u(runtime)?,
            Instruction::I32Popcnt | Instruction::I64Popcnt => popcnt(runtime)?,
            Instruction::I32RemU | Instruction::I64RemU => rem_u(runtime)?,
            Instruction::I32RemS | Instruction::I64RemS => rem_s(runtime)?,
            Instruction::I32And | Instruction::I64And => and(runtime)?,
            Instruction::I32Or | Instruction::I64Or => or(runtime)?,
            Instruction::I32Xor | Instruction::I64Xor => xor(runtime)?,
            Instruction::I32ShL | Instruction::I64ShL => shl(runtime)?,
            Instruction::I32ShrU | Instruction::I64ShrU => shr_u(runtime)?,
            Instruction::I32ShrS | Instruction::I64ShrS => shr_s(runtime)?,
            Instruction::I32RtoL | Instruction::I64RtoL => rotl(runtime)?,
            Instruction::I32RtoR | Instruction::I64RtoR => rotr(runtime)?,
            Instruction::I32Extend8S | Instruction::I64Extend8S => extend8_s(runtime)?,
            Instruction::I32Extend16S | Instruction::I64Extend16S => extend16_s(runtime)?,
            Instruction::I32Const(v) => push(runtime, *v)?,
            Instruction::I64Extend32S => i64extend_32s(runtime)?,
            Instruction::I64Const(v) => push(runtime, *v)?,
            Instruction::F32Const(v) => push(runtime, *v)?,
            Instruction::F64Const(v) => push(runtime, *v)?,
            Instruction::F32Add | Instruction::F64Add => add(runtime)?,
            Instruction::F32Sub | Instruction::F64Sub => sub(runtime)?,
            Instruction::F32Mul | Instruction::F64Mul => mul(runtime)?,
            Instruction::F32Div | Instruction::F64Div => div(runtime)?,
            Instruction::F32Ceil | Instruction::F64Ceil => ceil(runtime)?,
            Instruction::F32Floor | Instruction::F64Floor => floor(runtime)?,
            Instruction::F32Max | Instruction::F64Max => max(runtime)?,
            Instruction::F32Min | Instruction::F64Min => min(runtime)?,
            Instruction::F32Nearest | Instruction::F64Nearest => nearest(runtime)?,
            Instruction::F32Sqrt | Instruction::F64Sqrt => sqrt(runtime)?,
            Instruction::F32Trunc | Instruction::F64Trunc => trunc(runtime)?,
            Instruction::F32Copysign | Instruction::F64Copysign => copysign(runtime)?,
            Instruction::I32WrapI64 => wrap_i64(runtime)?,
            Instruction::F32Abs | Instruction::F64Abs => abs(runtime)?,
            Instruction::F32Neg | Instruction::F64Neg => neg(runtime)?,
            Instruction::F32Eq | Instruction::F64Eq => equal(runtime)?,
            Instruction::F32Ne | Instruction::F64Ne => not_equal(runtime)?,
            Instruction::F32Lt | Instruction::F64Lt => flt(runtime)?,
            Instruction::F32Gt | Instruction::F64Gt => fgt(runtime)?,
            Instruction::F32Le | Instruction::F64Le => fle(runtime)?,
            Instruction::F32Ge | Instruction::F64Ge => fge(runtime)?,
            Instruction::Drop => {
                runtime.stack.pop();
            }
            Instruction::Return => {
                return Ok(State::Return);
            }
            Instruction::Br(level) => {
                return Ok(State::Break(*level as usize));
            }
            Instruction::BrIf(level) => {
                let value: Value = runtime.stack.pop1()?;
                if value.is_true() {
                    return Ok(State::Break(*level as usize));
                }
            }
            Instruction::BrTable(label_idxs, default_idx) => {
                let value: Value = runtime.stack.pop1()?;
                let idx: i32 = value.into();
                let state = if idx < label_idxs.len() as i32 {
                    let idx = label_idxs[idx as usize];
                    State::Break(idx as usize)
                } else {
                    State::Break((*default_idx) as usize)
                };
                return Ok(state);
            }
            Instruction::Loop(block) => {
                // 1. push a label to the stack
                let arity = block.block_type.result_count();
                runtime.push_label(arity);

                // 2. execute the loop body
                loop {
                    match execute(runtime, &block.then_body)? {
                        State::Break(0) => {
                            // it's mean we need start loop again
                        }
                        state => {
                            // 3. pop the label from the stack
                            let _ = runtime.pop_label()?;
                            match state {
                                State::Continue => {
                                    // break current loop
                                    break;
                                }
                                State::Return => {
                                    // break current loop and return
                                    return Ok(State::Return);
                                }
                                State::Break(level) => {
                                    // break outer block
                                    return Ok(State::Break(level - 1));
                                }
                                _ => {
                                    unreachable!()
                                }
                            }
                        }
                    }
                }
            }
            Instruction::If(block) => {
                // 1. pop the value from the stack for check if true
                let value: Value = runtime.stack.pop1()?;

                // 2. push a label to the stack
                let arity = block.block_type.result_count();
                runtime.push_label(arity);

                // 3. if true, execute the then_body, otherwise execute the else_body
                let result = if value.is_true() {
                    execute(runtime, &block.then_body)?
                } else {
                    execute(runtime, &block.else_body)?
                };

                // 4. pop the label from the Stack
                let _ = runtime.pop_label()?;

                match result {
                    State::Continue => {}
                    State::Return => return Ok(State::Return),
                    State::Break(0) => {}
                    State::Break(level) => return Ok(State::Break(level - 1)),
                }
            }
            // NOTE: this instruction will not be executed
            Instruction::Else => unreachable!(),
            Instruction::Block(block) => {
                // 1. push a label to the stack
                let arity = block.block_type.result_count();
                runtime.push_label(arity);

                // 2. execute the block body
                let result = execute(runtime, &block.then_body)?;

                // 3. pop the label from the stack
                let _ = runtime.pop_label()?;

                match result {
                    State::Continue => {}
                    State::Return => return Ok(State::Return),
                    State::Break(0) => {}
                    State::Break(level) => return Ok(State::Break(level - 1)),
                }
            }
            Instruction::Call(idx) => {
                let result = runtime.invoke(*idx as usize)?;
                match result {
                    Some(value) => {
                        runtime.stack.push(value.into());
                    }
                    _ => {}
                }
            }
            Instruction::CallIndirect((signature_idx, table_idx)) => {
                let table = runtime
                    .store
                    .tables
                    .get(*table_idx as usize) // NOTE: table_idx is always 0 now
                    .with_context(|| {
                        format!(
                            "not found table with index {}, tables: {:?}",
                            table_idx, &runtime.store.tables
                        )
                    })?;
                let elem_idx = runtime.stack.pop1::<i32>()? as usize;
                let func_idx = table.elem.get(elem_idx as usize).with_context(|| {
                    trace!(
                        "not found function with index {}, stack: {:?}",
                        elem_idx,
                        &runtime.stack
                    );
                    format!("undefined element")
                })?;
                trace!(
                    "func_idx: {}, func instance: {:#?}",
                    func_idx,
                    &runtime.store.funcs
                );

                // validation
                let func = runtime
                    .store
                    .funcs
                    .get(*func_idx as usize)
                    .with_context(|| {
                        format!(
                            "not found function from store.funcs with index {}, funcs: {:?}",
                            func_idx, &runtime.store.funcs
                        )
                    })?;
                // validate expect func signature and actual func signature
                let expect_func_type = runtime
                    .module
                    .func_types
                    .get(*signature_idx as usize)
                    .with_context(|| {
                        format!(
                            "not found type from module.func_types with index {}, types: {:?}",
                            func_idx, &runtime.module.func_types
                        )
                    })?
                    .clone();

                if func.func_type.params != expect_func_type.params
                    || func.func_type.results != expect_func_type.results
                {
                    trace!(
                        "expect func signature: {:?}, actual func signature: {:?}",
                        expect_func_type,
                        func.func_type
                    );
                    bail!("indirect call type mismatch")
                }

                match runtime.invoke(*func_idx as usize)? {
                    Some(value) => {
                        runtime.stack.push(value.into());
                    }
                    _ => {}
                }
            }
            // NOTE: only support 1 memory now
            Instruction::MemoryGrow(_) => {
                let memory = &mut runtime.store.memory;
                let size = memory.size();
                let n = runtime.stack.pop1::<i32>()?;
                match memory.grow(n as u32) {
                    Ok(_) => {
                        runtime.stack.push((size as i32).into());
                    }
                    Err(e) => {
                        error!("memory grow error: {}", e);
                        runtime.stack.push((-1).into());
                    }
                }
            }
            Instruction::MemorySize => {
                let size = runtime.store.memory.size() as i32;
                runtime.stack.push(size.into());
            }
            Instruction::I32Load(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<i32>(addr, arg)?.into();
                runtime.stack.push(value);
            }
            Instruction::I64Load(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<i64>(addr, arg)?.into();
                runtime.stack.push(value);
            }
            Instruction::F32Load(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<f32>(addr, arg)?.into();
                runtime.stack.push(value);
            }
            Instruction::F64Load(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<f64>(addr, arg)?.into();
                runtime.stack.push(value);
            }
            Instruction::I32Load8S(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<i8>(addr, arg)? as i32;
                runtime.stack.push(value.into());
            }
            Instruction::I32Load8U(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<u8>(addr, arg)? as i32;
                runtime.stack.push(value.into());
            }
            Instruction::I32Load16S(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<i16>(addr, arg)? as i32;
                runtime.stack.push(value.into());
            }
            Instruction::I32Load16U(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<u16>(addr, arg)? as i32;
                runtime.stack.push(value.into());
            }
            Instruction::I64Load8S(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<i8>(addr, arg)? as i64;
                runtime.stack.push(value.into());
            }
            Instruction::I64Load8U(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<u8>(addr, arg)? as i64;
                runtime.stack.push(value.into());
            }
            Instruction::I64Load16S(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<i16>(addr, arg)? as i64;
                runtime.stack.push(value.into());
            }
            Instruction::I64Load16U(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<u16>(addr, arg)? as i64;
                runtime.stack.push(value.into());
            }
            Instruction::I64Load32S(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<i32>(addr, arg)? as i64;
                runtime.stack.push(value.into());
            }
            Instruction::I64Load32U(arg) => {
                let addr = runtime.stack.pop1::<i32>()? as usize;
                let value = runtime.store.memory.load::<u32>(addr, arg)? as i64;
                runtime.stack.push(value.into());
            }
            Instruction::I32Store(arg) => {
                let value = runtime.stack.pop1::<i32>()?;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::I64Store(arg) => {
                let value = runtime.stack.pop1::<i64>()?;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::F32Store(arg) => {
                let value = runtime.stack.pop1::<f32>()?;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::F64Store(arg) => {
                let value = runtime.stack.pop1::<f64>()?;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::I32Store8(arg) => {
                let value = runtime.stack.pop1::<i32>()? as i8;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::I32Store16(arg) => {
                let value = runtime.stack.pop1::<i32>()? as i16;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::I64Store16(arg) => {
                let value = runtime.stack.pop1::<i64>()? as i16;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::I64Store8(arg) => {
                let value = runtime.stack.pop1::<i64>()? as i8;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::I64Store16(arg) => {
                let value = runtime.stack.pop1::<i64>()? as i16;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::I64Store32(arg) => {
                let value = runtime.stack.pop1::<i64>()? as i32;
                let addr = runtime.stack.pop1::<i32>()? as usize;
                runtime.store.memory.write(addr, arg, value)?;
            }
            Instruction::Select => {
                let cond = runtime.stack.pop1::<i32>()?;
                let val2 = runtime.stack.pop1::<Value>()?;
                let val1 = runtime.stack.pop1::<Value>()?;
                runtime.stack.push(if cond != 0 { val1 } else { val2 });
            }
            Instruction::I32TruncF32S => i32_trunc_f32_s(runtime)?,
            Instruction::I32TruncF32U => i32_trunc_f32_u(runtime)?,
            Instruction::I32TruncF64S => i32_trunc_f64_s(runtime)?,
            Instruction::I32TruncF64U => i32_trunc_f64_u(runtime)?,
            Instruction::I64ExtendI32S => i64_extend_i32_s(runtime)?,
            Instruction::I64ExtendI32U => i64_extend_i32_u(runtime)?,
            Instruction::I64TruncF32S => i64_trunc_f32_s(runtime)?,
            Instruction::I64TruncF32U => i64_trunc_f32_u(runtime)?,
            Instruction::I64TruncF64S => i64_trunc_f64_s(runtime)?,
            Instruction::I64TruncF64U => i64_trunc_f64_u(runtime)?,
            Instruction::F32ConvertI32S => f32_convert_i32_s(runtime)?,
            Instruction::F32ConvertI32U => f32_convert_i32_u(runtime)?,
            Instruction::F32ConvertI64S => f32_convert_i64_s(runtime)?,
            Instruction::F32ConvertI64U => f32_convert_i64_u(runtime)?,
            Instruction::F32DemoteF64 => f32_demote_f64(runtime)?,
            Instruction::F64ConvertI32S => f64_convert_i32_s(runtime)?,
            Instruction::F64ConvertI32U => f64_convert_i32_u(runtime)?,
            Instruction::F64ConvertI64S => f64_convert_i64_s(runtime)?,
            Instruction::F64ConvertI64U => f64_convert_i64_u(runtime)?,
            Instruction::F64PromoteF32 => f64_demote_f32(runtime)?,
            Instruction::I32ReinterpretF32 => i32_reinterpret_f32(runtime)?,
            Instruction::I64ReinterpretF64 => i64_reinterpret_f64(runtime)?,
            Instruction::F32ReinterpretI32 => f32_reinterpret_i32(runtime)?,
            Instruction::F64ReinterpretI64 => f64_reinterpret_i64(runtime)?,
            _ => {
                unimplemented!("instruction: {:?}", inst);
            }
        };
    }
    Ok(State::Continue)
}

#[cfg(test)]
mod test {
    use super::{Runtime, Value};
    use anyhow::{Context, Result};
    use wasmer::wat2wasm;

    #[test]
    fn invoke() -> Result<()> {
        let wat_code = include_bytes!("./fixtures/invoke.wat");
        let wasm = &mut wat2wasm(wat_code)?;
        let mut runtime = Runtime::from_bytes(wasm)?;

        // expect some return value
        {
            let tests = [
                ("call", vec![10, 10], 20),
                ("return", vec![], 15),
                ("as-loop-first", vec![], 1),
                ("as-loop-mid", vec![], 1),
                ("as-loop-last", vec![], 1),
                ("singular", vec![], 7),
                ("nested", vec![], 9),
                ("as-if-then", vec![], 1),
                ("as-if-else", vec![], 1),
                ("if", vec![1, 0], 0),
                ("fib", vec![10], 55),
                ("as-br-value", vec![], 9),
                ("as-br-last", vec![], 5),
                ("as-if-cond", vec![], 2),
                ("as-br_if-value", vec![], 8),
                ("while", vec![5], 120),
                ("as-if-then-return", vec![1, 2], 3),
                ("call-nested", vec![1, 0], 10),
                ("if1", vec![], 5),
                ("br-nested", vec![], 1),
                ("singleton", vec![0], 22),
                ("memsize", vec![], 1),
            ];

            for test in tests.into_iter() {
                let args = test.1.into_iter().map(Value::from).collect();
                let result = runtime.call(test.0.into(), args)?;
                print!("testing ... {} ", test.0);
                assert_eq!(
                    result.context("no return value")?,
                    test.2.into(),
                    "func {} fail",
                    test.0
                );
                println!("ok");
            }
        }

        // none return value
        {
            let result = runtime.call("if_else_empty".into(), vec![])?;
            assert_eq!(result, None);
        }

        // test memory load
        {
            macro_rules! test_memory_load {
                ($(($ty: ty, $expected: expr)),*) => {
                    $({
                        let name = stringify!($ty).to_string() + ".load";
                        let result = runtime.call(name.clone(), vec![])?;
                        print!("testing ... {} ", name);
                        assert_eq!(
                            result.context("no return value")?,
                            $expected.into(),
                            "func {} fail",
                            name,
                        );
                        println!("ok");
                    })*
                };
            }

            test_memory_load!(
                (i32, 1701077858),
                (i64, 0x6867666564636261_i64),
                (f32, 1.6777999e22_f32),
                (f64, 8.540883223036124e194_f64)
            );
        }

        Ok(())
    }
}
