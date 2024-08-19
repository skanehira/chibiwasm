#[cfg(test)]
mod importer {
    use anyhow::{bail, Context as _, Result};
    use chibiwasm::{
        module::{ExternalFuncInst, FuncInst, GlobalInst, InternalMemoryInst, InternalTableInst},
        ExternalVal, Importer, Runtime, Store, Value,
    };
    use std::{cell::RefCell, rc::Rc};

    #[derive(Clone)]
    pub struct Import((String, Rc<RefCell<Store>>));

    impl Import {
        pub fn new(name: String, store: Rc<RefCell<Store>>) -> Self {
            Self((name, store))
        }
    }

    impl Importer for Import {
        fn name(&self) -> &str {
            let (name, _) = &self.0;
            name.as_str()
        }

        fn get(&self, name: &str) -> Result<Option<Rc<RefCell<Store>>>> {
            if self.name() != name {
                return Ok(None);
            }
            let (_, store) = &self.0;
            Ok(Some(Rc::clone(store)))
        }

        fn invoke(
            &self,
            store: Rc<RefCell<Store>>,
            func: ExternalFuncInst,
            args: Vec<Value>,
        ) -> Result<Option<Value>> {
            let mut runtime = Runtime::instantiate(Rc::clone(&store))?;
            runtime.call(func.field, args)
        }

        fn resolve_table(
            &self,
            name: &str,
            field: &str,
        ) -> Result<Option<Rc<RefCell<InternalTableInst>>>> {
            let store = self.get(name)?;
            match store {
                Some(store) => {
                    let store = store.borrow();

                    let export_inst = store
                        .module
                        .exports
                        .get(field)
                        .context(format!("not found exported table '{field}' from {name}"))?;

                    let external_val = &export_inst.desc;
                    let ExternalVal::Table(idx) = external_val else {
                        bail!("invalid export desc: {:?}", external_val);
                    };

                    let table = store
                        .tables
                        .get(*idx as usize)
                        .with_context(|| format!("not found table {idx} in module: {name}"))?;

                    Ok(Some(Rc::clone(table)))
                }
                None => {
                    bail!("cannot resolve table. not found module: {name} in imports",);
                }
            }
        }

        fn resolve_global(&self, name: &str, field: &str) -> Result<Option<GlobalInst>> {
            let store = self.get(name)?;
            match store {
                Some(store) => {
                    let store = store.borrow();
                    let export_inst = store
                        .module
                        .exports
                        .get(field)
                        .context(format!("not found exported global '{field}' from {name}"))?;
                    let external_val = &export_inst.desc;

                    let ExternalVal::Global(idx) = external_val else {
                        bail!("invalid export desc: {:?}", external_val);
                    };
                    let global = store
                        .globals
                        .get(*idx as usize)
                        .with_context(|| format!("not found global index '{idx}' from {name}"))?;

                    Ok(Some(Rc::clone(global)))
                }
                None => {
                    bail!("cannot resolve global. not found module: {name} in imports",);
                }
            }
        }

        fn resolve_func(&self, name: &str, field: &str) -> Result<Option<FuncInst>> {
            let store = self.get(name)?;
            match store {
                Some(store) => {
                    let store = store.borrow();

                    let export_inst =
                        store.module.exports.get(field).context(format!(
                            "not found exported function '{field}' from {name}"
                        ))?;
                    let external_val = &export_inst.desc;

                    let ExternalVal::Func(idx) = external_val else {
                        bail!("invalid export desc: {:?}", external_val);
                    };
                    let func = store
                        .funcs
                        .get(*idx as usize)
                        .with_context(|| format!("not found function by {name}"))?;

                    Ok(Some(func.clone()))
                }
                None => {
                    bail!("cannot resolve function. not found module: {name} in imports",);
                }
            }
        }

