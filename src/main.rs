#![feature(buf_read_has_data_left)]
#![allow(dead_code)]
#![allow(unused)]

use anyhow::Result;
use anyhow::{bail, Context};
use clap::Parser;
use runtime::{Runtime, Value};
use section::*;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::{env, result};
use value::FuncType;

mod instruction;
mod runtime;
mod section;
mod value;

#[derive(Debug, Default)]
pub struct Module {
    magic: String,
    version: u32,
    type_section: Option<Vec<FuncType>>,
    function_section: Option<Vec<u32>>,
    code_section: Option<Vec<FunctionBody>>,
    export_section: Option<Vec<Export>>,
}

impl Module {
    pub fn add_section(&mut self, section: Section) {
        match section {
            Section::Type(section) => self.type_section = Some(section),
            Section::Function(section) => self.function_section = Some(section),
            Section::Code(section) => self.code_section = Some(section),
            Section::Export(section) => self.export_section = Some(section),
        };
    }
}

pub struct Decoder<R> {
    reader: BufReader<R>,
}

impl<R: io::Read> Decoder<R> {
    fn new(reader: R) -> Self {
        let reader = BufReader::new(reader);
        Self { reader }
    }

    fn byte(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn bytes(&mut self, num: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; num];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn decode_to_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.as_slice().try_into()?))
    }

    fn decode_to_string(&mut self, num: usize) -> Result<String> {
        let str = String::from_utf8_lossy(self.bytes(num)?.as_slice()).to_string();
        Ok(str)
    }

    pub fn decode_section_header(&mut self) -> Result<(SectionID, u32)> {
        let id: SectionID = self.byte()?.into();
        let size: u32 = self.byte()?.try_into()?;
        Ok((id, size))
    }

    pub fn decode_header(&mut self) -> Result<(String, u32)> {
        let magic = self.decode_to_string(4)?;
        if magic != "\0asm" {
            bail!("invalid binary magic")
        }

        let version = self.decode_to_u32()?;
        if version != 1 {
            bail!("invalid binary version")
        }
        Ok((magic, version))
    }

    pub fn decode(&mut self) -> Result<Module> {
        let (magic, version) = self.decode_header()?;
        let mut module = Module {
            magic,
            version,
            ..Module::default()
        };
        while self.reader.has_data_left()? {
            let (id, size) = self.decode_section_header()?;
            // TODO: decode custom section
            if id == SectionID::Custom {
                break;
            }
            let data = self.bytes(size as usize)?;
            let section = Section::decode(id, data)?;
            module.add_section(section);
        }
        Ok(module)
    }
}

#[derive(Debug, Parser)]
#[clap(author, about, version)]
struct Args {
    file: String,

    func: String,

    func_args: Vec<i32>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let file = fs::File::open(args.file)?;
    let mut decoder = Decoder::new(file);
    let mut module = decoder.decode()?;
    let mut runtime = Runtime::new(&mut module)?;
    let mut func_args = vec![];
    for arg in args.func_args.into_iter() {
        func_args.push(Value::from(arg));
    }
    let result = runtime.invoke(args.func, &mut func_args);
    println!("{}", result?.unwrap());
    Ok(())
}
