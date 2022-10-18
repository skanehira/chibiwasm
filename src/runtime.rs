use crate::instruction::{self, Instruction};
use crate::section::{Export, ExportDesc, FunctionBody, Section};
use crate::types::{FuncType, ValueType};
use crate::value::{Function, Value};
use crate::Module;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;

#[derive(Debug, Default)]
pub struct Runtime {
    exports: HashMap<String, ExportDesc>,
    functions: Vec<Function>, // for fetch instructions of function
    frames: Vec<Frame>,       // stack frame
    stack: Vec<Value>,        // value stack
}

impl Runtime {
    pub fn new(module: &mut Module) -> Result<Self> {
        let functions = new_functions(module)?;
        let mut exports = HashMap::<String, ExportDesc>::new();
        for ex in module
            .export_section
            .as_ref()
            .context("not found export section")?
            .iter()
        {
            exports.insert(ex.name.clone(), ex.desc.clone());
        }
        Ok(Self {
            exports,
            functions,
            frames: vec![],
            stack: vec![],
        })
    }

    pub fn invoke(&mut self, func_name: String, args: &mut Vec<Value>) -> Result<Option<Value>> {
        let func = self.resolve_func(func_name)?;
        let frame = Frame::new(func.body.clone(), args);
        self.frames.push(frame);
        self.execute()
    }

    fn resolve_func(&mut self, func_name: String) -> Result<&Function> {
        let desc = self.exports.get(&func_name).context("not found function")?;
        let idx = match desc {
            ExportDesc::Func(i) => *i,
            _ => bail!("invalid export desc: {:?}", desc),
        };
        self.functions.get(idx as usize).context("")
    }

    fn stack_pop(&mut self) -> Result<Value> {
        self.stack.pop().context("not found variable from stack")
    }

    fn execute(&mut self) -> Result<Option<Value>> {
        while let Some(inst) = self.instruction()? {
            self.frame_pc_inc();
            match inst {
                Instruction::LocalGet(idx) => {
                    let value = self
                        .current_frame()?
                        .local_stack
                        .get(idx as usize)
                        .context("not found local variable")?;
                    self.stack.push(value.clone());
                }
                Instruction::I32Add => {
                    let b = self.stack_pop()?;
                    let a = self.stack_pop()?;
                    self.stack.push(a + b);
                }
                Instruction::I32Sub => {
                    let b = self.stack_pop()?;
                    let a = self.stack_pop()?;
                    self.stack.push(a - b);
                }
                Instruction::I32Mul => {
                    let b = self.stack_pop()?;
                    let a = self.stack_pop()?;
                    self.stack.push(a * b);
                }
                Instruction::I32DivU | Instruction::I32DivS => {
                    let b = self.stack_pop()?;
                    let a = self.stack_pop()?;
                    self.stack.push(a / b);
                }
                Instruction::I32Eq => {
                    let b = self.stack_pop()?;
                    let a = self.stack_pop()?;
                    let v = i32::from(a == b);
                    self.stack.push(v.into());
                }
                Instruction::I32Const(v) => {
                    self.stack.push(v.into());
                }
                Instruction::Return => {
                    self.frames.pop();
                }
                Instruction::Void | Instruction::End => {
                    // do nothing
                }
                Instruction::If => {
                    let v = self.stack_pop()?;
                    if v != Value::from(1) {
                        loop {
                            let ins = self.instruction()?.context("not found instruction")?;
                            if ins == Instruction::End || ins == Instruction::Else {
                                self.frame_pc_inc();
                                break;
                            }
                            self.frame_pc_inc();
                        }
                    }
                }
                Instruction::Call(func_idx) => {
                    let func = self
                        .functions
                        .get(func_idx as usize)
                        .context("not found function")?;
                    let body = func.body.clone();

                    let mut args = vec![];
                    for _ in 0..func.func_type.params.len() {
                        args.push(self.stack_pop()?);
                    }
                    let frame = Frame::new(body, &mut args);
                    self.frames.push(frame);
                    let result = self.execute()?;
                    if let Some(value) = result {
                        self.stack.push(value);
                    }
                }
                _ => {
                    dbg!(inst);
                    dbg!(self.instructions()?);
                    unimplemented!()
                }
            };
        }
        Ok(self.stack.pop())
    }

    fn instruction(&mut self) -> Result<Option<Instruction>> {
        loop {
            let frame = self.frames.last();

            if frame.is_none() {
                return Ok(None);
            }
            let insts = self.instructions()?;
            let inst = insts.get(self.frame_pc()?);

            if inst.is_some() {
                return Ok(inst.cloned());
            }
            self.frames.pop();
        }
    }

    fn instructions(&mut self) -> Result<Vec<Instruction>> {
        let insts = self
            .frames
            .last()
            .context("not found frame")?
            .instructions
            .clone();
        Ok(insts)
    }

    fn current_frame(&self) -> Result<&Frame> {
        self.frames.last().context("not found frame")
    }

    fn frame_pc(&mut self) -> Result<usize> {
        Ok(self.frames.last_mut().context("not found frame")?.pc as usize)
    }