        fn resolve_memory(
            &self,
            name: &str,
            field: &str,
        ) -> Result<Option<Rc<RefCell<InternalMemoryInst>>>> {
            let store = self.get(name)?;
            match store {
                Some(store) => {
                    let store = store.borrow();

                    let export_inst = store
                        .module
                        .exports
                        .get(field)
                        .context(format!("not found exported memory '{field}' from {name}"))?;
                    let external_val = &export_inst.desc;

                    let ExternalVal::Memory(idx) = external_val else {
                        bail!("invalid export desc: {:?}", external_val);
                    };
                    let memory = store
                        .memory
                        .get(*idx as usize)
                        .with_context(|| format!("not found memory from {name}"))?;

                    Ok(Some(Rc::clone(memory)))
                }
                None => {
                    bail!("cannot resolve memory. not found module: {name} in imports",);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::importer::Import;
    use anyhow::Result;
    use chibiwasm::execution::{Exports, Importer, Runtime, Store, Value};
    use log::debug;
    use paste::paste;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::{Cursor, Read};
    use std::rc::Rc;
    use std::sync::Once;
    use std::{fs, path::Path};
    use wabt::{script::*, Features};
    static INIT: Once = Once::new();

    #[derive(Default)]
    struct Spec {
        modules: HashMap<Option<String>, Rc<RefCell<Runtime>>>,
        imports: HashMap<String, Import>,
    }

    fn into_wasm_value(values: Vec<wabt::script::Value>) -> Vec<Value> {
        values
            .into_iter()
            .map(|a| match a {
                wabt::script::Value::I32(v) => Value::I32(v),
                wabt::script::Value::I64(v) => Value::I64(v),
                wabt::script::Value::F32(v) => Value::F32(v),
                wabt::script::Value::F64(v) => Value::F64(v),
                wabt::script::Value::V128(_) => todo!(),
            })
            .collect()
    }

    fn run_test(spec_file: &str) -> Result<()> {
        INIT.call_once(|| {
            // enable logger
            let _ = pretty_env_logger::env_logger::builder()
                .is_test(true)
                .try_init();
        });

        // add module for testing module importing
        let testspec = {
            let code = r#"
(module
  (table (export "table") 10 funcref)
  (global (export "global_i32") i32 (i32.const 42))
  (memory (export "memory") 1)
  
  (func $print (export "print")
    (nop)
  )
  (func $print_i32 (export "print_i32") (param i32)
    (nop)
  )
  (func $print_f32 (export "print_f32") (param f32)
    (nop)
  )
  (func $print_f64 (export "print_f64") (param f64)
    (nop)
  )
  (func $print_i32_f32 (export "print_i32_f32") (param i32 f32)
    (nop)
  )
  (func $print_f64_f64 (export "print_f64_f64") (param f64 f64)
    (nop)
  )
  (func $i64->i64 (export "i64->i64") (param i64) (result i64)
    return (local.get 0)
  )
)
                "#;
            let wasm = wat::parse_str(code).unwrap();
            let store = Store::from_bytes(wasm, None).unwrap();
            Rc::new(RefCell::new(store))
        };

        let mut imports = HashMap::new();
        let module_name = "spectest".to_string();
        imports.insert(module_name.clone(), Import::new(module_name, testspec));

        let spec = &mut Spec {
            modules: HashMap::new(),
            imports,
        };

        let mut file = fs::File::open(Path::new("./tests/testsuite").join(spec_file))?;
        let mut wast = String::new();
        file.read_to_string(&mut wast)?;

        let features = {
            let mut f = Features::new();
            f.enable_all();
            f
        };
        let mut parser = ScriptParser::<f32, f64>::from_source_and_name_with_features(
            wast.as_bytes(),
            spec_file,
            features,
        )?;

        fn assert_values(results: Vec<Value>, expected: Vec<wabt::script::Value>) -> Result<()> {
            let got: Vec<_> = results
                .into_iter()
                .map(|result| match result {
                    Value::I32(v) => wabt::script::Value::I32(v),
                    Value::I64(v) => wabt::script::Value::I64(v),
                    Value::F32(v) => {
                        if v.is_nan() {
                            wabt::script::Value::F32(0_f32)
                        } else {
                            wabt::script::Value::F32(v)
                        }
                    }
                    Value::F64(v) => {
                        if v.is_nan() {
                            wabt::script::Value::F64(0_f64)
                        } else {
                            wabt::script::Value::F64(v)
                        }
                    }
                })
                .collect();

            let want: Vec<_> = expected
                .into_iter()
                .map(|e| match e {
                    wabt::script::Value::F32(v) => {
                        if v.is_nan() {
                            return wabt::script::Value::F32(0_f32);
                        }
                        e
                    }
                    wabt::script::Value::F64(v) => {
                        if v.is_nan() {
                            return wabt::script::Value::F64(0_f64);
                        }
                        e
                    }
                    _ => e,
                })
                .collect();
            assert_eq!(want, got, "unexpected result, want={want:?}, got={got:?}");
            Ok(())
        }

        fn invoke(
            runtime: &mut Runtime,
            field: String,
            args: Vec<wabt::script::Value>,
            expected: Vec<wabt::script::Value>,
        ) -> Result<()> {
            let args = into_wasm_value(args);
            let result = runtime.call(field, args)?;
            match result {
                Some(result) => assert_values(vec![result], expected),
                None => Ok(()),
            }
        }

        while let Some(command) = parser.next()? {
            match command.kind {
                CommandKind::AssertReturn { action, expected } => match action {
                    Action::Invoke {
                        field,
                        args,
                        module,
                    } => {
                        debug!(
                            "invoke module: {:?}, func: {}, args: {:#?}",
                            &module, &field, &args
                        );
                        let runtime = spec.modules.get(&module).expect("not found mdoule").clone();
                        let runtime = &mut *runtime.borrow_mut();
                        invoke(runtime, field, args, expected)?;
                    }
                    Action::Get { module, field } => {
                        debug!("get module: {:?}, field: {}", &module, &field);
                        let runtime = spec.modules.get(&module).expect("not found mdoule").clone();
                        let runtime = &mut *runtime.borrow_mut();
                        let exports = runtime.exports(field.clone())?;

                        let results = match exports {
                            Exports::Global(global) => vec![global.borrow().value.clone()],
                            _ => {
                                todo!();
                            }
                        };

                        _ = assert_values(results, expected);
                    }
                },
                CommandKind::PerformAction(action) => match action {
                    Action::Invoke {
                        field,
                        args,
                        module,
                    } => {
                        debug!(
                            "invoke module: {:?}, func: {}, args: {:#?}",
                            &module, &field, &args
                        );
                        let runtime = spec.modules.get(&module).expect("not found mdoule").clone();
                        let runtime = &mut *runtime.borrow_mut();
                        invoke(runtime, field, args, vec![])?;
                    }
                    Action::Get { .. } => todo!(),
                },
                CommandKind::AssertReturnCanonicalNan { .. } => {
                    // TODO
                }
                CommandKind::AssertReturnArithmeticNan { .. } => {
                    // TODO
                }
                CommandKind::AssertTrap { action, message } => match action {
                    Action::Invoke {
                        field,
                        args,
                        module,
                    } => {
                        debug!(
                            "invoke module: {:?}, func: {}, args: {:#?}",
                            &module, &field, &args
                        );
                        let runtime = spec.modules.get(&module).expect("not found mdoule").clone();
                        let runtime = &mut *runtime.borrow_mut();
                        let args = into_wasm_value(args);
                        let result = runtime.call(field.clone(), args.clone());

                        match result {
                            Err(err) => {
                                let want = message;
                                let got = err.to_string();
                                assert_eq!(
                                    want,
                                    got,
                                    "unexpected result, want={want}, got={got}, test: {field}, args: {args:?}",
                                );
                            }
                            _ => {
                                panic!("test must be fail: {}", field);
                            }
                        }
                    }
                    Action::Get { .. } => todo!(),
                },
                CommandKind::AssertInvalid { .. } => {
                    // TODO
                }
                CommandKind::AssertMalformed { .. } => {
                    // TODO
                }
                CommandKind::AssertUninstantiable { .. } => {
                    // TODO
                }
                CommandKind::AssertExhaustion { .. } => {
                    // TODO
                }
                CommandKind::AssertUnlinkable { .. } => {
                    // TODO
                }
                CommandKind::Register { name, as_name } => {
                    let runtime = spec.modules.get(&name).expect("not found mdoule").clone();
                    let store = &runtime.borrow().store;
                    spec.imports
                        .insert(as_name.clone(), Import::new(as_name, Rc::clone(store)));
                }
                CommandKind::Module { module, name } => {
                    let mut reader = Cursor::new(module.into_vec());

                    let mut imports: Vec<Box<dyn Importer>> = vec![];
                    for (_, import) in spec.imports.iter() {
                        imports.push(Box::new(import.clone()));
                    }

                    let runtime = Runtime::from_reader(&mut reader, Some(imports))?;
                    let runtime = Rc::new(RefCell::new(runtime));
                    spec.modules.insert(name, runtime.clone());
                    spec.modules.insert(None, runtime);
                }
            }
        }
        Ok(())
    }

    macro_rules! test {
        ($ty: ident) => {
            paste! {
                #[test]
                fn [<test_ $ty>]() -> Result<()> {
                    let file = format!("{}.wast", stringify!($ty));
                    run_test(&file)?;
                    Ok(())
                }
            }
        };
    }

    test!(i32);
    test!(i64);
    test!(f32);
    test!(f32_cmp);
    test!(f32_bitwise);
    test!(f64);
    test!(f64_cmp);
    test!(f64_bitwise);
    test!(load);
    test!(nop);
    test!(store);
    test!(loop);
    test!(int_literals);
    test!(if);
    test!(br_if);
    test!(globals);
    test!(func);
    test!(block);
    test!(comments);
    test!(binary);
    test!(break_drop);
    test!(const);
    test!(forward);
    test!(inline_module);
    test!(names);
    test!(stack);
    test!(return);
    test!(br);
    test!(br_table);
    test!(local_set);
    test!(local_get);
    test!(local_tee);
    test!(select);
    test!(labels);
    test!(unreachable);
    test!(type);
    test!(fac);
    test!(memory_size);
    test!(address);
    test!(memory_trap);
    test!(align);
    test!(memory);
    test!(float_misc);
    test!(int_exprs);
    test!(memory_grow);
    test!(memory_redundancy);
    test!(call);
    test!(call_indirect);
    test!(float_memory);
    test!(float_exprs);
    test!(left_to_right);
    test!(skip_stack_guard_page);
    test!(unwind);
    test!(binary_leb128);
    test!(exports);
    test!(switch);
    test!(custom);
    test!(start);
    test!(imports);
    test!(func_ptrs);
    test!(elem);
    test!(data);
    test!(float_literals);
    test!(endianness);
    test!(conversions);
    test!(traps);
    test!(linking);

    // Skip invalid tests
    //test!(token);
    //test!(unreached_invalid);
    //test!(utf8_custom_section_id);
    //test!(utf8_import_field);
    //test!(utf8_import_module);
    //test!(utf8_invalid_encoding);
}
