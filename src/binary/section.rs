#![allow(clippy::needless_range_loop)]

use super::error::Error::*;
use super::instruction::{Instruction, MemoryArg, Opcode};
use super::types::*;
use anyhow::{bail, Context, Result};
use log::trace;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive as _;
use std::io::{BufRead, Cursor, Read};

#[derive(Debug, PartialEq, Eq, FromPrimitive)]
pub enum SectionID {
    Custom = 0x00,
    Type = 0x01,
    Import = 0x02,
    Function = 0x03,
    Table = 0x04,
    Memory = 0x05,
    Global = 0x06,
    Export = 0x07,
    Start = 0x08,
    Element = 0x09,
    Code = 0x0a,
    Data = 0x0b,
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
            0x0b => SectionID::Data,
            0x0a => SectionID::Code,
            _ => panic!("unknown section id: {}", id),
        }
    }
}

pub struct SectionReader<'a> {
    buf: Cursor<&'a [u8]>,
}

impl<'a> SectionReader<'a> {
    fn new(buf: &'a [u8]) -> Self {
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

    // https://www.w3.org/TR/wasm-core-1/#floating-point%E2%91%A4
    fn f32(&mut self) -> Result<f32> {
        let buf = &mut [0u8; 4];
        self.buf.read_exact(buf)?;
        Ok(f32::from_le_bytes(*buf))
    }

    fn f64(&mut self) -> Result<f64> {
        let buf = &mut [0u8; 8];
        self.buf.read_exact(buf)?;
        Ok(f64::from_le_bytes(*buf))
    }

    // https://www.w3.org/TR/wasm-core-1/#integers%E2%91%A4
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

    fn string(&mut self, size: usize) -> Result<String> {
        let bytes = self.bytes(size)?;
        let string = String::from_utf8(bytes)?;
        Ok(string)
    }

    fn is_end(&mut self) -> Result<bool> {
        Ok(self.buf.fill_buf().map(|b| !b.is_empty())?)
    }
}

// https://webassembly.github.io/spec/core/binary/modules.html#sections
#[derive(Debug)]
pub enum Section {
    Custom(Custom),
    Type(Vec<FuncType>),
    Import(Vec<Import>),
    Function(Vec<u32>),
    Table(Vec<Table>),
    Memory(Vec<Memory>), // only 1 memory for now
    Global(Vec<Global>),
    Export(Vec<Export>),
    Start(u32),
    Element(Vec<Element>),
    Data(Vec<Data>),
    Code(Vec<FunctionBody>),
}

pub fn decode(id: SectionID, data: &[u8]) -> Result<Section> {
    let mut reader = SectionReader::new(data);
    let section = match id {
        SectionID::Custom => decode_custom_section(&mut reader)?,
        SectionID::Type => decode_type_section(&mut reader)?,
        SectionID::Import => decode_import_section(&mut reader)?,
        SectionID::Function => decode_function_section(&mut reader)?,
        SectionID::Table => decode_table_secttion(&mut reader)?,
        SectionID::Memory => decode_memory_section(&mut reader)?,
        SectionID::Global => decode_global_section(&mut reader)?,
        SectionID::Export => decode_export_section(&mut reader)?,
        SectionID::Start => decode_start_section(&mut reader)?,
        SectionID::Element => decode_element_section(&mut reader)?,
        SectionID::Data => decode_data_section(&mut reader)?,
        SectionID::Code => decode_code_section(&mut reader)?,
    };
    Ok(section)
}

fn decode_custom_section(reader: &mut SectionReader) -> Result<Section> {
    let name_size = reader.u32()?;
    let name = reader.string(name_size as usize)?;
    let data = reader.bytes(reader.buf.get_ref().len() - reader.buf.position() as usize)?;
    Ok(Section::Custom(Custom { name, data }))
}

fn decode_data_section(reader: &mut SectionReader) -> Result<Section> {
    let mut data = vec![];
    let count = reader.u32()?;
    for _ in 0..count {
        let memory_index = reader.u32()?;
        let offset = decode_expr(reader)?;
        let size = reader.u32()?;
        let init = reader.bytes(size as usize)?;
        data.push(Data {
            memory_index,
            offset,
            init,
        });
    }

    Ok(Section::Data(data))
}

fn decode_element_section(reader: &mut SectionReader) -> Result<Section> {
    let mut elements = vec![];
    let count = reader.u32()?;
    for _ in 0..count {
        let mut init = vec![];
        let table_index = reader.u32()?;
        let offset = decode_expr(reader)?;
        let count = reader.u32()?;
        for _ in 0..count {
            let index = reader.u32()?;
            init.push(index);
        }
        elements.push(Element {
            table_index,
            offset,
            init,
        });
    }

    Ok(Section::Element(elements))
}

fn decode_start_section(reader: &mut SectionReader) -> Result<Section> {
    let index = reader.u32()?;
    Ok(Section::Start(index))
}

fn decode_import_section(reader: &mut SectionReader) -> Result<Section> {
    let count = reader.u32()?;
    let mut imports = vec![];

    for _ in 0..count {
        // module name
        let size = reader.u32()? as usize;
        let module_name = reader.string(size)?;

        // field name
        let size = reader.u32()? as usize;
        let field_name = reader.string(size)?;

        // implrt kind
        let import_kind = reader.byte()?;
        let kind = match import_kind {
            0x00 => {
                // function
                let type_index = reader.u32()?;
                ImportKind::Func(type_index)
            }
            0x01 => {
                // table
                let table = decode_table(reader)?;
                ImportKind::Table(table)
            }
            0x02 => {
                // memory
                let mem = decode_memory(reader)?;
                ImportKind::Memory(mem)
            }
            0x03 => {
                // global
                let global_type = decode_global_type(reader)?;
                ImportKind::Global(global_type)
            }
            _ => bail!(InvalidImportKind(import_kind)),
        };

        imports.push(Import {
            module_name,
            field_name,
            kind,
        })
    }

    Ok(Section::Import(imports))
}

fn decode_global_type(reader: &mut SectionReader) -> Result<GlobalType> {
    let value_type = reader.byte()?;
    let mutability = reader.byte()?;
    let global_type = GlobalType {
        value_type: value_type.into(),
        mutability: Mutability::from_u8(mutability).unwrap(),
    };
    Ok(global_type)
}

fn decode_global_section(reader: &mut SectionReader) -> Result<Section> {
    let count = reader.u32()?;
    let mut globals = vec![];
    for _ in 0..count {
        let global_type = decode_global_type(reader)?;
        let init_expr = decode_expr(reader)?;
        let global = Global {
            global_type,
            init_expr,
        };
        globals.push(global);
    }
    Ok(Section::Global(globals))
}

fn decode_expr(reader: &mut SectionReader) -> Result<ExprValue> {
    let byte = reader.byte()?;
    let opcode = Opcode::from_u8(byte).unwrap();
    let value = match opcode {
        Opcode::I32Const => {
            let value = reader.i32()?;
            ExprValue::I32(value)
        }
        Opcode::I64Const => {
            let value = reader.i64()?;
            ExprValue::I64(value)
        }
        Opcode::F32Const => {
            let value = reader.f32()?;
            ExprValue::F32(value)
        }
        Opcode::F64Const => {
            let value = reader.f64()?;
            ExprValue::F64(value)
        }
        _ => bail!(InvalidInitExprOpcode(byte)),
    };

    let end_opcode = Opcode::from_u8(reader.byte()?).unwrap();
    if end_opcode != Opcode::End {
        bail!(InvalidInitExprEndOpcode(end_opcode));
    }
    Ok(value)
}

fn decode_table(reader: &mut SectionReader) -> Result<Table> {
    let elem_type = reader.byte()?;
    if elem_type != 0x70 {
        bail!(InvalidElmType(elem_type));
    }
    let limits = decode_limits(reader)?;
    let table = Table {
        elem_type: ElemType::from_u8(elem_type).unwrap(),
        limits,
    };
    Ok(table)
}

fn decode_table_secttion(reader: &mut SectionReader) -> Result<Section> {
    let count = reader.u32()?;
    if count != 1 {
        bail!(InvalidTableCount);
    }
    let mut tables = vec![];
    for _ in 0..count {
        let table = decode_table(reader)?;
        tables.push(table);
    }
    Ok(Section::Table(tables))
}

fn decode_limits(reader: &mut SectionReader) -> Result<Limits> {
    let limits = reader.u32()?;
    let min = reader.u32()?;
    let max = if limits == 0x00 {
        None
    } else {
        let max = reader.u32()?;
        Some(max)
    };
    Ok(Limits { min, max })
}

fn decode_memory(reader: &mut SectionReader) -> Result<Memory> {
    let limits = decode_limits(reader)?;
    Ok(Memory { limits })
}

fn decode_memory_section(reader: &mut SectionReader) -> Result<Section> {
    let count = reader.u32()?;
    let mut mems: Vec<Memory> = vec![];
    if count != 1 {
        bail!(InvalidMemoryCount);
    }
    for _ in 0..count {
        mems.push(decode_memory(reader)?);
    }
    Ok(Section::Memory(mems))
}

fn decode_type_section(reader: &mut SectionReader) -> Result<Section> {
    let mut func_types: Vec<FuncType> = vec![];

    // size of function types
    let count = reader.u32()?;

    // read each func types
    for _ in 0..count {
        let func_type = reader.byte()?;
        if 0x60 != func_type {
            bail!("invalid func type: {:x}", func_type);
        }
        let mut func = FuncType::default();

        // read each params
        let size = reader.u32()?;
        for _ in 0..size {
            let value_type: ValueType = reader.byte()?.into();
            func.params.push(value_type);
        }

        // read each results
        let size = reader.u32()?;
        for _ in 0..size {
            let value_type: ValueType = reader.byte()?.into();
            func.results.push(value_type);
        }

        func_types.push(func)
    }
    Ok(Section::Type(func_types))
}

fn decode_function_section(reader: &mut SectionReader) -> Result<Section> {
    let mut func_idx: Vec<u32> = vec![];
    let count = reader.u32()?;
    for _ in 0..count {
        func_idx.push(reader.u32()?);
    }
    Ok(Section::Function(func_idx))
}

fn decode_export_section(reader: &mut SectionReader) -> Result<Section> {
    let count = reader.u32()?;
    let mut exports: Vec<Export> = vec![];
    for _ in 0..count {
        // name of exported function
        let str_len = reader.u32()?;
        let name = String::from_utf8(reader.bytes(str_len as usize)?)?;
        let exportkind = reader.byte()?;
        let idx = reader.u32()?;
        let desc = match exportkind {
            0x00 => ExportDesc::Func(idx),
            0x01 => ExportDesc::Table(idx),
            0x02 => ExportDesc::Memory(idx),
            0x03 => ExportDesc::Global(idx),
            _ => bail!("unknown export kind: {:x}", exportkind),
        };
        exports.push(Export { name, desc })
    }
    Ok(Section::Export(exports))
}

fn decode_code_section(reader: &mut SectionReader) -> Result<Section> {
    let mut functions: Vec<FunctionBody> = vec![];
    // size of function
    let count = reader.u32()?;

    for _ in 0..count {
        let func_body_size = reader.u32()?;
        let bytes = reader.bytes(func_body_size as usize)?;
        let mut body = SectionReader::new(&bytes);
        functions.push(decode_function_body(&mut body)?);
    }
    Ok(Section::Code(functions))
}

fn decode_function_body(reader: &mut SectionReader) -> Result<FunctionBody> {
    let mut function_body = FunctionBody::default();

    // count of local variable declarations
    let count = reader.u32()?;
    for _ in 0..count {
        let type_count = reader.u32()?;
        let value_type: ValueType = reader.byte()?.into();
        function_body.locals.push(FunctionLocal {
            type_count,
            value_type,
        })
    }

    while reader.is_end()? {
        let inst = decode_instruction(reader)?;
        function_body.code.push(inst);
    }

    Ok(function_body)
}

fn decode_block_type(reader: &mut SectionReader) -> Result<BlockType> {
    let byte = reader.byte()?;
    let block_type = if byte == 0x40 {
        BlockType::Empty
    } else {
        let value_type = byte.into();
        BlockType::Value(vec![value_type])
    };
    Ok(block_type)
}

fn decode_block(reader: &mut SectionReader) -> Result<Block> {
    let block_type = decode_block_type(reader)?;
    let mut then_body = vec![];
    let mut else_body = vec![];

    // blockの命令部分をデコードする
    loop {
        let inst = decode_instruction(reader)?;

        // elseがあったら、elseの命令部分をデコードする
        if inst == Instruction::Else {
            loop {
                let inst = decode_instruction(reader)?;
                if inst == Instruction::End {
                    break;
                }
                else_body.push(inst);
            }
            break;
        }

        // endがあったら、命令部分のデコードを終了する
        if inst == Instruction::End {
            break;
        }

        // それ以外はthenの命令部分として追加する
        then_body.push(inst);
    }

    let block = Block {
        block_type,
        then_body,
        else_body,
    };

    Ok(block)
}

fn decode_instruction(reader: &mut SectionReader) -> Result<Instruction> {
    let op = reader.byte()?;
    let op: Opcode =
        Opcode::from_u8(op).with_context(|| format!("unimplemented opcode: {:x}", op))?;
    trace!("decode opcode: {:?}", op);
    let inst = match op {
        Opcode::Unreachable => Instruction::Unreachable,
        Opcode::Nop => Instruction::Nop,
        Opcode::Block => Instruction::Block(decode_block(reader)?),
        Opcode::Loop => Instruction::Loop(decode_block(reader)?),
        Opcode::If => Instruction::If(decode_block(reader)?),
        Opcode::Else => Instruction::Else,
        Opcode::End => Instruction::End,
        Opcode::Br => Instruction::Br(reader.u32()?),
        Opcode::BrIf => Instruction::BrIf(reader.u32()?),
        Opcode::BrTable => {
            let count = reader.u32()? as usize;
            let mut indexs = vec![0; count];
            for i in 0..count {
                let index = reader.u32()?;
                indexs[i] = index;
            }
            let default = reader.u32()?;
            Instruction::BrTable(indexs, default)
        }
        Opcode::Call => {
            let local_idx = reader.u32()?;
            Instruction::Call(local_idx)
        }
        // first u32 is function signature index, second u32 is table index
        Opcode::CallIndirect => Instruction::CallIndirect((reader.u32()?, reader.u32()?)),
        Opcode::Return => Instruction::Return,
        Opcode::LocalGet => {
            let local_idx = reader.u32()?;
            Instruction::LocalGet(local_idx)
        }
        Opcode::LocalSet => Instruction::LocalSet(reader.u32()?),
        Opcode::LocalTee => Instruction::LocalTee(reader.u32()?),
        Opcode::GlobalSet => Instruction::GlobalSet(reader.u32()?),
        Opcode::GlobalGet => Instruction::GlobalGet(reader.u32()?),
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
        Opcode::I32WrapI64 => Instruction::I32WrapI64,
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
        Opcode::F64Const => {
            let num = reader.f64()?;
            Instruction::F64Const(num)
        }
        Opcode::Drop => Instruction::Drop,
        Opcode::I32Load => Instruction::I32Load(read_memory_arg(reader)?),
        Opcode::I64Load => Instruction::I64Load(read_memory_arg(reader)?),
        Opcode::F32Load => Instruction::F32Load(read_memory_arg(reader)?),
        Opcode::F64Load => Instruction::F64Load(read_memory_arg(reader)?),
        Opcode::I32Load8S => Instruction::I32Load8S(read_memory_arg(reader)?),
        Opcode::I32Load8U => Instruction::I32Load8U(read_memory_arg(reader)?),
        Opcode::I32Load16S => Instruction::I32Load16S(read_memory_arg(reader)?),
        Opcode::I32Load16U => Instruction::I32Load16U(read_memory_arg(reader)?),
        Opcode::I64Load8S => Instruction::I64Load8S(read_memory_arg(reader)?),
        Opcode::I64Load8U => Instruction::I64Load8U(read_memory_arg(reader)?),
        Opcode::I64Load16S => Instruction::I64Load16S(read_memory_arg(reader)?),
        Opcode::I64Load16U => Instruction::I64Load16U(read_memory_arg(reader)?),
        Opcode::I64Load32S => Instruction::I64Load32S(read_memory_arg(reader)?),
        Opcode::I64Load32U => Instruction::I64Load32U(read_memory_arg(reader)?),
        Opcode::I32Store => Instruction::I32Store(read_memory_arg(reader)?),
        Opcode::I64Store => Instruction::I64Store(read_memory_arg(reader)?),
        Opcode::F32Store => Instruction::F32Store(read_memory_arg(reader)?),
        Opcode::F64Store => Instruction::F64Store(read_memory_arg(reader)?),
        Opcode::I32Store8 => Instruction::I32Store8(read_memory_arg(reader)?),
        Opcode::I32Store16 => Instruction::I32Store16(read_memory_arg(reader)?),
        Opcode::I64Store8 => Instruction::I64Store8(read_memory_arg(reader)?),
        Opcode::I64Store16 => Instruction::I64Store16(read_memory_arg(reader)?),
        Opcode::I64Store32 => Instruction::I64Store32(read_memory_arg(reader)?),
        Opcode::MemoryGrow => Instruction::MemoryGrow(reader.u32()?),
        Opcode::MemorySize => {
            // NOTE: memory index is always 0 now
            let _ = reader.byte();
            Instruction::MemorySize
        }
        Opcode::Select => Instruction::Select,
        Opcode::I32TruncF32S => Instruction::I32TruncF32S,
        Opcode::I32TruncF32U => Instruction::I32TruncF32U,
        Opcode::I32TruncF64S => Instruction::I32TruncF64S,
        Opcode::I32TruncF64U => Instruction::I32TruncF64U,
        Opcode::I64ExtendI32S => Instruction::I64ExtendI32S,
        Opcode::I64ExtendI32U => Instruction::I64ExtendI32U,
        Opcode::I64TruncF32S => Instruction::I64TruncF32S,
        Opcode::I64TruncF32U => Instruction::I64TruncF32U,
        Opcode::I64TruncF64S => Instruction::I64TruncF64S,
        Opcode::I64TruncF64U => Instruction::I64TruncF64U,
        Opcode::F32ConvertI32S => Instruction::F32ConvertI32S,
        Opcode::F32ConvertI32U => Instruction::F32ConvertI32U,
        Opcode::F32ConvertI64S => Instruction::F32ConvertI64S,
        Opcode::F32ConvertI64U => Instruction::F32ConvertI64U,
        Opcode::F32DemoteF64 => Instruction::F32DemoteF64,
        Opcode::F64ConvertI32S => Instruction::F64ConvertI32S,
        Opcode::F64ConvertI32U => Instruction::F64ConvertI32U,
        Opcode::F64ConvertI64S => Instruction::F64ConvertI64S,
        Opcode::F64ConvertI64U => Instruction::F64ConvertI64U,
        Opcode::F64PromoteF32 => Instruction::F64PromoteF32,
        Opcode::I32ReinterpretF32 => Instruction::I32ReinterpretF32,
        Opcode::I64ReinterpretF64 => Instruction::I64ReinterpretF64,
        Opcode::F32ReinterpretI32 => Instruction::F32ReinterpretI32,
        Opcode::F64ReinterpretI64 => Instruction::F64ReinterpretI64,
    };
    Ok(inst)
}

fn read_memory_arg(reader: &mut SectionReader) -> Result<MemoryArg> {
    let arg = MemoryArg {
        align: reader.u32()?,
        offset: reader.u32()?,
    };
    Ok(arg)
}
