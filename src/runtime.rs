use crate::instruction::{self, Instruction};
use crate::section::{Export, ExportDesc, FunctionBody, Section};
use crate::types::{FuncType, ValueType};
use crate::value::{Function, Value};
use crate::Module;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;

#[macro_export]
macro_rules! binop {
    ($self:expr, $f:expr, $ty:ty) => {{
        let b = $self.stack_pop()?;
        let a = $self.stack_pop()?;

        let result = match (a, b) {
            (Value::I32(lhs), Value::I32(rhs)) => $f(lhs, rhs) as $ty,
            (Value::I64(lhs), Value::I64(rhs)) => $f(lhs, rhs) as $ty,
            (Value::F32(lhs), Value::F32(rhs)) => $f(lhs, rhs) as $ty,
            (Value::F64(lhs), Value::F64(rhs)) => $f(lhs, rhs) as $ty,
            _ => panic!("Unsupported opration"),
        };
        $self.stack.push(result.into());
        Ok::<(), anyhow::Error>(())
    }};
}

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
                    binop!(self, |a, b| a + b, i32)?;
                }
                Instruction::I32Sub => {
                    binop!(self, |a, b| a - b, i32)?;
                }
                Instruction::I32Mul => {
                    binop!(self, |a, b| a * b, i32)?;
                }
                Instruction::I32Clz => {
                    let v = self.stack_pop()?;
                    match v {
                        Value::I32(v) => self.stack.push(Value::I32(v.leading_zeros() as i32)),
                        _ => bail!("unexpected value"),
                    }
                }
                Instruction::I32Ctz => {
                    let v = self.stack_pop()?;
                    match v {
                        Value::I32(v) => self.stack.push(Value::I32(v.trailing_zeros() as i32)),
                        _ => bail!("unexpected value"),
                    }
                }
                Instruction::I32DivU => {
                    binop!(self, |a, b| a / b, i32)?;
                }
                Instruction::I32DivS => {
                    binop!(self, |a, b| a / b, i32)?;
                }
                Instruction::I32Eq => {
                    binop!(self, |a, b| a == b, i32)?;
                }
                Instruction::I32Eqz => {
                    let v = self.stack_pop()?;
                    self.stack.push(i32::from(v == Value::from(0)).into());
                }
                Instruction::I32Ne => {
                    binop!(self, |a, b| a != b, i32)?;
                }
                Instruction::I32LtS => {
                    binop!(self, |a, b| a < b, i32)?;
                }
                Instruction::I32LtU => {
                    binop!(self, |a, b| a < b, i32)?;
                }
                Instruction::I32GtS => {
                    binop!(self, |a, b| a > b, i32)?;
                }
                Instruction::I32GtU => {
                    binop!(self, |a, b| a > b, i32)?;
                }
                Instruction::I32LeS => {
                    binop!(self, |a, b| a <= b, i32)?;
                }
                Instruction::I32LeU => {
                    binop!(self, |a, b| a <= b, i32)?;
                }
                Instruction::I32GeS => {
                    binop!(self, |a, b| a >= b, i32)?;
                }
                Instruction::I32GeU => {
                    binop!(self, |a, b| a >= b, i32)?;
                }
                Instruction::I32Popcnt => {
                    let value = self.stack_pop()?;
                    match value {
                        Value::I32(v) => self.stack.push(v.count_ones().into()),
                        _ => bail!("unexpected value"),
                    }
                }
                Instruction::I32RemU => {
                    binop!(self, |a, b| a % b, i32)?;
                }
                Instruction::I32RemS => {
                    binop!(self, |a, b| a % b, i32)?;
                }
                Instruction::I32And => {
                    binop!(self, |a, b| a as i32 & b as i32, i32)?;
                }
                Instruction::I32Or => {
                    binop!(self, |a, b| a as i32 | b as i32, i32)?;
                }
                Instruction::I32Xor => {
                    binop!(self, |a, b| a as i32 ^ b as i32, i32)?;
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
                    if v != 1.into() {
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
                        .context("not found function with index")?;
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
  (func $i32.add (param $lhs i32) (param $rhs i32) (result i32)
    local.get $lhs
    local.get $rhs
    i32.add
  )
  (func $i32.sub (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.sub
  )
  (func $i32.mul (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.mul
  )
  (func $i32.clz (param $a i32) (result i32)
    local.get $a
    i32.clz
  )
  (func $i32.ctz (param $a i32) (result i32)
    local.get $a
    i32.ctz
  )
  (func $i32.div_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.div_u
  )
  (func $i32.div_s (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.div_s
  )
  (func $i32.eq (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.eq
  )
  (func $i32.eqz (param $a i32) (result i32)
    local.get $a
    i32.eqz
  )
  (func $i32.ne (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.ne
  )
  (func $i32.lt_s (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.lt_s
  )
  (func $i32.lt_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.lt_u
  )
  (func $i32.gt_s (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.gt_s
  )
  (func $i32.gt_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.gt_u
  )
  (func $i32.le_s (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.le_s
  )
  (func $i32.le_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.le_u
  )
  (func $i32.ge_s (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.ge_s
  )
  (func $i32.ge_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.ge_u
  )
  (func $call (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    call $i32.add
  )
  (func $i32.const (result i32)
    i32.const 1
    i32.const 1
    i32.add
  )
  (func $return (result i32)
    (return (i32.const 15))
  )
  (func $if (param $a i32) (param $b i32) (result i32)
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
  (func (export "i32.popcnt") (param $x i32) (result i32) (i32.popcnt (local.get $x)))
  (func (export "i32.rem_s") (param $x i32) (param $y i32) (result i32) (i32.rem_s (local.get $x) (local.get $y)))
  (func (export "i32.rem_u") (param $x i32) (param $y i32) (result i32) (i32.rem_u (local.get $x) (local.get $y)))
  (func (export "i32.and") (param $x i32) (param $y i32) (result i32) (i32.and (local.get $x) (local.get $y)))
  (func (export "i32.or") (param $x i32) (param $y i32) (result i32) (i32.or (local.get $x) (local.get $y)))
  (func (export "i32.xor") (param $x i32) (param $y i32) (result i32) (i32.xor (local.get $x) (local.get $y)))
  (export "i32.add" (func $i32.add))
  (export "i32.sub" (func $i32.sub))
  (export "i32.mul" (func $i32.mul))
  (export "i32.clz" (func $i32.clz))
  (export "i32.ctz" (func $i32.ctz))
  (export "i32.div_u" (func $i32.div_u))
  (export "i32.div_s" (func $i32.div_s))
  (export "i32.eq" (func $i32.eq))
  (export "i32.eqz" (func $i32.eqz))
  (export "i32.ne" (func $i32.ne))
  (export "i32.lt_s" (func $i32.lt_s))
  (export "i32.lt_u" (func $i32.lt_u))
  (export "i32.gt_s" (func $i32.gt_s))
  (export "i32.gt_u" (func $i32.gt_u))
  (export "i32.le_s" (func $i32.le_s))
  (export "i32.le_u" (func $i32.le_u))
  (export "i32.ge_s" (func $i32.ge_s))
  (export "i32.ge_u" (func $i32.ge_u))
  (export "i32.const" (func $i32.const))
  (export "call" (func $call))
  (export "return" (func $return))
  (export "if" (func $if))
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
            ("i32.add", vec![10, 11], 21),
            ("i32.sub", vec![10, 11], -1),
            ("i32.div_u", vec![100, 20], 5),
            ("i32.div_s", vec![-10, -2], 5),
            ("i32.mul", vec![10, 10], 100),
            ("i32.clz", vec![(u32::MAX >> 2) as i32], 2),
            ("i32.clz", vec![(u32::MAX >> 5) as i32], 5),
            ("i32.ctz", vec![(u32::MAX << 2) as i32], 2),
            ("i32.ctz", vec![(u32::MAX << 5) as i32], 5),
            ("i32.eq", vec![10, 10], 1),
            ("i32.eq", vec![10, 10], 1),
            ("i32.eqz", vec![1], 0),
            ("i32.eqz", vec![0], 1),
            ("i32.ne", vec![10, 10], 0),
            ("i32.ne", vec![10, 11], 1),
            ("i32.lt_u", vec![10, 11], 1),
            ("i32.lt_u", vec![11, 11], 0),
            ("i32.lt_u", vec![12, 11], 0),
            ("i32.lt_s", vec![-10, -11], 0),
            ("i32.lt_s", vec![-11, -11], 0),
            ("i32.lt_s", vec![-12, -11], 1),
            ("i32.gt_u", vec![10, 11], 0),
            ("i32.gt_u", vec![11, 11], 0),
            ("i32.gt_u", vec![12, 11], 1),
            ("i32.gt_s", vec![-10, -11], 1),
            ("i32.gt_s", vec![-11, -11], 0),
            ("i32.gt_s", vec![-12, -11], 0),
            ("i32.le_u", vec![9, 10], 1),
            ("i32.le_u", vec![10, 10], 1),
            ("i32.le_u", vec![11, 10], 0),
            ("i32.le_s", vec![-10, -10], 1),
            ("i32.le_s", vec![-10, -9], 1),
            ("i32.le_s", vec![-10, -11], 0),
            ("i32.ge_u", vec![9, 10], 0),
            ("i32.ge_u", vec![10, 10], 1),
            ("i32.ge_u", vec![11, 10], 1),
            ("i32.ge_s", vec![-10, -10], 1),
            ("i32.ge_s", vec![-10, -9], 0),
            ("i32.ge_s", vec![-10, -11], 1),
            ("i32.const", vec![], 2),
            ("i32.popcnt", vec![0], 0),
            ("i32.popcnt", vec![2147483647], 31),
            ("i32.popcnt", vec![-1], 32),
            ("i32.rem_s", vec![-5, 2], -1),
            ("i32.rem_u", vec![5, 2], 1),
            ("i32.and", vec![1, 1], 1),
            ("i32.and", vec![0, 1], 0),
            ("i32.and", vec![0, 0], 0),
            ("i32.or", vec![1, 0], 1),
            ("i32.or", vec![0, 0], 0),
            ("i32.xor", vec![1, 1], 0),
            ("i32.xor", vec![0, 0], 0),
            ("i32.xor", vec![1, 0], 1),
            ("call", vec![10, 10], 20),
            ("return", vec![], 15),
            ("if", vec![1, 0], 0),
            ("if_else", vec![1], 1),
            ("if_else", vec![0], 0),
            ("fib", vec![10], 55),
            ("fib", vec![1], 1),
            ("fib", vec![2], 1),
            ("fib", vec![4], 3),
            ("fib", vec![5], 5),
            ("fib", vec![6], 8),
            ("fib", vec![8], 21),
        ];

        for mut test in tests.into_iter() {
            let args = test.1.into_iter().map(Value::from);
            let result = runtime.invoke(test.0.into(), &mut args.into_iter().collect())?;
            assert_eq!(result.unwrap(), Value::from(test.2), "func {}", test.0)
        }

        Ok(())
    }
}
