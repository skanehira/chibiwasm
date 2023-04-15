(module
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory 1)
  (data (i32.const 0) "Hello, world!\n")
  (func $main (export "main") (param i32 i32 i32 i32) (result i32)
    (call $fd_write (local.get 0) (local.get 1) (local.get 2) (local.get 3))
  )
)
