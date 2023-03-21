use crate::error::Error::InvalidMemoryCountError;
use crate::instruction::{Instruction, Opcode};
use crate::types::{FuncType, ValueType};
use anyhow::{bail, Context, Result};
use num_traits::FromPrimitive;
use std::io::{BufRead, Cursor, Read};

#[derive(Debug, PartialEq, Eq)]
pub enum SectionID {
    Custom,
    Type,
    Import,
    Function,
    Table,
    Memory,
    Global,
    Export,
    Start,
    Element,
    Code,
    Data,
    DataCount,
    Unknown,
}

impl From<u8> for SectionID {
    fn from(id: u8) -> Self {
        match id {
            0x00 => SectionID::Custom,
            0x01 => SectionID::Type,
            0x02 => SectionID::Import,
            0x03 => SectionID::Function,
            0x04 => SectionID::Table,
            0x05 => SectionID::Memory,
            0x06 => SectionID::Global,
            0x07 => SectionID::Export,
            0x08 => SectionID::Start,
            0x09 => SectionID::Element,
            0x0a => SectionID::Code,
            0x0b => SectionID::Data,
            0x0c => SectionID::DataCount,
            _ => SectionID::Unknown,
        }
    }
}

// https://webassembly.github.io/spec/core/binary/modules.html#binary-codesec
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FunctionLocal {
    type_count: u32,
    value_type: ValueType,
}

#[derive(Debug, Default, Clone)]
pub struct FunctionBody {
    pub locals: Vec<FunctionLocal>,
    pub code: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub enum ExportDesc {
    Func(u32),
    Table(u32),
    Memory(u32),
    Global(u32),
}

#[derive(Debug)]
pub struct Export {
    pub name: String,
    pub desc: ExportDesc,
}

// https://webassembly.github.io/spec/core/binary/modules.html#sections
#[derive(Debug)]
pub enum Section {
    Type(Vec<FuncType>),
    Function(Vec<u32>),
    Code(Vec<FunctionBody>),
    Export(Vec<Export>),
    Mem(Vec<Mem>),
}

#[derive(Debug)]
pub struct Mem {
    pub limits: Limits,
}

#[derive(Debug)]
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}

pub struct ContentsReader {
    buf: Cursor<Vec<u8>>,
}

impl ContentsReader {
    fn new(buf: Vec<u8>) -> Self {
        Self {
            buf: Cursor::new(buf),
        }
    }

    fn byte(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.buf.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn u32(&mut self) -> Result<u32> {
        let num = leb128::read::unsigned(&mut self.buf)?;
        let num = u32::try_from(num)?;
        Ok(num)
    }

    fn f32(&mut self) -> Result<f32> {
        let num = leb128::read::unsigned(&mut self.buf)?;
        let num = f32::from_bits(num as u32);
        Ok(num)
    }

    fn i32(&mut self) -> Result<i32> {
        let num = leb128::read::signed(&mut self.buf)?;
        let num = i32::try_from(num)?;
        Ok(num)
    }

    fn i64(&mut self) -> Result<i64> {
        let num = leb128::read::signed(&mut self.buf)?;
        Ok(num)
    }

    fn bytes(&mut self, num: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; num];
        self.buf.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn is_end(&mut self) -> Result<bool> {
        Ok(self.buf.fill_buf().map(|b| !b.is_empty())?)
    }
}

impl Section {
    pub fn decode(id: SectionID, data: Vec<u8>) -> Result<Section> {
        let mut reader = ContentsReader::new(data);
        let section = match id {
            SectionID::Type => Section::decode_type_section(&mut reader)?,
            SectionID::Code => Section::decode_code_section(&mut reader)?,
            SectionID::Function => Section::decode_function_section(&mut reader)?,
            SectionID::Export => Section::decode_export_section(&mut reader)?,
            SectionID::Memory => Section::decode_memory_section(&mut reader)?,
            _ => bail!("Unimplemented: {:x}", id as u8),
        };
        Ok(section)
    }

    fn decode_memory_section(reader: &mut ContentsReader) -> Result<Section> {
        let count = reader.u32()?;
        let mut mems: Vec<Mem> = vec![];
        if count != 1 {
            bail!(InvalidMemoryCountError);
        }
        for _ in 0..count {
            let limits = reader.u32()?;
            let min = reader.u32()?;
            let max = if limits == 0x00 {
                None
            } else {
                let max = reader.u32()?;
                Some(max)
            };
            let mem = Mem {
                limits: Limits { min, max },
            };
            mems.push(mem);
        }
        Ok(Self::Mem(mems))
    }

    fn decode_type_section(reader: &mut ContentsReader) -> Result<Section> {
        let mut func_types: Vec<FuncType> = vec![];
        let count = reader.u32()?;

        // read each func types
        for _ in 0..count {
            let func_type = reader.byte()?;
            if 0x60 != func_type {
                bail!("invalid func type: {:x}", func_type);
            }
            let mut func = FuncType::default();

            // read each params
            let num_params = reader.u32()?;
            for _ in 0..num_params {
                let value_type: ValueType = reader.byte()?.into();
                func.params.push(value_type);
            }

            // read each results
            let num_results = reader.u32()?;
            for _ in 0..num_results {
                let value_type: ValueType = reader.byte()?.into();
                func.results.push(value_type);
            }

            func_types.push(func)
        }
        Ok(Section::Type(func_types))
    }

