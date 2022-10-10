use crate::instruction::{Instruction, Opcode};
use crate::value::{FuncType, ValueType};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::{
    io::{BufRead, BufReader, Cursor, Read},
    u8,
};

#[derive(Debug)]
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

    fn num(&mut self) -> Result<u32> {
        let num: u32 = self.byte()?.try_into()?;
        Ok(num)
    }

    fn bytes(&mut self, num: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; num];
        self.buf.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn is_end(&mut self) -> Result<bool> {
        Ok(self.buf.has_data_left()?)
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
            _ => bail!("Unimplemented: {:x}", id as u8),
        };
        Ok(section)
    }

    fn decode_type_section(reader: &mut ContentsReader) -> Result<Section> {
        let mut func_types: Vec<FuncType> = vec![];
        let count = reader.num()?;

        // read each func types
        for _ in 0..count {
            let func_type = reader.byte()?;
            if 0x60 != func_type {
                bail!("invalid func type: {:x}", func_type);
            }
            let mut func = FuncType::default();

            // read each params
            let num_params = reader.num()?;
            for _ in 0..num_params {
                let value_type: ValueType = reader.byte()?.into();
                func.params.push(value_type);
            }

            // read each results
            let num_results = reader.num()?;
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
        let count = reader.num()?;
        for _ in 0..count {
            func_idx.push(reader.num()?);
        }
        Ok(Section::Function(func_idx))
    }

    fn decode_export_section(reader: &mut ContentsReader) -> Result<Section> {
        let count = reader.num()?;
        let mut exports: Vec<Export> = vec![];
        for _ in 0..count {
            // name of exported function
            let str_len = reader.num()?;
            let name = String::from_utf8(reader.bytes(str_len as usize)?)?;
            let kind = reader.byte()?;
            let idx = reader.num()?;
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
        let count = reader.num()?;

        for _ in 0..count {
            let body_size = reader.num()?;
            let mut body = ContentsReader::new(reader.bytes(body_size as usize)?);
            functions.push(Section::decode_function_body(&mut body)?);
        }
        Ok(Section::Code(functions))
    }

    fn decode_function_body(reader: &mut ContentsReader) -> Result<FunctionBody> {
        let mut function_body = FunctionBody::default();
        let local_count = reader.num()?;
        for _ in 0..local_count {
            let type_count = reader.num()?;
            let value_type: ValueType = reader.byte()?.into();
            function_body.locals.push(FunctionLocal {
                type_count,
                value_type,
            })
        }

        loop {
            let op = reader.byte()?;
            if op == 0x0b {
                break;
            }

            let op: Opcode = op.try_into()?;
            let inst = match op {
                Opcode::Unreachable => Instruction::Unreachable,
                Opcode::Nop => Instruction::Nop,
                Opcode::Call => {
                    let local_idx = reader.num()?;
                    Instruction::Call(local_idx)
                }
                Opcode::LocalGet => {
                    let local_idx = reader.num()?;
                    Instruction::LocalGet(local_idx)
                }
                Opcode::I32Sub => Instruction::I32Sub,
                Opcode::I32Add => Instruction::I32Add,
            };
            function_body.code.push(inst);
        }

        Ok(function_body)
    }
}