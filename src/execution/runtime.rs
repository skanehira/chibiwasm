use super::module::{FuncInst, InternalFuncInst};
use super::op::*;
use super::store::{Exports, Store};
use super::value::{ExternalVal, Frame, Label, StackAccess, Value};
use crate::binary::instruction::*;
use crate::binary::types::ValueType;
use crate::execution::error::Error;
use crate::execution::value::LabelKind;
use crate::{load, store, Importer};
use anyhow::{bail, Context as _, Result};
use log::{error, trace};
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

#[derive(Default)]
pub struct Runtime {
    pub store: Rc<RefCell<Store>>,
    pub stack: Vec<Value>,
    pub call_stack: Vec<Frame>,
}

impl Runtime {
    pub fn from_file(file: &str, imports: Option<Box<dyn Importer>>) -> Result<Self> {
        let store = Store::from_file(file, imports)?;
        Self::instantiate(Rc::new(RefCell::new(store)))
    }

    pub fn from_reader(reader: &mut impl Read, imports: Option<Box<dyn Importer>>) -> Result<Self> {
        let store = Store::from_reader(reader, imports)?;
        Self::instantiate(Rc::new(RefCell::new(store)))
    }

    pub fn from_bytes<T: AsRef<[u8]>>(b: T, imports: Option<Box<dyn Importer>>) -> Result<Self> {
        let store = Store::from_bytes(b, imports)?;
        Self::instantiate(Rc::new(RefCell::new(store)))
    }

    // https://www.w3.org/TR/wasm-core-1/#instantiation%E2%91%A1
    pub fn instantiate(store: Rc<RefCell<Store>>) -> Result<Self> {
        let start = store.borrow().start;
        let mut runtime = Self {
            store,
            ..Default::default()
        };

        // https://www.w3.org/TR/wasm-core-1/#start-function%E2%91%A1
        if let Some(idx) = start {
            let result = runtime.call_start(idx as usize, vec![])?;
            if let Some(out) = result {
                runtime.stack.push(out);
            }
        }

        Ok(runtime)
    }

    // execute function by name
    pub fn call(&mut self, name: String, args: Vec<Value>) -> Result<Option<Value>> {
        trace!("call function: {}", name);
        for arg in args {
            self.stack.push(arg);
        }

        let idx = {
            let store = self.store.borrow();
            let export_inst = store
                .module
                .exports
                .get(&name)
                .with_context(|| Error::NotFoundExportInstance(name))?;
            let external_val = &export_inst.desc;

            let ExternalVal::Func(idx) = external_val else {
                bail!("invalid export desc: {:?}", external_val);
            };

            *idx as usize
        };
        self.invoke(idx)
    }

    // execute function when module has start section
    pub fn call_start(&mut self, idx: usize, args: Vec<Value>) -> Result<Option<Value>> {
        for arg in args {
            self.stack.push(arg);
        }
        self.invoke(idx)
    }

    // get exported instances by name, like table, memory, global
    pub fn exports(&mut self, name: String) -> Result<Exports> {
        let store = self.store.borrow();
        let export_inst = store
            .module
            .exports
            .get(&name)
            .with_context(|| Error::NotFoundExportInstance(name))?;

        let exports = match export_inst.desc {
            ExternalVal::Table(idx) => {
                let table = store
                    .tables
                    .get(idx as usize)
                    .with_context(|| Error::NotFoundExportedTable(idx))?;
                Exports::Table(Rc::clone(table))
            }
            ExternalVal::Memory(idx) => {
                let memory = store
                    .memory
                    .get(idx as usize)
                    .with_context(|| Error::NotFoundExportedMemory(idx))?;
                Exports::Memory(Rc::clone(memory))
            }
            ExternalVal::Global(idx) => {
                let global = store
                    .globals
                    .get(idx as usize)
                    .with_context(|| Error::NotFoundExportedGlobal(idx))?;
                Exports::Global(Rc::clone(global))
            }
            ExternalVal::Func(idx) => {
                let func = store
                    .funcs
                    .get(idx as usize)
                    .with_context(|| Error::NotFoundExportedFunction(idx))?;
                Exports::Func(func.clone())
            }
        };

        Ok(exports)
    }

