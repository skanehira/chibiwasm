#![allow(unused)]

use super::error::Error;
use super::module::{FuncInst, ModuleInst};
use super::op::*;
use super::store::Store;
use super::value::{ExternalVal, State, Value};
use super::value::{Frame, Label, StackAccess as _, StackValue};
use crate::binary::instruction::*;
use crate::binary::module::{Decoder, Module};
use crate::binary::types::{BlockType, FuncType};
use anyhow::{bail, Context as _, Result};
use std::fs;
use std::io::{Cursor, Read};
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct Runtime {
    pub store: Store,
    pub module: Rc<ModuleInst>,
    pub stack: Vec<StackValue>,
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

    pub fn instantiate(module: &mut Module) -> Result<Self> {
        let store = Store::new(module)?;
        let module = ModuleInst::new(&store, &module);

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
        let (idx, func) = self.resolve_by_name(name)?;
        if func.func_type.params.len() != args.len() {
            bail!("invalid argument length");
        }

        for arg in args {
            self.stack.push(arg.into());
        }

        match self.invoke(idx) {
            Ok(value) => Ok(value),
            Err(e) => {
                self.stack = vec![]; // when traped, need to cleanup stack
                Err(e)
            }
        }
    }

    // https://www.w3.org/TR/wasm-core-1/#exec-invoke
    fn invoke(&mut self, idx: usize) -> Result<Option<Value>> {
        // 1. get function instance from store
        let func = self.resolve_by_idx(idx)?;

        // 2. push the arguments to frame local
        let bottom = self.stack.len() - func.func_type.params.len();
        let locals = self
            .stack
            .split_off(bottom)
            .into_iter()
            .map(Into::into)
            .collect();

        // 3. push a frame
        let arity = func.func_type.results.len();
        self.push_frame(arity, locals);

        // 4. execute instruction of function
        // TODO: check state
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
        match inst {
            Instruction::Nop | Instruction::End => {}
            Instruction::LocalGet(idx) => local_get(runtime, *idx as usize)?,
            Instruction::LocalSet(idx) => local_set(runtime, *idx as usize)?,
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
            Instruction::MemoryGrow => {
                // TODO
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
                        // break current loop
                        State::Break(0) => {
                            // break to the current loop
                            // it's mean we need start loop again
                        }
                        state => {
                            let _ = runtime.pop_label()?;
                            match state {
                                State::Continue => {
                                    // 3. pop the label from the stack
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
                    State::Break(level) => return Ok(State::Break(level)),
                }
            }
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
            _ => {
                unimplemented!("{:?}", inst);
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
        let wat_code = br#"
(module
  (func $dummy)
  (func $i32.add (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add
  )
  (func $call (export "call") (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    call $i32.add
  )
  (func $return (export "return") (result i32)
    (return (i32.const 15))
  )
  (func $if (export "if") (param $a i32) (param $b i32) (result i32)
    (if
      (i32.eq (local.get $a) (local.get $b))
      (then (return (i32.const 1)))
    )
    (return (i32.const 0))
  )
  (func $fib (export "fib") (param $N i32) (result i32)
    (if
      (i32.eq (local.get $N) (i32.const 1))
      (then (return (i32.const 1)))
    )
    (if
      (i32.eq (local.get $N) (i32.const 2))
      (then (return (i32.const 1)))
    )
    (i32.add
      (call $fib (i32.sub (local.get $N) (i32.const 1)))
      (call $fib (i32.sub (local.get $N) (i32.const 2)))
    )
  )
  (func $if_else_empty (export "if_else_empty")
    (if (i32.const 1)
      (then)
      (else)
    )
  )
  (func (export "as-loop-first") (result i32)
    (loop (result i32) (block (result i32) (i32.const 1)) (call $dummy) (call $dummy))
  )
  (func (export "as-loop-mid") (result i32)
    (loop (result i32) (call $dummy) (block (result i32) (i32.const 1)) (call $dummy))
  )
  (func (export "as-loop-last") (result i32)
    (loop (result i32) (call $dummy) (call $dummy) (block (result i32) (i32.const 1)))
  )
  (func (export "singular") (result i32)
    (block (nop))
    (block (result i32) (i32.const 7))
  )
  (func (export "nested") (result i32)
    (block (result i32)
      (block (call $dummy) (block) (nop))
      (block (result i32) (call $dummy) (i32.const 9))
    )
  )
  (func (export "as-if-then") (result i32)
    (if (result i32) (i32.const 1) (then (block (result i32) (i32.const 1))) (else (i32.const 2)))
  )
  (func (export "as-if-else") (result i32)
    (if (result i32) (i32.const 0) (then (i32.const 2)) (else (block (result i32) (i32.const 1))))
  )
  (func (export "as-br-value") (result i32)
    (block (result i32) (br 0 (br 0 (i32.const 9))))
  )
  (func (export "as-br-last") (result i32)
    (block (result i32)
      (loop (result i32) (nop) (call $dummy) (br 1 (i32.const 5)))
    )
  )
  (func (export "as-if-cond") (result i32)
    (block (result i32)
      (if (result i32) (br 0 (i32.const 2))
        (then (i32.const 0))
        (else (i32.const 1))
      )
    )
  )
  (func (export "as-br_if-value") (result i32)
    (block (result i32)
      (drop (br_if 0 (br 0 (i32.const 8)) (i32.const 1))) (i32.const 7)
    )
  )
  (func (export "while") (param i32) (result i32)
    (local i32)
    (local.set 1 (i32.const 1))
    (block
      (loop
        (br_if 1 (i32.eqz (local.get 0)))
        (local.set 1 (i32.mul (local.get 0) (local.get 1)))
        (local.set 0 (i32.sub (local.get 0) (i32.const 1)))
        (br 0)
      )
    )
    (local.get 1)
  )
  (func (export "as-if-then-return") (param i32 i32) (result i32)
    (if (result i32)
      (local.get 0) 
        (then 
          (i32.const 1)
          (i32.const 2)
          (return (i32.const 3))
        )
        (else (local.get 1))
    )
  )

  (func (export "call-nested") (param i32 i32) (result i32)
    (if (result i32) (local.get 0)
      (then
        (if (local.get 1) (then (call $dummy) (block) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (block) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 9))
          (else (call $dummy) (i32.const 10))
        )
      )
      (else
        (if (local.get 1) (then (call $dummy) (block) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (block) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 10))
          (else (call $dummy) (i32.const 11))
        )
      )
    )
  )

  (func (export "br-nested") (result i32)
    (block
      (block
        (block
          (block 
            (i32.const 1)
            br 3
          )
        )
      )
    )
  )

  (func (export "if1") (result i32)
    (local $i i32)
    (local.set $i (i32.const 0))
    (block
      (if $l
        (i32.const 1)
        (then (br $l) (local.set $i (i32.const 666)))
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if $l
        (i32.const 1)
        (then (br $l) (local.set $i (i32.const 666)))
        (else (local.set $i (i32.const 888)))
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if $l
        (i32.const 1)
        (then (br $l) (local.set $i (i32.const 666)))
        (else (local.set $i (i32.const 888)))
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if $l
        (i32.const 0)
        (then (local.set $i (i32.const 888)))
        (else (br $l) (local.set $i (i32.const 666)))
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if $l
        (i32.const 0)
        (then (local.set $i (i32.const 888)))
        (else (br $l) (local.set $i (i32.const 666)))
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
    )
    (local.get $i)
  )
  (func (export "singleton") (param i32) (result i32)
    (block
      (block
        (br_table 1 0 (local.get 0))
        (return (i32.const 21))
      )
      (return (i32.const 20))
    )
    (i32.const 22)
  )

)
"#;
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

        Ok(())
    }
}
