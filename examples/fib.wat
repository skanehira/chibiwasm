(module
  (func $fib (export "fib") (param i32) (result i32)
    (if (result i32) (i32.le_u (local.get 0) (i32.const 1))
      (then (i32.const 1))
      (else
        (i32.add
          (call $fib (i32.sub (local.get 0) (i32.const 2)))
          (call $fib (i32.sub (local.get 0) (i32.const 1)))
        )
      )
    )
  )
)
