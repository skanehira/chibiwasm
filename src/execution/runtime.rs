#![allow(unused)]

use super::error::Error;
use super::instance::{FuncInst, ModuleInst};
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
    pub pc: usize,
    pub stack: Vec<StackValue>,
    pub frame_idxs: Vec<usize>, // frame index, the last one is the current frame
}

impl Runtime {
    pub fn from_file(file: &str) -> Result<Self> {
        let file = fs::File::open(file)?;
        let mut decoder = Decoder::new(file);
        let mut module = decoder.decode()?;
        Ok(Self::new(&mut module)?)
    }

    pub fn from_reader(reader: &mut impl Read) -> Result<Self> {
        let mut decoder = Decoder::new(reader);
        let mut module = decoder.decode()?;
        Ok(Self::new(&mut module)?)
    }

    pub fn from_bytes<T: AsRef<[u8]>>(b: T) -> Result<Self> {
        let buf = Cursor::new(b);
        let mut decoder = Decoder::new(buf);
        let mut module = decoder.decode()?;
        Ok(Self::new(&mut module)?)
    }

    pub fn new(module: &mut Module) -> Result<Self> {
        let store = Store::new(module)?;
        let module = ModuleInst::instantiate(&store, &module);

        let runtime = Self {
            store,
            module: Rc::new(module),
            ..Default::default()
        };

        Ok(runtime)
    }

    pub fn current_frame(&self) -> &Frame {
        let idx = self.frame_idxs.last().unwrap();
        let value = self.stack.get(*idx);
        match value {
            Some(StackValue::Frame(frame)) => frame,
            _ => panic!("not found current frame"),
        }
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
        let func = self.resolve_by_idx(idx)?;

        // push the arguments to frame local
        let bottom = self.stack.len() - func.func_type.params.len();
        let locals = self
            .stack
            .split_off(bottom)
            .into_iter()
            .map(Into::into)
            .collect();

        let arity = func.func_type.results.len();

        push_frame(self, arity, locals)?;

        execute(self, &func.code.body)?;

        let result = if arity > 0 {
            // NOTE: only returns one value now
            let value: Value = self.stack.pop1()?;
            Some(value)
        } else {
            None
        };

        // pop current frame
        let idx = self
            .frame_idxs
            .pop()
            .with_context(|| format!("no any frame in the stack, stack: {:?}", self.stack))?;
        self.stack.split_off(idx);

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

fn pop_label(runtime: &mut Runtime) -> Result<()> {
    let mut tmp = vec![];
    loop {
        let value = runtime.stack.pop1()?;
        match value {
            StackValue::Value(value) => {
                tmp.push(value);
            }
            StackValue::Label(label) => {
                if label.arity > 0 {
                    let values = &mut tmp[..label.arity];
                    values.reverse();
                    for v in values.iter() {
                        runtime.stack.push(v.to_owned().into());
                    }
                }
                break;
            }
            StackValue::Frame(frame) => {
                panic!(
                    "expect value or label when pop label from stack, but got frame: {:?}",
                    frame
                );
            }
        }
    }
    Ok(())
}

fn pop_frame(runtime: &mut Runtime) -> Result<()> {
    let arity = runtime.current_frame().arity;

    // pop frame from stack
    let idx = runtime.frame_idxs.pop().context("not found frame index")?;
    let values = &mut runtime.stack.split_off(idx);

    // push results to stack
    for _ in 0..arity {
        runtime
            .stack
            .push(values.pop().context("not found frame result")?);
    }

    Ok(())
}

fn push_frame(runtime: &mut Runtime, arity: usize, locals: Vec<Value>) -> Result<()> {
    let frame = Frame { arity, locals };
    runtime.frame_idxs.push(runtime.stack.len());
    runtime.stack.push(frame.into());
    Ok(())
}

fn execute(runtime: &mut Runtime, insts: &Vec<Instruction>) -> Result<State> {
    for inst in insts {
        match inst {
            Instruction::Nop => {}
            Instruction::LocalGet(idx) => local_get(runtime, *idx as usize)?,
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
            Instruction::Return => {
                return Ok(State::Return);
            }
            Instruction::Drop => {
                runtime.stack.pop();
            }
            Instruction::MemoryGrow => {
                // TODO
            }
            Instruction::Loop(block) => {
                // if br 0, jump to the latest label in the stack
                let label = Label {
                    arity: block.block_type.result_count(),
                };
                runtime.stack.push(label.into());
                loop {
                    match execute(runtime, &block.then_body)? {
                        State::Continue => {
                            pop_label(runtime)?;
                            break;
                        }
                        State::Break(0) => {
                            pop_label(runtime)?; // pop current label
                        }
                        State::Break(level) => return Ok(State::Break(level - 1)),
                        State::Return => {
                            return Ok(State::Return);
                        }
                    }
                }
            }
            Instruction::If(block) => {
                let value: Value = runtime.stack.pop1()?;
                // if value is not true, skip until else or end
                let result = if value.is_true() {
                    execute(runtime, &block.then_body)?
                } else {
                    execute(runtime, &block.else_body)?
                };
                match result {
                    State::Continue => {}
                    State::Break(level) => return Ok(State::Break(level)),
                    State::Return => {
                        return Ok(State::Return);
                    }
                }
            }
            Instruction::Block(block) => {
                let arity = block.block_type.result_count();
                let label = Label { arity };

                runtime.stack.push(label.into());

                let result = execute(runtime, &block.then_body)?;
                match result {
                    State::Continue => {
                        pop_label(runtime)?;
                    }
                    State::Return => {
                        pop_frame(runtime)?;
                    }
                    State::Break(0) => {}
                    State::Break(level) => {}
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
                // do nothing
                //unimplemented!("{:?}", inst);
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
  (func (export "deep") (result i32)
    (loop (result i32) (block (result i32)
      (loop (result i32) (block (result i32)
        (loop (result i32) (block (result i32)
          (loop (result i32) (block (result i32)
            (loop (result i32) (block (result i32)
              (loop (result i32) (block (result i32)
                (loop (result i32) (block (result i32)
                  (loop (result i32) (block (result i32)
                    (loop (result i32) (block (result i32)
                      (loop (result i32) (block (result i32)
                        (loop (result i32) (block (result i32)
                          (loop (result i32) (block (result i32)
                            (loop (result i32) (block (result i32)
                              (loop (result i32) (block (result i32)
                                (loop (result i32) (block (result i32)
                                  (loop (result i32) (block (result i32)
                                    (loop (result i32) (block (result i32)
                                      (loop (result i32) (block (result i32)
                                        (loop (result i32) (block (result i32)
                                          (loop (result i32) (block (result i32)
                                            (call $dummy) (i32.const 150)
                                          ))
                                        ))
                                      ))
                                    ))
                                  ))
                                ))
                              ))
                            ))
                          ))
                        ))
                      ))
                    ))
                  ))
                ))
              ))
            ))
          ))
        ))
      ))
    ))
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
                ("deep", vec![], 150),
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
