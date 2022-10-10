#![feature(buf_read_has_data_left)]
#![allow(dead_code)]
#![allow(unused)]

use anyhow::Result;
use anyhow::{bail, Context};
use section::*;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

mod instruction;
mod section;
mod value;

#[derive(Debug)]
pub struct Module {
    magic: String,
    version: u32,
    sections: Vec<Section>,
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

    pub fn decode_section(&mut self) -> Result<Vec<Section>> {
        let mut sections = vec![];
        while self.reader.has_data_left()? {
            let (id, size) = self.decode_section_header()?;
            let data = self.bytes(size as usize)?;
            let section = Section::decode(id, data)?;
            sections.push(section);
        }
        Ok(sections)
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
        let sections = self.decode_section()?;
        let module = Module {
            magic,
            version,
            sections,
        };
        Ok(module)
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let file = fs::File::open(args.get(1).context("Please specify a file name")?)?;
    let reader = BufReader::new(Box::new(file));
    let mut decoder = Decoder::new(reader);
    let module = decoder.decode()?;
    dbg!(module);
    Ok(())
}
