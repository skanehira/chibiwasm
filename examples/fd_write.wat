(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32))
  )
  (memory (export "memory") 1)
  (data (i32.const 0) "Hello, World!\n")

  (func $write_hello_world (result i32)
    (local $iovec i32)

    (i32.store (i32.const 16) (i32.const 0))
    (i32.store (i32.const 20) (i32.const 7))
    (i32.store (i32.const 24) (i32.const 7))
    (i32.store (i32.const 28) (i32.const 7))

    (local.set $iovec (i32.const 16))

    (call $fd_write
      (i32.const 1)
      (local.get $iovec)
      (i32.const 2)
      (i32.const 28)
    )
  )
  (export "_start" (func $write_hello_world))
)
