# chibiwasm (WIP)
This repository was created for the purpose of learning how Wasm works.
Please do not use it in production.

## Usage
```sh
$ cargo run -- add.wasm 1 2
I32(2)
```

## Spec
Base on core 1.

https://www.w3.org/TR/wasm-core-1/

# tests
The list is base on https://github.com/WebAssembly/spec/tree/wg-1.0/test/core

NOTE: Passes only normal tests

- [ ] address.wast
- [ ] align.wast
- [ ] binary-leb128.wast
- [x] binary.wast
- [x] block.wast
- [ ] br.wast
- [x] br_if.wast
- [ ] br_table.wast
- [x] break_drop.wast
- [ ] call.wast
- [ ] call_indirect.wast
- [x] comments.wast
- [x] const.wast
- [ ] conversions.wast
- [ ] custom.wast
- [ ] data.wast
- [ ] elem.wast
- [ ] endianness.wast
- [ ] exports.wast
- [x] f32.wast
- [x] f32_bitwise.wast
- [x] f32_cmp.wast
- [x] f64.wast
- [x] f64_bitwise.wast
- [x] f64_cmp.wast
- [ ] fac.wast
- [ ] float_exprs.wast
- [ ] float_literals.wast
- [ ] float_memory.wast
- [ ] float_misc.wast
- [x] forward.wast
- [x] func.wast
- [ ] func_ptrs.wast
- [x] globals.wast
- [x] i32.wast
- [x] i64.wast
- [ ] if.wast
- [ ] imports.wast
- [x] inline_module.wast
- [ ] int_exprs.wast
- [x] int_literals.wast
- [ ] labels.wast
- [ ] left-to-right.wast
- [ ] linking.wast
- [x] load.wast
- [ ] local_get.wast
- [ ] local_set.wast
- [ ] local_tee.wast
- [ ] loop.wast
- [ ] memory.wast
- [ ] memory_grow.wast
- [ ] memory_redundancy.wast
- [ ] memory_size.wast
- [ ] memory_trap.wast
- [x] names.wast
- [x] nop.wast
- [ ] return.wast
- [ ] select.wast
- [ ] skip_stack_guard_page.wast
- [x] stack.wast
- [ ] start.wast
- [x] store.wast
- [ ] switch.wast
- [ ] token.wast
- [ ] traps.wast
- [ ] type.wast
- [ ] unreachable.wast
- [ ] unreached_invalid.wast
- [ ] unwind.wast
- [ ] utf8_custom_section_id.wast
- [ ] utf8_import_field.wast
- [ ] utf8_import_module.wast
- [ ] utf8_invalid_encoding.wast

## LICENSE
This software includes the work that is distributed in the Apache License 2.0.
