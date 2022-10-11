use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;

use anyhow::{bail, Context, Result};

use crate::instruction::{self, Instruction};
use crate::section::{Export, ExportDesc, FunctionBody, Section};
use crate::value::{FuncType, ValueType};
use crate::Module;

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
                    let a = self.stack.pop().context("not found variable from stack")?;
                    let b = self.stack.pop().context("not found variable from stack")?;
                    self.stack.push(a + b);
                }
                _ => unimplemented!(),
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

// https://webassembly.github.io/spec/core/exec/runtime.html#syntax-val
#[derive(Debug, Clone)]
pub enum Value {
    Num(Number),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Num(number) => {
                write!(f, "{}", number)
            }
            Value::Num(_) => todo!(),
        }
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Self::Num(Number::I32(v))
    }
}

impl std::ops::Add for Value {
    type Output = Value;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Num(Number::I32(a)), Self::Num(Number::I32(b))) => {
                Value::Num(Number::I32(a + b))
            }
            _ => panic!("cannot add values"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Number {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::I32(v) => write!(f, "{}", v),
            Number::I64(v) => write!(f, "{}", v),
            Number::F32(v) => write!(f, "{}", v),
            Number::F64(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Debug)]
pub struct Function {
    func_type: FuncType,
    body: Vec<Instruction>,
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
