#![feature(buf_read_has_data_left)]
#![allow(dead_code)]
#![allow(unused)]

use anyhow::Result;
use anyhow::{bail, Context};
use runtime::{Runtime, Value};
use section::*;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
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

pub struct Decoder {
    reader: BufReader<Box<File>>,
}

impl Decoder {
    fn new(reader: BufReader<Box<File>>) -> Self {
        Self { reader }
    }

    fn byte(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        if buf[0] == 0x00 {
            bail!("end of file")
        }
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
        let mut module = Module::default();
        while self.reader.has_data_left()? {
            let (id, size) = self.decode_section_header()?;
            let data = self.bytes(size as usize)?;
            let section = Section::decode(id, data)?;
            module.add_section(section);
        }
        Ok(module)
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let file = fs::File::open(args.get(1).context("Please specify a file name")?)?;
    let reader = BufReader::new(Box::new(file));
    let mut decoder = Decoder::new(reader);
    let mut module = decoder.decode()?;
    let mut runtime = Runtime::new(&mut module)?;
    let mut args = vec![Value::from(10), Value::from(5)];
    let result = runtime.invoke("add".into(), &mut args);
    println!("{}", result?.unwrap());
    Ok(())
}
