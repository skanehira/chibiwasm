pub mod error;
pub(crate) mod float;
pub(crate) mod integer;
pub(crate) mod op;
pub mod value;

use crate::binary::instruction::*;
use crate::binary::module::{Decoder, Module};
use crate::binary::types::ExportDesc;
use crate::binary::types::FuncType;
use anyhow::{bail, Context as _, Result};
use op::*;
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
use value::{ExternalVal, Function, Value};

#[derive(Debug)]
pub struct ExportInst(HashMap<String, ExternalVal>);

impl ExportInst {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn add(&mut self, key: String, value: ExternalVal) {
        self.0.insert(key, value);
    }

    fn get(&self, key: &str) -> Option<&ExternalVal> {
        self.0.get(key)
    }
}

#[derive(Debug)]
pub struct Runtime {
    pub exports: ExportInst,
    pub functions: Vec<Function>, // for fetch instructions of function
    pub stack_frame: Vec<Frame>,  // stack frame
    pub value_stack: Vec<Value>,  // value stack
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
        let functions = new_functions(module)?;
        let mut export = ExportInst::new();
        for ex in module
            .export_section
            .as_ref()
            .context("not found export section")?
            .iter()
        {
            export.add(ex.name.clone(), ex.desc.clone().into());
        }
        Ok(Self {
            exports: export,
            functions,
            stack_frame: vec![],
            value_stack: vec![],
        })
    }

    pub fn invoke(&mut self, func_name: String, args: Vec<Value>) -> Result<Option<Value>> {
        let func = self.resolve_func(func_name)?;
        let frame = Frame::new(func.body.clone(), args);
        self.stack_frame.push(frame);
        self.execute()
    }

    fn resolve_func(&mut self, func_name: String) -> Result<&Function> {
        let external_val = self
            .exports
            .get(&func_name)
            .context(format!("not found function {func_name}"))?;
        let idx = match external_val {
            ExternalVal::Func(i) => *i,
            _ => bail!("invalid export desc: {:?}", external_val),
        };
        self.functions.get(idx as usize).context("")
    }

    pub fn stack_pop(&mut self) -> Result<Value> {
        self.value_stack
            .pop()
            .context("not found variable from stack")
    }

    fn execute(&mut self) -> Result<Option<Value>> {
        while let Some(inst) = self.instruction()? {
            self.frame_pc_inc()?;
            match inst {
                Instruction::LocalGet(idx) => local_get(self, idx as usize)?,
                Instruction::I32Add | Instruction::I64Add => add(self)?,
                Instruction::I32Sub | Instruction::I64Sub => sub(self)?,
                Instruction::I32Mul | Instruction::I64Mul => mul(self)?,
                Instruction::I32Clz | Instruction::I64Clz => clz(self)?,
                Instruction::I32Ctz | Instruction::I64Ctz => ctz(self)?,
                Instruction::I32DivU | Instruction::I64DivU => div_u(self)?,
                Instruction::I32DivS | Instruction::I64DivS => div_s(self)?,
                Instruction::I32Eq | Instruction::I64Eq => equal(self)?,
                Instruction::I32Eqz | Instruction::I64Eqz => eqz(self)?,
                Instruction::I32Ne | Instruction::I64Ne => not_equal(self)?,
                Instruction::I32LtS | Instruction::I64LtS => lt_s(self)?,
                Instruction::I32LtU | Instruction::I64LtU => lt_u(self)?,
                Instruction::I32GtS | Instruction::I64GtS => gt_s(self)?,
                Instruction::I32GtU | Instruction::I64GtU => gt_u(self)?,
                Instruction::I32LeS | Instruction::I64LeS => le_s(self)?,
                Instruction::I32LeU | Instruction::I64LeU => le_u(self)?,
                Instruction::I32GeS | Instruction::I64GeS => ge_s(self)?,
                Instruction::I32GeU | Instruction::I64GeU => ge_u(self)?,
                Instruction::I32Popcnt | Instruction::I64Popcnt => popcnt(self)?,
                Instruction::I32RemU | Instruction::I64RemU => rem_u(self)?,
                Instruction::I32RemS | Instruction::I64RemS => rem_s(self)?,
                Instruction::I32And | Instruction::I64And => and(self)?,
                Instruction::I32Or | Instruction::I64Or => or(self)?,
                Instruction::I32Xor | Instruction::I64Xor => xor(self)?,
                Instruction::I32ShL | Instruction::I64ShL => shl(self)?,
                Instruction::I32ShrU | Instruction::I64ShrU => shr_u(self)?,
                Instruction::I32ShrS | Instruction::I64ShrS => shr_s(self)?,
                Instruction::I32RtoL | Instruction::I64RtoL => rotl(self)?,
                Instruction::I32RtoR | Instruction::I64RtoR => rotr(self)?,
                Instruction::I32Extend8S | Instruction::I64Extend8S => extend8_s(self)?,
                Instruction::I32Extend16S | Instruction::I64Extend16S => extend16_s(self)?,
                Instruction::I32Const(v) => push(self, v)?,
                Instruction::I64Extend32S => i64extend_32s(self)?,
                Instruction::I64Const(v) => push(self, v)?,
                Instruction::F32Add | Instruction::F64Add => add(self)?,
                Instruction::F32Sub | Instruction::F64Sub => sub(self)?,
                Instruction::F32Mul | Instruction::F64Mul => mul(self)?,
                Instruction::F32Div | Instruction::F64Div => div(self)?,
                Instruction::F32Ceil | Instruction::F64Ceil => ceil(self)?,
                Instruction::F32Floor | Instruction::F64Floor => floor(self)?,
                Instruction::F32Max | Instruction::F64Max => max(self)?,
                Instruction::F32Min | Instruction::F64Min => min(self)?,
                Instruction::F32Nearest | Instruction::F64Nearest => nearest(self)?,
                Instruction::F32Sqrt | Instruction::F64Sqrt => sqrt(self)?,
                Instruction::F32Trunc | Instruction::F64Trunc => trunc(self)?,
                Instruction::F32Copysign | Instruction::F64Copysign => copysign(self)?,
                Instruction::F32Abs | Instruction::F64Abs => abs(self)?,
                Instruction::F32Neg | Instruction::F64Neg => neg(self)?,
                Instruction::F32Eq | Instruction::F64Eq => equal(self)?,
                Instruction::F32Ne | Instruction::F64Ne => not_equal(self)?,
                Instruction::F32Lt | Instruction::F64Lt => flt(self)?,
                Instruction::F32Gt | Instruction::F64Gt => fgt(self)?,
                Instruction::F32Le | Instruction::F64Le => fle(self)?,
                Instruction::F32Ge | Instruction::F64Ge => fge(self)?,
                Instruction::Return => {
                    // FIXME: we make stack frame in the if/else, so we need to pop stack
                    // frame tow times
                    self.stack_frame.pop();
                    self.stack_frame.pop();
                }
                Instruction::Drop => {
                    self.stack_pop()?;
                }
                Instruction::Void | Instruction::End => {
                    // do nothing
                }
                Instruction::If(block) => {
                    let value = self.stack_pop()?;
                    // if value is not true, skip until else or end
                    if value.is_true() {
                        // FIXME: we shouldn't use stack frame for if
                        let frame = Frame::new(block.then_body, vec![]);
                        self.stack_frame.push(frame);
                    } else {
                        let frame = Frame::new(block.else_body, vec![]);
                        self.stack_frame.push(frame);
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
                    let frame = Frame::new(body, args);
                    self.stack_frame.push(frame);
                }
                _ => {
                    dbg!(inst);
                    dbg!(self.instructions()?);
                    unimplemented!()
                }
            };
        }
        Ok(self.value_stack.pop())
    }

    fn instruction(&mut self) -> Result<Option<Instruction>> {
        loop {
            let frame = self.stack_frame.last();
            match frame {
                Some(frame) => {
                    let insts = frame.instructions.clone();
                    let inst = insts.get(frame.pc as usize);

                    if inst.is_some() {
                        return Ok(inst.cloned());
                    }
                    self.stack_frame.pop();
                }
                None => return Ok(None),
            }
        }
    }

    fn instructions(&mut self) -> Result<Vec<Instruction>> {
        let insts = self
            .stack_frame
            .last()
            .context("not found frame")?
            .instructions
            .clone();
        Ok(insts)
    }

    pub fn current_frame(&self) -> Result<&Frame> {
        self.stack_frame.last().context("not found frame")
    }

    fn frame_pc_inc(&mut self) -> Result<()> {
        self.stack_frame
            .last_mut()
            .context("not found frame")?
            .inc();
        Ok(())
    }
}

