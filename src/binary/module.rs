use super::{section::*, types::*};
use anyhow::{bail, Result};
use num_traits::FromPrimitive;
use std::io;
use std::{
    io::{BufRead, BufReader, Read},
    u8,
};

#[derive(Debug, Default, PartialEq)]
pub struct Module {
    pub magic: String,
    pub version: u32,
    pub custom_section: Option<Custom>,
    pub type_section: Option<Vec<FuncType>>,
    pub import_section: Option<Vec<Import>>,
    pub function_section: Option<Vec<u32>>,
    pub table_section: Option<Vec<Table>>,
    pub memory_section: Option<Vec<Memory>>,
    pub global_section: Option<Vec<Global>>,
    pub export_section: Option<Vec<Export>>,
    pub start_section: Option<u32>,
    pub element_section: Option<Vec<Element>>,
    pub data: Option<Vec<Data>>,
    pub code_section: Option<Vec<FunctionBody>>,
}

impl Module {
    pub fn add_section(&mut self, section: Section) {
        match section {
            Section::Custom(section) => self.custom_section = Some(section),
            Section::Type(section) => self.type_section = Some(section),
            Section::Import(section) => self.import_section = Some(section),
            Section::Function(section) => self.function_section = Some(section),
            Section::Table(section) => self.table_section = Some(section),
            Section::Memory(section) => self.memory_section = Some(section),
            Section::Global(section) => self.global_section = Some(section),
            Section::Export(section) => self.export_section = Some(section),
            Section::Code(section) => self.code_section = Some(section),
            Section::Element(section) => self.element_section = Some(section),
            Section::Data(section) => self.data = Some(section),
            Section::Start(section) => self.start_section = Some(section),
        };
    }
}

pub struct Decoder<R> {
    reader: BufReader<R>,
}

impl<R: io::Read> Decoder<R> {
    pub fn new(reader: R) -> Self {
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
        let id: SectionID = FromPrimitive::from_u8(self.byte()?).unwrap();
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
            let bytes = self.bytes(size as usize)?;
            let section = decode(id, &bytes)?;
            module.add_section(section);
        }
        Ok(module)
    }
}

#[cfg(test)]
mod test {
    use super::Decoder;
    use anyhow::Result;
    use insta::assert_debug_snapshot;
    use wasmer::wat2wasm;

    #[test]
    fn test_decode_module() -> Result<()> {
        let source = r#"
(module
  ;; import section
  (import "test" "print_i32" (func $print_i32 (param i32)))
  (import "test" "memory-2-inf" (table 10 funcref))
  (import "test" "global-i32" (global i32))

  ;; memory section
  (memory 1 256)

  ;; table section
  (table 1 256 funcref)

  ;; global section
  (global $a i32 (i32.const -2))
  (global $x (mut f32) (f32.const 5.5))

  ;; function section
  (func (export "test") (param i32)
    (i32.add
      (local.get 0)
      (i32.const 1)
    )
    (drop)
  )
  (func (export "test2") (param i32) (param i32) (result i32)
    (i32.add
      (local.get 0)
      (local.get 1)
    )
  )
  (func $main (call $print_i32 (i32.const 2)))

  ;; data section
  (data (memory 0x0) (i32.const 1) "a" "" "bcd")

  ;; element_section
  (elem (i32.const 0) $main)

  ;; start section
  (start $main)
)
            "#;
        let wasm = wat2wasm(source.as_bytes())?;

        let reader = std::io::Cursor::new(wasm);
        let mut decoder = Decoder::new(reader);
        let module = decoder.decode()?;

        assert_debug_snapshot!(module);

        Ok(())
    }

    #[test]
    fn test_nested_if() -> Result<()> {
        let source = r#"
(module
  ;; Auxiliary definition
  (memory 1)

  (func $dummy)
  (func (export "nested") (param i32 i32) (result i32)
    (if (result i32) (local.get 0)
      (then
        (if (local.get 1) (then (call $dummy) (block) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (block) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 9))
          (else (call $dummy) (i32.const 10))
        )
      )
      (else
        (if (local.get 1) (then (call $dummy) (block) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (block) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 10))
          (else (call $dummy) (i32.const 11))
        )
      )
    )
  )
)
            "#;
        let wasm = wat2wasm(source.as_bytes())?;

        let reader = std::io::Cursor::new(wasm);
        let mut decoder = Decoder::new(reader);
        let module = decoder.decode()?;

        assert_debug_snapshot!(module);

        Ok(())
    }

    #[test]
    fn test_return() -> Result<()> {
        let source = r#"
(module
  (func (export "type-i32-value") (result i32)
    (block (result i32) (i32.ctz (return (i32.const 1))))
  )
)
            "#;
        let wasm = wat2wasm(source.as_bytes())?;

        let reader = std::io::Cursor::new(wasm);
        let mut decoder = Decoder::new(reader);
        let module = decoder.decode()?;

        assert_debug_snapshot!(module);

        Ok(())
    }
}