    fn invoke_internal(&mut self, func: InternalFuncInst) -> Result<Option<Value>> {
        let bottom = self.stack.len() - func.func_type.params.len();
        let mut locals: Vec<_> = self.stack.split_off(bottom).into_iter().collect();

        for local in func.code.locals.iter() {
            match local {
                ValueType::I32 => locals.push(Value::I32(0)),
                ValueType::I64 => locals.push(Value::I64(0)),
                ValueType::F32 => locals.push(Value::F32(0.0)),
                ValueType::F64 => locals.push(Value::F64(0.0)),
            }
        }

        let arity = func.func_type.results.len();

        let frame = Frame {
            pc: -1,
            sp: self.stack.len(),
            insts: func.code.body,
            arity,
            locals,
            labels: vec![],
        };
        self.call_stack.push(frame);

        trace!("call stack: {:?}", &self.call_stack.last());
        self.execute()?;

        let result = if arity > 0 {
            // NOTE: only returns one value now
            let value: Value = self.stack.pop1()?;
            Some(value)
        } else {
            None
        };

        Ok(result)
    }

    // https://www.w3.org/TR/wasm-core-1/#exec-invoke
    fn invoke(&mut self, idx: usize) -> Result<Option<Value>> {
        let func = self.get_func_by_idx(idx)?;
        let result = match func {
            FuncInst::Internal(func) => self.invoke_internal(func),
            FuncInst::External(func) => {
                let stack = &mut self.stack;
                invoke_external(Rc::clone(&self.store), stack, func)
            }
        };
        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                self.stack = vec![]; // when traped, need to cleanup stack
                self.call_stack = vec![];
                Err(e)
            }
        }
    }

    fn get_func_by_idx(&mut self, idx: usize) -> Result<FuncInst> {
        let store = self.store.borrow();
        let func = store
            .funcs
            .get(idx)
            .with_context(|| Error::NotFoundFunction(idx))?;
        Ok(func.clone())
    }

    fn execute(&mut self) -> Result<()> {
        let stack = &mut self.stack;

        loop {
            let Some(frame) = self.call_stack.last_mut() else {
                trace!("call stack is empty, return");
                break;
            };
            let insts = &frame.insts;
            frame.pc += 1;
            let Some(inst) = insts.get(frame.pc as usize) else {
                trace!("reach the end of function");
                break;
            };
            trace!("pc: {}, inst: {:?}", frame.pc, &inst);
            match inst {
                Instruction::Unreachable => bail!("unreachable"),
                Instruction::Nop => {}
                Instruction::LocalGet(idx) => {
                    local_get(&frame.locals, stack, *idx as usize)?;
                }
                Instruction::LocalSet(idx) => {
                    local_set(&mut frame.locals, stack, *idx as usize)?;
                }
                Instruction::LocalTee(idx) => {
                    local_tee(&mut frame.locals, stack, *idx as usize)?;
                }
                Instruction::GlobalGet(idx) => {
                    global_get(&mut self.store.borrow_mut(), stack, *idx as usize)?
                }
                Instruction::GlobalSet(idx) => {
                    global_set(&mut self.store.borrow_mut(), stack, *idx as usize)?
                }
                Instruction::I32Add | Instruction::I64Add => add(stack)?,
                Instruction::I32Sub | Instruction::I64Sub => sub(stack)?,
                Instruction::I32Mul | Instruction::I64Mul => mul(stack)?,
                Instruction::I32Clz | Instruction::I64Clz => clz(stack)?,
                Instruction::I32Ctz | Instruction::I64Ctz => ctz(stack)?,
                Instruction::I32DivU | Instruction::I64DivU => div_u(stack)?,
                Instruction::I32DivS | Instruction::I64DivS => div_s(stack)?,
                Instruction::I32Eq | Instruction::I64Eq => equal(stack)?,
                Instruction::I32Eqz | Instruction::I64Eqz => eqz(stack)?,
                Instruction::I32Ne | Instruction::I64Ne => not_equal(stack)?,
                Instruction::I32LtS | Instruction::I64LtS => lt_s(stack)?,
                Instruction::I32LtU | Instruction::I64LtU => lt_u(stack)?,
                Instruction::I32GtS | Instruction::I64GtS => gt_s(stack)?,
                Instruction::I32GtU | Instruction::I64GtU => gt_u(stack)?,
                Instruction::I32LeS | Instruction::I64LeS => le_s(stack)?,
                Instruction::I32LeU | Instruction::I64LeU => le_u(stack)?,
                Instruction::I32GeS | Instruction::I64GeS => ge_s(stack)?,
                Instruction::I32GeU | Instruction::I64GeU => ge_u(stack)?,
                Instruction::I32Popcnt | Instruction::I64Popcnt => popcnt(stack)?,
                Instruction::I32RemU | Instruction::I64RemU => rem_u(stack)?,
                Instruction::I32RemS | Instruction::I64RemS => rem_s(stack)?,
                Instruction::I32And | Instruction::I64And => and(stack)?,
                Instruction::I32Or | Instruction::I64Or => or(stack)?,
                Instruction::I32Xor | Instruction::I64Xor => xor(stack)?,
                Instruction::I32ShL | Instruction::I64ShL => shl(stack)?,
                Instruction::I32ShrU | Instruction::I64ShrU => shr_u(stack)?,
                Instruction::I32ShrS | Instruction::I64ShrS => shr_s(stack)?,
                Instruction::I32RtoL | Instruction::I64RtoL => rotl(stack)?,
                Instruction::I32RtoR | Instruction::I64RtoR => rotr(stack)?,
                Instruction::I32Extend8S | Instruction::I64Extend8S => extend8_s(stack)?,
                Instruction::I32Extend16S | Instruction::I64Extend16S => extend16_s(stack)?,
                Instruction::I32Const(v) => stack.push((*v).into()),
                Instruction::I64Extend32S => i64extend_32s(stack)?,
                Instruction::I64Const(v) => stack.push((*v).into()),
                Instruction::F32Const(v) => stack.push((*v).into()),
                Instruction::F64Const(v) => stack.push((*v).into()),
                Instruction::F32Add | Instruction::F64Add => add(stack)?,
                Instruction::F32Sub | Instruction::F64Sub => sub(stack)?,
                Instruction::F32Mul | Instruction::F64Mul => mul(stack)?,
                Instruction::F32Div | Instruction::F64Div => div(stack)?,
                Instruction::F32Ceil | Instruction::F64Ceil => ceil(stack)?,
                Instruction::F32Floor | Instruction::F64Floor => floor(stack)?,
                Instruction::F32Max | Instruction::F64Max => max(stack)?,
                Instruction::F32Min | Instruction::F64Min => min(stack)?,
                Instruction::F32Nearest | Instruction::F64Nearest => nearest(stack)?,
                Instruction::F32Sqrt | Instruction::F64Sqrt => sqrt(stack)?,
                Instruction::F32Trunc | Instruction::F64Trunc => trunc(stack)?,
                Instruction::F32Copysign | Instruction::F64Copysign => copysign(stack)?,
                Instruction::I32WrapI64 => i32_wrap_i64(stack)?,
                Instruction::F32Abs | Instruction::F64Abs => abs(stack)?,
                Instruction::F32Neg | Instruction::F64Neg => neg(stack)?,
                Instruction::F32Eq | Instruction::F64Eq => equal(stack)?,
                Instruction::F32Ne | Instruction::F64Ne => not_equal(stack)?,
                Instruction::F32Lt | Instruction::F64Lt => flt(stack)?,
                Instruction::F32Gt | Instruction::F64Gt => fgt(stack)?,
                Instruction::F32Le | Instruction::F64Le => fle(stack)?,
                Instruction::F32Ge | Instruction::F64Ge => fge(stack)?,
                Instruction::Drop => {
                    stack.pop();
                }
                Instruction::Return => {
                    let frame = self
                        .call_stack
                        .pop()
                        .with_context(|| Error::CallStackPopError("return".into()))?;
                    trace!("frame in the return instruction: {:?}", &frame);
                    let Frame { sp, arity, .. } = frame;
                    stack_unwind(stack, sp, arity)?;
                }
                Instruction::End => {
                    match frame.labels.pop() {
                        // if label is exists, this means the end
                        // instruction is in a block, if, loop, or else
                        Some(label) => {
                            trace!("end instruction, label: {:?}", &label);
                            let Label { pc, sp, arity, .. } = label;
                            frame.pc = pc as isize;
                            stack_unwind(stack, sp, arity)?;
                        }
                        // it label is not exists, this means the end of
                        // function
                        None => {
                            trace!("frame: {:?}, stack: {:#?}", &frame, &stack);
                            let frame = self
                                .call_stack
                                .pop()
                                .with_context(|| Error::CallStackPopError("end".into()))?;
                            trace!("end instruction, frame: {:?}", &frame);
                            let Frame { sp, arity, .. } = frame;
                            stack_unwind(stack, sp, arity)?;
                        }
                    }
                }
                Instruction::Br(level) => {
                    let labels = &mut frame.labels;
                    let pc = br(labels, stack, level)?;
                    frame.pc = pc;
                }
                Instruction::BrIf(level) => {
                    let value: Value = stack.pop1()?;
                    if value.is_true() {
                        let labels = &mut frame.labels;
                        let pc = br(labels, stack, level)?;
                        frame.pc = pc;
                    }
                }
                Instruction::BrTable(label_idxs, default_idx) => {
                    let value: i32 = stack.pop1::<Value>()?.into();
                    let idx = value as usize;

                    let level = if idx < label_idxs.len() {
                        label_idxs
                            .get(idx)
                            .with_context(|| Error::InvalidBrTableIndex(idx))?
                    } else {
                        default_idx
                    };

                    let labels = &mut frame.labels;
                    let pc = br(labels, stack, level)?;
                    frame.pc = pc;
                }
                Instruction::Loop(block) => {
                    let arity = block.block_type.result_count();
                    let start_pc = frame.pc;
                    let pc = get_end_address(insts, frame.pc)?;

                    let label = Label {
                        start: Some(start_pc),
                        kind: LabelKind::Loop,
                        pc,
                        sp: stack.len(),
                        arity,
                    };
                    trace!("push label '{:?}' in the loop", &label);
                    frame.labels.push(label);
                }
                Instruction::If(block) => {
                    let cond: Value = stack.pop1()?;

                    // cal pc when the if block is end
                    let next_pc = get_end_address(insts, frame.pc)?;

                    if !cond.is_true() {
                        // if the condition is false, skip the if block
                        frame.pc = get_else_or_end_address(insts, frame.pc)? as isize;
                    }

                    let label = Label {
                        start: None,
                        kind: LabelKind::If,
                        pc: next_pc,
                        sp: stack.len(),
                        arity: block.block_type.result_count(),
                    };
                    trace!("push label '{:?}' in the if block", &label);
                    frame.labels.push(label);
                }
                Instruction::Else => {
                    let label = frame
                        .labels
                        .pop()
                        .with_context(|| Error::LabelPopError("else".into()))?;
                    let Label { pc, sp, arity, .. } = label;
                    frame.pc = pc as isize;
                    stack_unwind(stack, sp, arity)?;
                }
                Instruction::Block(block) => {
                    let arity = block.block_type.result_count();
                    let pc = get_end_address(insts, frame.pc)?;

                    let label = Label {
                        start: None,
                        kind: LabelKind::Block,
                        pc,
                        sp: stack.len(),
                        arity,
                    };
                    trace!("push label '{:?}' in the block", &label);
                    frame.labels.push(label);
                }
                Instruction::Call(idx) => {
                    let idx = *idx as usize;
                    let store = self.store.borrow();
                    let func = store
                        .funcs
                        .get(idx)
                        .with_context(|| Error::NotFoundFunction(idx))?;
                    match func {
                        FuncInst::Internal(func) => {
                            push_frame(stack, &mut self.call_stack, func);
                        }
                        FuncInst::External(func) => {
                            let result =
                                invoke_external(Rc::clone(&self.store), stack, func.clone())?;
                            if let Some(value) = result {
                                stack.push(value);
                            }
                        }
                    }
                }
                Instruction::CallIndirect((signature_idx, table_idx)) => {
                    let elem_idx = stack.pop1::<i32>()? as usize;

                    let func = {
                        let idx = *table_idx as usize;
                        let tables = &self.store.borrow().tables;
                        let table = tables
                            .get(idx) // NOTE: table_idx is always 0 now
                            .with_context(|| Error::NotFoundTable(idx))?;

                        let table = Rc::clone(table);
                        let table = table.borrow();
                        let func = table
                            .funcs
                            .get(elem_idx)
                            .with_context(|| Error::UndefinedElement)?
                            .as_ref()
                            .with_context(|| Error::UninitializedElement(elem_idx))?;

                        (*func).clone()
                    };

                    // validate expect func signature and actual func signature
                    let idx = *signature_idx as usize;
                    let store = self.store.borrow();
                    let expect_func_type = store
                        .module
                        .func_types
                        .get(idx)
                        .with_context(|| Error::NotFoundFuncType(idx))?;

                    let func_type = match func {
                        FuncInst::Internal(ref func) => &func.func_type,
                        FuncInst::External(ref func) => &func.func_type,
                    };

                    if func_type.params != expect_func_type.params
                        || func_type.results != expect_func_type.results
                    {
                        trace!(
                            "expect func signature: {:?}, actual func signature: {:?}",
                            expect_func_type,
                            func_type
                        );
                        bail!(Error::TypeMismatchIndirectCall)
                    }

                    match func {
                        FuncInst::Internal(ref func) => {
                            push_frame(stack, &mut self.call_stack, func);
                        }
                        FuncInst::External(ref func) => {
                            let result =
                                invoke_external(Rc::clone(&self.store), stack, func.clone())?;
                            if let Some(value) = result {
                                stack.push(value);
                            }
                        }
                    };
                }
                // NOTE: only support 1 memory now
                Instruction::MemoryGrow(idx) => {
                    let idx = *idx as usize;
                    let store = self.store.borrow();
                    let memory = store
                        .memory
                        .get(idx)
                        .with_context(|| Error::NotFoundMemory(idx))?;
                    let memory = Rc::clone(memory);
                    let n = stack.pop1::<i32>()?;
                    let mut memory = memory.borrow_mut();
                    let size = memory.size();
                    match memory.grow(n as u32) {
                        Ok(_) => {
                            stack.push((size as i32).into());
                        }
                        Err(e) => {
                            error!("memory grow error: {}", e);
                            stack.push((-1).into());
                        }
                    }
                }
                Instruction::MemorySize => {
                    let idx = 0;
                    let store = self.store.borrow();
                    let memory = store
                        .memory
                        .get(idx)
                        .with_context(|| Error::NotFoundMemory(idx))?;
                    let memory = memory.borrow();
                    let size = memory.size() as i32;
                    stack.push(size.into());
                }
                Instruction::I32Load(arg) => load!(stack, self.store, i32, arg),
                Instruction::I64Load(arg) => load!(stack, self.store, i64, arg),
                Instruction::F32Load(arg) => load!(stack, self.store, f32, arg),
                Instruction::F64Load(arg) => load!(stack, self.store, f64, arg),
                Instruction::I32Load8S(arg) => load!(stack, self.store, i8, arg, i32),
                Instruction::I32Load8U(arg) => load!(stack, self.store, u8, arg, i32),
                Instruction::I32Load16S(arg) => load!(stack, self.store, i16, arg, i32),
                Instruction::I32Load16U(arg) => load!(stack, self.store, u16, arg, i32),
                Instruction::I64Load8S(arg) => load!(stack, self.store, i8, arg, i64),
                Instruction::I64Load8U(arg) => load!(stack, self.store, u8, arg, i64),
                Instruction::I64Load16S(arg) => load!(stack, self.store, i16, arg, i64),
                Instruction::I64Load16U(arg) => load!(stack, self.store, u16, arg, i64),
                Instruction::I64Load32S(arg) => load!(stack, self.store, i32, arg, i64),
                Instruction::I64Load32U(arg) => load!(stack, self.store, u32, arg, i64),
                Instruction::I32Store(arg) => store!(stack, self.store, i32, arg),
                Instruction::I64Store(arg) => store!(stack, self.store, i64, arg),
                Instruction::F32Store(arg) => store!(stack, self.store, f32, arg),
                Instruction::F64Store(arg) => store!(stack, self.store, f64, arg),
                Instruction::I32Store8(arg) => store!(stack, self.store, i32, arg, i8),
                Instruction::I32Store16(arg) => store!(stack, self.store, i32, arg, i16),
                Instruction::I64Store16(arg) => store!(stack, self.store, i64, arg, i16),
                Instruction::I64Store8(arg) => store!(stack, self.store, i64, arg, i8),
                Instruction::I64Store32(arg) => store!(stack, self.store, i64, arg, i32),
                Instruction::Select => {
                    let cond = stack.pop1::<i32>()?;
                    let val2 = stack.pop1::<Value>()?;
                    let val1 = stack.pop1::<Value>()?;
                    stack.push(if cond != 0 { val1 } else { val2 });
                }
                Instruction::I32TruncF32S => i32_trunc_f32_s(stack)?,
                Instruction::I32TruncF32U => i32_trunc_f32_u(stack)?,
                Instruction::I32TruncF64S => i32_trunc_f64_s(stack)?,
                Instruction::I32TruncF64U => i32_trunc_f64_u(stack)?,
                Instruction::I64ExtendI32S => i64_extend_i32_s(stack)?,
                Instruction::I64ExtendI32U => i64_extend_i32_u(stack)?,
                Instruction::I64TruncF32S => i64_trunc_f32_s(stack)?,
                Instruction::I64TruncF32U => i64_trunc_f32_u(stack)?,
                Instruction::I64TruncF64S => i64_trunc_f64_s(stack)?,
                Instruction::I64TruncF64U => i64_trunc_f64_u(stack)?,
                Instruction::F32ConvertI32S => f32_convert_i32_s(stack)?,
                Instruction::F32ConvertI32U => f32_convert_i32_u(stack)?,
                Instruction::F32ConvertI64S => f32_convert_i64_s(stack)?,
                Instruction::F32ConvertI64U => f32_convert_i64_u(stack)?,
                Instruction::F32DemoteF64 => f32_demote_f64(stack)?,
                Instruction::F64ConvertI32S => f64_convert_i32_s(stack)?,
                Instruction::F64ConvertI32U => f64_convert_i32_u(stack)?,
                Instruction::F64ConvertI64S => f64_convert_i64_s(stack)?,
                Instruction::F64ConvertI64U => f64_convert_i64_u(stack)?,
                Instruction::F64PromoteF32 => f64_demote_f32(stack)?,
                Instruction::I32ReinterpretF32 => i32_reinterpret_f32(stack)?,
                Instruction::I64ReinterpretF64 => i64_reinterpret_f64(stack)?,
                Instruction::F32ReinterpretI32 => f32_reinterpret_i32(stack)?,
                Instruction::F64ReinterpretI64 => f64_reinterpret_i64(stack)?,
            };
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{Runtime, Value};
    use anyhow::{Context, Result};

    #[test]
    fn invoke() -> Result<()> {
        pretty_env_logger::init();
        let wat_code = include_bytes!("./fixtures/invoke.wat");
        let wasm = wat::parse_bytes(wat_code)?;
        let mut runtime = Runtime::from_bytes(wasm, None)?;

        // expect some return value
        {
            let tests = [
                ("call", vec![10, 10], 20),
                ("return", vec![], 15),
                ("as-loop-first", vec![], 1),
                ("as-loop-mid", vec![], 1),
                ("as-loop-last", vec![], 1),
                ("singular", vec![0; 0], 7),
                ("nested", vec![], 9),
                ("as-if-then", vec![0; 0], 1),
                ("as-if-else", vec![], 1),
                ("if", vec![1, 0], 0),
                ("fib", vec![5], 5),
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

// TODO: test wasi
//    #[test]
//    fn test_wasi() -> Result<()> {
//        let wat_code = r#"
//(module
//  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
//  (memory 1)
//  (data (i32.const 0) "Hello, world!")
//  (func $main (export "main") (result i32)
//    (call $fd_write (i32.const 0) (i32.const 1) (i32.const 1) (i32.const 1))
//  )
//)
//        "#;
//        let wasm = &mut wat::parse_str(wat_code)?;
//        let wasi = crate::wasi::wasi_snapshot_preview1::Wasi {};
//        let mut runtime = Runtime::from_bytes(wasm, Some(Box::new(wasi)))?;
//        runtime.call("main".into(), vec![Value::I32(0), Value::I32(13)])?;
//        Ok(())
//    }
}