    fn decode_function_section(reader: &mut ContentsReader) -> Result<Section> {
        let mut func_idx: Vec<u32> = vec![];
        let count = reader.u32()?;
        for _ in 0..count {
            func_idx.push(reader.u32()?);
        }
        Ok(Section::Function(func_idx))
    }

    fn decode_export_section(reader: &mut ContentsReader) -> Result<Section> {
        let count = reader.u32()?;
        let mut exports: Vec<Export> = vec![];
        for _ in 0..count {
            // name of exported function
            let str_len = reader.u32()?;
            let name = String::from_utf8(reader.bytes(str_len as usize)?)?;
            let kind = reader.byte()?;
            let idx = reader.u32()?;
            let desc = match kind {
                0x00 => ExportDesc::Func(idx),
                0x01 => ExportDesc::Table(idx),
                0x02 => ExportDesc::Memory(idx),
                0x03 => ExportDesc::Global(idx),
                _ => bail!("invalid export desc: {:x}", kind),
            };
            exports.push(Export { name, desc })
        }
        Ok(Section::Export(exports))
    }

    fn decode_code_section(reader: &mut ContentsReader) -> Result<Section> {
        let mut functions: Vec<FunctionBody> = vec![];
        let count = reader.u32()?;

        for _ in 0..count {
            let body_size = reader.u32()?;
            let mut body = ContentsReader::new(reader.bytes(body_size as usize)?);
            functions.push(Section::decode_function_body(&mut body)?);
        }
        Ok(Section::Code(functions))
    }