pub type Exports = HashMap<String, ExportDesc>;

#[derive(Debug)]
pub struct Frame {
    pub local_stack: Vec<Value>,
    pub pc: u32,
    pub instructions: Vec<Instruction>,
}

impl Frame {
    pub fn new(instructions: Vec<Instruction>, args: Vec<Value>) -> Self {
        Self {
            local_stack: args,
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

        let func_body = module
            .code_section
            .as_ref()
            .context("not found code section")?;
        let func_body = func_body.get(idx).context("not found function body")?;

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
    use anyhow::{Context, Result};
    use wasmer::wat2wasm;

    #[test]
    fn invoke() -> Result<()> {
        let wat_code = br#"
(module
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
  (func $if_else (export "if_else") (param $a i32) (result i32)
    (if (local.get $a)
      (then (return (i32.const 1)))
      (else (return (i32.const 0)))
    )
  )
  (func $if_else_empty (export "if_else_empty") (param $a i32) (result i32)
    (if (i32.eq (local.get $a) (i32.const 1))
      (then (return (i32.const 1)))
      (else (return (i32.const 0)))
    )
  )
)
"#;
        let wasm = &mut wat2wasm(wat_code)?;
        let mut runtime = Runtime::from_bytes(wasm)?;

        let tests = [
            ("call", vec![10, 10], 20),
            ("return", vec![], 15),
            ("if", vec![1, 0], 0),
            ("if_else", vec![1], 1),
            ("if_else", vec![0], 0),
            ("fib", vec![10], 55),
        ];

        for test in tests.into_iter() {
            let args = test.1.into_iter().map(Value::from).collect();
            let result = runtime.invoke(test.0.into(), args)?;
            print!("testing ... {} ", test.0);
            assert_eq!(
                result.context("no result")?,
                test.2.into(),
                "func {} fail",
                test.0
            );
            println!("ok");
        }

        Ok(())
    }
}
