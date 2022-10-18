(module
  (func $add (param $lhs i32) (param $rhs i32) (result i32)
    local.get $lhs
    local.get $rhs
    i32.add
	)
  (func $sub (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.sub
	)
  (func $mul (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.mul
  )
  (func $div_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.div_u
  )
  (func $div_s (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.div_s
  )
  (func $i32.eq (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.eq
	)
  (func $i32.ne (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.ne
	)
  (func $call_add (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    call $add
	)
  (func $const_i32 (result i32)
    i32.const 1
    i32.const 1
    i32.add
  )
  (func $return_value (result i32)
    (return (i32.const 15))
  )
  (func $test_if (param $a i32) (param $b i32) (result i32)
    (if
      (i32.eq (local.get $a) (local.get $b))
      (then (return (i32.const 1)))
    )
    (return (i32.const 0))
  )
  (func $fib (param $N i32) (result i32)
    (if
      (i32.eq (local.get $N) (i32.const 1))
      (then (return (i32.const 1)))
    )
    (if
      (i32.eq (local.get $N) (i32.const 2))
      (then (return (i32.const 1)))
    )
    (i32.add (call $fib
      (i32.sub (local.get $N) (i32.const 1)))
      (call $fib (i32.sub (local.get $N) (i32.const 2)))
    )
  )
  (func $if_else (param $a i32) (result i32)
    (if (i32.eq (local.get $a) (i32.const 1))
      (then (return (i32.const 1)))
      (else (return (i32.const 0)))
    )
    (return (i32.const -1))
  )
  (export "add" (func $add))
  (export "sub" (func $sub))
  (export "mul" (func $mul))
  (export "div_u" (func $div_u))
  (export "div_s" (func $div_s))
  (export "call_add" (func $call_add))
  (export "i32.eq" (func $i32.eq))
  (export "i32.ne" (func $i32.ne))
  (export "const_i32" (func $const_i32))
  (export "return_value" (func $return_value))
  (export "test_if" (func $test_if))
  (export "fib" (func $fib))
  (export "if_else" (func $if_else))
)