    fn frame_pc_inc(&mut self) -> Result<()> {
        self.frames.last_mut().context("not found frame")?.pc += 1;
        Ok(())
    }
}

pub type Exports = HashMap<String, ExportDesc>;

#[derive(Debug)]
pub struct Frame {
    local_stack: Vec<Value>,
    pc: u32,
    instructions: Vec<Instruction>,
}

impl Frame {
    pub fn new(instructions: Vec<Instruction>, args: &mut Vec<Value>) -> Self {
        let mut stack = vec![];
        stack.append(args);
        Self {
            local_stack: stack,
            pc: 0,
            instructions,
        }
    }

    pub fn inc(&mut self) {
        self.pc += 1;
    }
}

fn new_functions(module: &mut Module) -> Result<Vec<Function>> {
    let mut functions: Vec<Function> = vec![];
    // 'idx' is index of function table
    // 'func_sing_idx' is indx of function signature
    for (idx, func_sig_idx) in module
        .function_section
        .as_ref()
        .context("not noud function section")?
        .iter()
        .enumerate()
    {
        let t = module
            .type_section
            .as_ref()
            .context("not found type section")?;
        let t = t
            .get(*func_sig_idx as usize)
            .context("cannot get type section")?;

        let func_type = FuncType {
            params: t.params.clone(),
            results: t.results.clone(),
        };

        let mut func_body = module
            .code_section
            .as_ref()
            .context("not found code section")?;
        let mut func_body = func_body.get(idx).context("not found function body")?;

        let func = Function {
            func_type,
            body: func_body.code.clone(),
        };
        functions.push(func);
    }
    Ok(functions)
}

#[cfg(test)]
mod test {
    use super::{Runtime, Value};
    use crate::Decoder;
    use anyhow::{Context, Result};
    use std::{
        fs,
        io::{self, BufReader, Cursor},
    };
    use wasmer::wat2wasm;

    #[test]
    fn invoke() -> Result<()> {
        let wat_code = br#"
(module
  (func $add (param $lhs i32) (param $rhs i32) (result i32)
    local.get $lhs
    local.get $rhs
    i32.add
	)
  (func $sub (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.sub
	)
  (func $mul (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.mul
  )
  (func $div_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.div_u
  )
  (func $div_s (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.div_s
  )
  (func $eq (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.eq
	)
  (func $call_add (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    call $add
	)
  (func $const_i32 (result i32)
    i32.const 1
    i32.const 1
    i32.add
  )
  (func $return_value (result i32)
    (return (i32.const 15))
  )
  (func $test_if (param $a i32) (param $b i32) (result i32)
    (if
      (i32.eq (local.get $a) (local.get $b))
      (then (return (i32.const 1)))
    )
    (return (i32.const 0))
  )
  (func $fib (param $N i32) (result i32)
    (if
      (i32.eq (local.get $N) (i32.const 1))
      (then (return (i32.const 1)))
    )
    (if
      (i32.eq (local.get $N) (i32.const 2))
      (then (return (i32.const 1)))
    )
    (i32.add (call $fib
      (i32.sub (local.get $N) (i32.const 1)))
      (call $fib (i32.sub (local.get $N) (i32.const 2)))
    )
  )
  (func $if_else (param $a i32) (result i32)
    (if (i32.eq (local.get $a) (i32.const 1))
      (then (return (i32.const 1)))
      (else (return (i32.const 0)))
    )
    (return (i32.const -1))
  )
  (export "add" (func $add))
  (export "sub" (func $sub))
  (export "mul" (func $mul))
  (export "div_u" (func $div_u))
  (export "div_s" (func $div_s))
  (export "call_add" (func $call_add))
  (export "eq" (func $eq))
  (export "const_i32" (func $const_i32))
  (export "return_value" (func $return_value))
  (export "test_if" (func $test_if))
  (export "fib" (func $fib))
  (export "if_else" (func $if_else))
)
"#;
        let wasm = wat2wasm(wat_code)?;
        let reader = Cursor::new(wasm);
        let mut decoder = Decoder::new(reader);
        let mut module = decoder.decode()?;
        let mut runtime = Runtime::new(&mut module)?;

        let tests = [
            ("add", vec![10, 11], 21),
            ("sub", vec![10, 11], -1),
            ("div_u", vec![100, 20], 5),
            ("div_s", vec![-10, -2], 5),
            ("mul", vec![10, 10], 100),
            ("eq", vec![10, 10], 1),
            ("call_add", vec![10, 10], 20),
            ("const_i32", vec![], 2),
            ("return_value", vec![], 15),
            ("test_if", vec![1, 0], 0),
            ("fib", vec![10], 55),
            ("fib", vec![1], 1),
            ("fib", vec![2], 1),
            ("fib", vec![4], 3),
            ("fib", vec![5], 5),
            ("fib", vec![6], 8),
            ("fib", vec![8], 21),
            ("if_else", vec![1], 1),
            ("if_else", vec![0], 0),
        ];

        for mut test in tests.into_iter() {
            let args = test.1.into_iter().map(Value::from);
            let result = runtime.invoke(test.0.into(), &mut args.into_iter().collect())?;
            assert_eq!(result.unwrap(), Value::from(test.2))
        }

        Ok(())
    }
}
