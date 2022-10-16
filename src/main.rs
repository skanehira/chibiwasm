#![allow(dead_code)]
#![allow(unused)]

use crate::value::Value;
use anyhow::Result;
use anyhow::{bail, Context};
use clap::Parser;
use module::Module;
use runtime::Runtime;
use section::*;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::{env, result};
use types::FuncType;

mod instruction;
mod module;
mod runtime;
mod section;
mod types;
mod value;

pub struct Decoder<R> {
    reader: BufReader<R>,
}

impl<R: io::Read> Decoder<R> {
    fn new(reader: R) -> Self {
        let reader = BufReader::new(reader);
        Self { reader }
    }

    fn is_end(&mut self) -> Result<bool> {
        Ok(self.reader.fill_buf().map(|b| !b.is_empty())?)
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

    fn u32(&mut self) -> Result<u32> {
        let num = leb128::read::unsigned(&mut self.reader)?;
        let num = u32::try_from(num)?;
        Ok(num)
    }

    pub fn decode_section_header(&mut self) -> Result<(SectionID, u32)> {
        let id: SectionID = self.byte()?.into();
        let size: u32 = self.u32()?;
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
        while self.is_end()? {
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
