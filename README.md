# chibiwasm (WIP)
Implementation for understanding how Wasm Runtime works

## Spec
Implementation base on core 1.

https://www.w3.org/TR/wasm-core-1/

# tests
Base on https://github.com/WebAssembly/spec/tree/wg-1.0/test/core

- [ ] address.wast
- [ ] align.wast
- [ ] binary-leb128.wast
- [ ] binary.wast
- [ ] block.wast
- [ ] br.wast
- [ ] br_if.wast
- [ ] br_table.wast
- [ ] break-drop.wast
- [ ] call.wast
- [ ] call_indirect.wast
- [ ] comments.wast
- [ ] const.wast
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
- [ ] forward.wast
- [ ] func.wast
- [ ] func_ptrs.wast
- [ ] globals.wast
- [x] i32.wast
- [x] i64.wast
- [ ] if.wast
- [ ] imports.wast
- [ ] inline-module.wast
- [ ] int_exprs.wast
- [ ] int_literals.wast
- [ ] labels.wast
- [ ] left-to-right.wast
- [ ] linking.wast
- [ ] load.wast
- [ ] local_get.wast
- [ ] local_set.wast
- [ ] local_tee.wast
- [ ] loop.wast
- [ ] memory.wast
- [ ] memory_grow.wast
- [ ] memory_redundancy.wast
- [ ] memory_size.wast
- [ ] memory_trap.wast
- [ ] names.wast
- [ ] nop.wast
- [ ] return.wast
- [ ] select.wast
- [ ] skip-stack-guard-page.wast
- [ ] stack.wast
- [ ] start.wast
- [ ] store.wast
- [ ] switch.wast
- [ ] token.wast
- [ ] traps.wast
- [ ] type.wast
- [ ] unreachable.wast
- [ ] unreached-invalid.wast
- [ ] unwind.wast
- [ ] utf8-custom-section-id.wast
- [ ] utf8-import-field.wast
- [ ] utf8-import-module.wast
- [ ] utf8-invalid-encoding.wast
