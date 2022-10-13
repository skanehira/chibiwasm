(module
	(func $add (param $lhs i32) (param $rhs i32) (result i32)
				local.get $lhs
				local.get $rhs
				i32.add)
	(func $sub (param $a i32) (param $b i32) (result i32)
				local.get $a
				local.get $b
				i32.sub)
	(func $eq (param $a i32) (param $b i32) (result i32)
				local.get $a
				local.get $b
				i32.eq)
	(func $call_add (param $a i32) (param $b i32) (result i32)
				local.get $a
				local.get $b
				call $add)
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
	(export "add" (func $add))
	(export "sub" (func $sub))
	(export "call_add" (func $call_add))
	(export "eq" (func $eq))
	(export "const_i32" (func $const_i32))
	(export "return_value" (func $return_value))
	(export "test_if" (func $test_if))
	)
