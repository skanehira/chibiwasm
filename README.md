# chibiwasm
This repository was created for the purpose of learning how Wasm works.
Please do not use it in production.

If you want to learn how Wasm runtime works, I recommend this book :)

https://skanehira.github.io/writing-a-wasm-runtime-in-rust/

## Usage
```sh
$ cat
(module
  (func $add (export "add") (param i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.add)
  )
)
$ wat2wasm add.wat
$ cargo run -- add.wasm add 1 2
    Finished dev [unoptimized + debuginfo] target(s) in 0.09s
     Running `target/debug/chibiwasm add.wasm add 1 2`
3
```

## Use as a crate

```rust
use chibiwasm::{Runtime, Value};

fn main() -> anyhow::Result<()> {
    let mut runtime = Runtime::from_file("examples/fib.wasm", None)?;
    if let Some(output) = runtime.call("fib".into(), vec![Value::I32(10)])? {
        println!("output: {}", output);
    }
    Ok(())
}
```

```sh
$ cargo run -q --example fib
output: 89
```

## Test
```sh
$ cargo make test
```

## Spec
Base on core 1.

https://www.w3.org/TR/wasm-core-1/

# tests
The list is base on https://github.com/WebAssembly/spec/tree/wg-1.0/test/core

NOTE: Checking only the test cases that have passed both assert_return and assert_trap.

- [x] address.wast
- [x] align.wast
- [x] binary_leb128.wast
- [x] binary.wast
- [x] block.wast
- [x] br.wast
- [x] br_if.wast
- [x] br_table.wast
- [x] break_drop.wast
- [x] call.wast
- [x] call_indirect.wast
- [x] comments.wast
- [x] const.wast
- [x] conversions.wast
- [x] custom.wast
- [x] data.wast
- [x] elem.wast
- [x] endianness.wast
- [x] exports.wast
- [x] f32.wast
- [x] f32_bitwise.wast
- [x] f32_cmp.wast
- [x] f64.wast
- [x] f64_bitwise.wast
- [x] f64_cmp.wast
- [x] fac.wast
- [x] float_exprs.wast
- [x] float_literals.wast
- [x] float_memory.wast
- [x] float_misc.wast
- [x] forward.wast
- [x] func.wast
- [x] func_ptrs.wast
- [x] globals.wast
- [x] i32.wast
- [x] i64.wast
- [x] if.wast
- [x] imports.wast
- [x] inline_module.wast
- [x] int_exprs.wast
- [x] int_literals.wast
- [x] labels.wast
- [x] left_to_right.wast
- [x] linking.wast
- [x] load.wast
- [x] local_get.wast
- [x] local_set.wast
- [x] local_tee.wast
- [x] loop.wast
- [x] memory.wast
- [x] memory_grow.wast
- [x] memory_redundancy.wast
- [x] memory_size.wast
- [x] memory_trap.wast
- [x] names.wast
- [x] nop.wast
- [x] return.wast
- [x] select.wast
- [x] skip_stack_guard_page.wast
- [x] stack.wast
- [x] start.wast
- [x] store.wast
- [x] switch.wast
- [ ] ~~token.wast~~
- [x] traps.wast
- [x] type.wast
- [x] unreachable.wast
- [ ] ~~unreached_invalid.wast~~
- [x] unwind.wast
- [ ] ~~utf8_custom_section_id.wast~~
- [ ] ~~utf8_import_field.wast~~
- [ ] ~~utf8_import_module.wast~~
- [ ] ~~utf8_invalid_encoding.wast~~

## LICENSE
This software includes the work that is distributed in the Apache License 2.0.