    fn decode_function_body(reader: &mut ContentsReader) -> Result<FunctionBody> {
        let mut function_body = FunctionBody::default();
        let local_count = reader.u32()?;
        for _ in 0..local_count {
            let type_count = reader.u32()?;
            let value_type: ValueType = reader.byte()?.into();
            function_body.locals.push(FunctionLocal {
                type_count,
                value_type,
            })
        }

        while reader.is_end()? {
            let op = reader.byte()?;

            let op: Opcode =
                FromPrimitive::from_u8(op).context(format!("unsupported opcode: {op:X}"))?;

            let inst = match op {
                Opcode::Unreachable => Instruction::Unreachable,
                Opcode::Nop => Instruction::Nop,
                Opcode::Block => Instruction::Block,
                Opcode::Loop => Instruction::Loop,
                Opcode::Br => Instruction::Br,
                Opcode::Call => {
                    let local_idx = reader.u32()?;
                    Instruction::Call(local_idx)
                }
                Opcode::Return => Instruction::Return,
                Opcode::If => Instruction::If,
                Opcode::Else => Instruction::Else,
                Opcode::End => Instruction::End,
                Opcode::Void => Instruction::Void,
                Opcode::LocalGet => {
                    let local_idx = reader.u32()?;
                    Instruction::LocalGet(local_idx)
                }
                Opcode::I32Sub => Instruction::I32Sub,
                Opcode::I32Add => Instruction::I32Add,
                Opcode::I32Mul => Instruction::I32Mul,
                Opcode::I32Clz => Instruction::I32Clz,
                Opcode::I32Ctz => Instruction::I32Ctz,
                Opcode::I32DivU => Instruction::I32DivU,
                Opcode::I32DivS => Instruction::I32DivS,
                Opcode::I32Eq => Instruction::I32Eq,
                Opcode::I32Eqz => Instruction::I32Eqz,
                Opcode::I32Ne => Instruction::I32Ne,
                Opcode::I32LtS => Instruction::I32LtS,
                Opcode::I32LtU => Instruction::I32LtU,
                Opcode::I32GtS => Instruction::I32GtS,
                Opcode::I32GtU => Instruction::I32GtU,
                Opcode::I32LeS => Instruction::I32LeS,
                Opcode::I32GeU => Instruction::I32GeU,
                Opcode::I32GeS => Instruction::I32GeS,
                Opcode::I32LeU => Instruction::I32LeU,
                Opcode::I32Popcnt => Instruction::I32Popcnt,
                Opcode::I32RemS => Instruction::I32RemS,
                Opcode::I32RemU => Instruction::I32RemU,
                Opcode::I32And => Instruction::I32And,
                Opcode::I32Or => Instruction::I32Or,
                Opcode::I32Xor => Instruction::I32Xor,
                Opcode::I32ShL => Instruction::I32ShL,
                Opcode::I32ShrS => Instruction::I32ShrS,
                Opcode::I32ShrU => Instruction::I32ShrU,
                Opcode::I32RtoL => Instruction::I32RtoL,
                Opcode::I32RtoR => Instruction::I32RtoR,
                Opcode::I32Extend8S => Instruction::I32Extend8S,
                Opcode::I32Extend16S => Instruction::I32Extend16S,
                Opcode::I32Const => {
                    let value = reader.i32()?;
                    Instruction::I32Const(value)
                }
                Opcode::I64Sub => Instruction::I64Sub,
                Opcode::I64Add => Instruction::I64Add,
                Opcode::I64Mul => Instruction::I64Mul,
                Opcode::I64Clz => Instruction::I64Clz,
                Opcode::I64Ctz => Instruction::I64Ctz,
                Opcode::I64DivU => Instruction::I64DivU,
                Opcode::I64DivS => Instruction::I64DivS,
                Opcode::I64Eq => Instruction::I64Eq,
                Opcode::I64Eqz => Instruction::I64Eqz,
                Opcode::I64Ne => Instruction::I64Ne,
                Opcode::I64LtS => Instruction::I64LtS,
                Opcode::I64LtU => Instruction::I64LtU,
                Opcode::I64GtS => Instruction::I64GtS,
                Opcode::I64GtU => Instruction::I64GtU,
                Opcode::I64LeS => Instruction::I64LeS,
                Opcode::I64GeU => Instruction::I64GeU,
                Opcode::I64GeS => Instruction::I64GeS,
                Opcode::I64LeU => Instruction::I64LeU,
                Opcode::I64Popcnt => Instruction::I64Popcnt,
                Opcode::I64RemS => Instruction::I64RemS,
                Opcode::I64RemU => Instruction::I64RemU,
                Opcode::I64And => Instruction::I64And,
                Opcode::I64Or => Instruction::I64Or,
                Opcode::I64Xor => Instruction::I64Xor,
                Opcode::I64ShL => Instruction::I64ShL,
                Opcode::I64ShrS => Instruction::I64ShrS,
                Opcode::I64ShrU => Instruction::I64ShrU,
                Opcode::I64RtoL => Instruction::I64RtoL,
                Opcode::I64RtoR => Instruction::I64RtoR,
                Opcode::I64Extend8S => Instruction::I64Extend8S,
                Opcode::I64Extend16S => Instruction::I64Extend16S,
                Opcode::I64Extend32S => Instruction::I64Extend32S,
                Opcode::I64Const => {
                    let value = reader.i64()?;
                    Instruction::I64Const(value)
                }
                Opcode::F32Const => {
                    let num = reader.f32()?;
                    Instruction::F32Const(num)
                }
                Opcode::F32Eq => Instruction::F32Eq,
                Opcode::F32Ne => Instruction::F32Ne,
                Opcode::F32Lt => Instruction::F32Lt,
                Opcode::F32Gt => Instruction::F32Gt,
                Opcode::F32Le => Instruction::F32Le,
                Opcode::F32Ge => Instruction::F32Ge,
                Opcode::F32Abs => Instruction::F32Abs,
                Opcode::F32Neg => Instruction::F32Neg,
                Opcode::F32Ceil => Instruction::F32Ceil,
                Opcode::F32Floor => Instruction::F32Floor,
                Opcode::F32Trunc => Instruction::F32Trunc,
                Opcode::F32Nearest => Instruction::F32Nearest,
                Opcode::F32Sqrt => Instruction::F32Sqrt,
                Opcode::F32Add => Instruction::F32Add,
                Opcode::F32Sub => Instruction::F32Sub,
                Opcode::F32Mul => Instruction::F32Mul,
                Opcode::F32Div => Instruction::F32Div,
                Opcode::F32Min => Instruction::F32Min,
                Opcode::F32Max => Instruction::F32Max,
                Opcode::F32Copysign => Instruction::F32Copysign,
                Opcode::F64Eq => Instruction::F64Eq,
                Opcode::F64Ne => Instruction::F64Ne,
                Opcode::F64Lt => Instruction::F64Lt,
                Opcode::F64Gt => Instruction::F64Gt,
                Opcode::F64Le => Instruction::F64Le,
                Opcode::F64Ge => Instruction::F64Ge,
                Opcode::F64Abs => Instruction::F64Abs,
                Opcode::F64Neg => Instruction::F64Neg,
                Opcode::F64Ceil => Instruction::F64Ceil,
                Opcode::F64Floor => Instruction::F64Floor,
                Opcode::F64Trunc => Instruction::F64Trunc,
                Opcode::F64Nearest => Instruction::F64Nearest,
                Opcode::F64Sqrt => Instruction::F64Sqrt,
                Opcode::F64Add => Instruction::F64Add,
                Opcode::F64Sub => Instruction::F64Sub,
                Opcode::F64Mul => Instruction::F64Mul,
                Opcode::F64Div => Instruction::F64Div,
                Opcode::F64Min => Instruction::F64Min,
                Opcode::F64Max => Instruction::F64Max,
                Opcode::F64Copysign => Instruction::F64Copysign,
                Opcode::Drop => Instruction::Drop,
            };
            function_body.code.push(inst);
        }

        Ok(function_body)
    }
}
