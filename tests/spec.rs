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
        let result = match e.type_field.as_str() {
            "i32" => {
                let value = e.value.as_ref().unwrap().parse::<u32>().unwrap();
                Value::I32(value as i32)
            }
            "i64" => {
                let value = e.value.as_ref().unwrap().parse::<u64>().unwrap();
                Value::I64(value as i64)
            }
            "f32" => {
                let value = e.value.as_ref().unwrap().parse::<u32>().unwrap();
                Value::F32(f32::from_bits(value))
            }
            "f64" => {
                let value = e.value.as_ref().unwrap().parse::<u64>().unwrap();
                Value::F64(f64::from_bits(value))
            }
            _ => {
                panic!("unexpected type field")
            }
        };
        result
    }
}

#[cfg(test)]
mod tests {
    use anyhow::*;
    use chibiwasm::runtime::Runtime;
    use chibiwasm::{value::Value, *};
    use serde::Deserialize;
    use std::io::{Cursor, Read};
    use std::{fs, num::IntErrorKind, path::Path};
    use wabt::{script::*, Features};

    fn into_wasm_value(values: Vec<wabt::script::Value>) -> Vec<chibiwasm::value::Value> {
        values
            .into_iter()
            .map(|a| match a {
                wabt::script::Value::I32(v) => chibiwasm::value::Value::I32(v),
                wabt::script::Value::I64(v) => chibiwasm::value::Value::I64(v),
                wabt::script::Value::F32(v) => chibiwasm::value::Value::F32(v),
                wabt::script::Value::F64(v) => chibiwasm::value::Value::F64(v),
                wabt::script::Value::V128(_) => todo!(),
            })
            .collect()
    }

    fn run_test(spec_file: &str) -> Result<()> {
        println!("{} testing...", spec_file);
        let spec = Path::new("./tests/testsuite").join(spec_file);
        let mut file = fs::File::open(spec)?;
        let mut wast = String::new();
        file.read_to_string(&mut wast)?;

        let mut features = Features::new();
        features.enable_all();
        let mut parser = ScriptParser::<f32, f64>::from_source_and_name_with_features(
            wast.as_bytes(),
            spec_file,
            features,
        )?;

        let mut runtime = {
            if let Some(command) = parser.next()? {
                match command.kind {
                    CommandKind::Module { module, name } => {
                        let mut reader = Cursor::new(module.into_vec());
                        Runtime::from_reader(&mut reader)?
                    }
                    _ => panic!("not found module"),
                }
            } else {
                panic!("not found any command");
            }
        };

        while let Some(command) = parser.next()? {
            match command.kind {
                CommandKind::AssertReturn { action, expected } => match action {
                    Action::Invoke {
                        module,
                        field,
                        args,
                    } => {
                        let args = into_wasm_value(args);
                        let result = runtime.invoke(field, args)?;
                        if result.is_none() {
                            continue;
                        }
                        match result.unwrap() {
                            Value::I32(v) => {
                                assert_eq!(expected, vec![wabt::script::Value::I32(v)]);
                            }
                            Value::I64(v) => {
                                assert_eq!(expected, vec![wabt::script::Value::I64(v)]);
                            }
                            Value::F32(v) => {
                                assert_eq!(expected, vec![wabt::script::Value::F32(v)]);
                            }
                            Value::F64(v) => {
                                assert_eq!(expected, vec![wabt::script::Value::F64(v)]);
                            }
                        }
                    }
                    Action::Get { module, field } => todo!(),
                },
                CommandKind::AssertReturnCanonicalNan { action } => {
                    // TODO
                }
                CommandKind::AssertReturnArithmeticNan { action } => {
                    // TODO
                }
                CommandKind::AssertTrap { action, message } => match action {
                    Action::Invoke {
                        module,
                        field,
                        args,
                    } => {
                        let args = into_wasm_value(args);
                        let result = runtime.invoke(field.clone(), args);

                        match result {
                            Err(err) => {
                                assert_eq!(message, err.to_string());
                            }
                            _ => {
                                panic!("test must be fail: {}", field);
                            }
                        }
                    }
                    Action::Get { module, field } => todo!(),
                },
                CommandKind::AssertInvalid { module, message } => {
                    // TODO
                }
                CommandKind::AssertMalformed { module, message } => {
                    // TODO
                }
                CommandKind::AssertUninstantiable { module, message } => {
                    // TODO
                }
                CommandKind::AssertExhaustion { action, message } => {
                    // TODO
                }
                CommandKind::AssertUnlinkable { module, message } => {
                    // TODO
                }
                CommandKind::Register { name, as_name } => {
                    // TODO
                }
                CommandKind::PerformAction(_) => {
                    // TODO
                }
                _ => {
                    panic!("unexpect command kind: {:?}", command.kind);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn spec() -> Result<()> {
        let spec_files = vec!["i32.wast", "i64.wast", "f32.wast"];
        for file in spec_files {
            run_test(file)?;
        }

        Ok(())
    }
}
