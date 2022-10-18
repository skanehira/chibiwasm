(module
  (func $i32.add (param $lhs i32) (param $rhs i32) (result i32)
    local.get $lhs
    local.get $rhs
    i32.add
	)
  (func $i32.sub (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.sub
	)
  (func $i32.mul (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.mul
  )
  (func $i32.div_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.div_u
  )
  (func $i32.div_s (param $a i32) (param $b i32) (result i32)
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
  (func $call (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    call $i32.add
	)
  (func $i32.const (result i32)
    i32.const 1
    i32.const 1
    i32.add
  )
  (func $return (result i32)
    (return (i32.const 15))
  )
  (func $if (param $a i32) (param $b i32) (result i32)
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
  (export "i32.add" (func $i32.add))
  (export "i32.sub" (func $i32.sub))
  (export "i32.mul" (func $i32.mul))
  (export "i32.div_u" (func $i32.div_u))
  (export "i32.div_s" (func $i32.div_s))
  (export "i32.eq" (func $i32.eq))
  (export "i32.ne" (func $i32.ne))
  (export "i32.const" (func $i32.const))
  (export "call" (func $call))
  (export "return" (func $return))
  (export "if" (func $if))
  (export "fib" (func $fib))
  (export "if_else" (func $if_else))
)
