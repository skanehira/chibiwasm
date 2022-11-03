#![allow(dead_code)]
#![allow(unused)]

use anyhow::{bail, Context, Result};
use chibiwasm::{value::Value, *};
use serde::Deserialize;
use std::{fs, num::IntErrorKind, path::Path};

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    #[serde(rename = "source_filename")]
    pub source_filename: String,
    pub commands: Vec<Command>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    #[serde(rename = "type")]
    pub type_field: String,
    pub line: i64,
    pub filename: Option<String>,
    pub action: Option<Action>,
    #[serde(default)]
    pub expected: Vec<SpecValue>,
    pub text: Option<String>,
    #[serde(rename = "module_type")]
    pub module_type: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    #[serde(rename = "type")]
    pub type_field: String,
    pub field: String,
    pub args: Vec<SpecValue>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecValue {
    #[serde(rename = "type")]
    pub type_field: String,
    pub value: Option<String>,
}

impl From<&SpecValue> for Value {
    fn from(e: &SpecValue) -> Self {
        let bytes = e
            .value
            .as_ref()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            .to_le_bytes();

        let result = match e.type_field.as_str() {
            "i32" => {
                let src = bytes[..4].as_ref();
                let mut dst = [0u8; 4];
                dst.copy_from_slice(src);
                Value::I32(i32::from_le_bytes(dst))
            }
            "i64" => Value::I64(i64::from_le_bytes(bytes)),
            "f32" => {
                let src = bytes[..4].as_ref();
                let mut dst = [0u8; 4];
                dst.copy_from_slice(src);
                Value::F32(f32::from_le_bytes(dst))
            }
            "f64" => Value::F64(f64::from_le_bytes(bytes)),
            _ => {
                panic!("unexpected type field")
            }
        };
        result
    }
}

#[test]
fn spec() -> Result<()> {
    let spec_file = fs::File::open(Path::new("./tests/testsuite/i32.json"))?;
    let spec: Spec = serde_json::from_reader(spec_file)?;
    if spec.commands[0].type_field != "module" {
        bail!("first command is not module type");
    }

    // load wasm module
    let module_file = spec.commands[0]
        .filename
        .as_ref()
        .context("not module filename")?;
    let path = Path::join(Path::new("./tests/testsuite"), Path::new(&module_file));

    let file = fs::File::open(path)?;
    let mut decoder = module::Decoder::new(file);
    let mut module = decoder.decode()?;
    let mut runtime = runtime::Runtime::new(&mut module)?;

    for cmd in spec.commands.into_iter() {
        // TODO: 他のテストも動くようにする
        if cmd.type_field == "assert_return" {
            let action = cmd.action.context("not found action")?;
            let function_name = action.field;
            let mut args: Vec<value::Value> = action
                .args
                .into_iter()
                .map(|arg| -> value::Value { (&arg).into() })
                .collect();
            let result = runtime
                .invoke(function_name, &mut args)?
                .context("no result")?;
            let expected = &cmd.expected[0];

            let expect: Value = expected.into();
            assert_eq!(result, expect, "fail line: {}", cmd.line);
        }
    }

    let args = [1i32, 2i32].into_iter().map(Value::from);
    let result = runtime
        .invoke("add".into(), &mut args.into_iter().collect())?
        .context("no result")?;

    assert_eq!(result, Value::from(3));
    Ok(())
}
