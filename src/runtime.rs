use crate::instruction::*;
use crate::module::{Decoder, Module};
use crate::section::ExportDesc;
use crate::types::FuncType;
use crate::value::{Function, Value};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Read;

#[derive(Debug, Default)]
pub struct Runtime {
    pub exports: HashMap<String, ExportDesc>,
    pub functions: Vec<Function>, // for fetch instructions of function
    pub stack_frame: Vec<Frame>,       // stack frame
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
        let desc = self
            .exports
            .get(&func_name)
            .context(format!("not found function {func_name}"))?;
        let idx = match desc {
            ExportDesc::Func(i) => *i,
            _ => bail!("invalid export desc: {:?}", desc),
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
                    self.stack_frame.pop();
                }
                Instruction::Void | Instruction::End => {
                    // do nothing
                }
                Instruction::If => {
                    let v = self.stack_pop()?;
                    if v != 1.into() {
                        loop {
                            let ins = self.instruction()?.context("not found instruction")?;
                            match ins {
                                Instruction::End | Instruction::Else => {
                                    self.frame_pc_inc()?;
                                    break;
                                }
                                _ => {
                                    self.frame_pc_inc()?;
                                }
                            }
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
        self.stack_frame.last_mut().context("not found frame")?.pc += 1;
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
    use crate::module::Decoder;
    use anyhow::Result;
    use std::io::Cursor;
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
    (i32.add
      (call $fib (i32.sub (local.get $N) (i32.const 1)))
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
  (func (export "i32.shl") (param $x i32) (param $y i32) (result i32) (i32.shl (local.get $x) (local.get $y)))
  (func (export "i32.shr_s") (param $x i32) (param $y i32) (result i32) (i32.shr_s (local.get $x) (local.get $y)))
  (func (export "i32.shr_u") (param $x i32) (param $y i32) (result i32) (i32.shr_u (local.get $x) (local.get $y)))
  (func (export "i32.rtol") (param $x i32) (param $y i32) (result i32) (i32.rotl (local.get $x) (local.get $y)))
  (func (export "i32.rtor") (param $x i32) (param $y i32) (result i32) (i32.rotr (local.get $x) (local.get $y)))
  (func (export "i32.extend8_s") (param $x i32) (result i32) (i32.extend8_s (local.get $x)))
  (func (export "i32.extend16_s") (param $x i32) (result i32) (i32.extend16_s (local.get $x)))
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
            ("i32.shl", vec![1, 0], 1),
            ("i32.shl", vec![0x40000000, 1], -0x80000000),
            ("i32.shl", vec![-0x80000000, 1], 0),
            ("i32.shl", vec![1, 31], -0x80000000),
            ("i32.shr_u", vec![1, 1], 0),
            ("i32.shr_u", vec![0x7fffffff, 1], 0x3fffffff),
            ("i32.shr_u", vec![0x40000000, 1], 0x20000000),
            ("i32.shr_s", vec![1, 1], 0),
            ("i32.shr_s", vec![0x7fffffff, 1], 0x3fffffff),
            ("i32.shr_s", vec![0x40000000, 1], 0x20000000),
            ("i32.rtol", vec![1, 1], 2),
            ("i32.rtol", vec![1, 31], -0x80000000),
            ("i32.rtol", vec![1, 32], 1),
            ("i32.rtor", vec![1, 1], -0x80000000),
            ("i32.rtor", vec![1, 0], 1),
            ("i32.rtor", vec![1, 32], 1),
            ("i32.extend8_s", vec![0], 0),
            ("i32.extend8_s", vec![0x80], -128),
            ("i32.extend8_s", vec![-1], -1),
            ("i32.extend16_s", vec![0], 0),
            ("i32.extend16_s", vec![0x7fff], 32767),
            ("i32.extend16_s", vec![-1], -1),
            ("call", vec![10, 10], 20),
            ("return", vec![], 15),
            ("if", vec![1, 0], 0),
            ("if_else", vec![1], 1),
            ("if_else", vec![0], 0),
            ("fib", vec![1], 1),
            ("fib", vec![2], 1),
            ("fib", vec![4], 3),
            ("fib", vec![5], 5),
            ("fib", vec![6], 8),
            ("fib", vec![8], 21),
            ("fib", vec![10], 55),
        ];

        for test in tests.into_iter() {
            let args = test.1.into_iter().map(Value::from);
            let result = runtime.invoke(test.0.into(), args.into_iter().collect())?;
            assert_eq!(result.unwrap(), Value::from(test.2), "func {}", test.0)
        }

        Ok(())
    }
}
