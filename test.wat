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
	(func $get_i32 (result i32)
				i32.const 1
				i32.const 1
				i32.add
				)
	(export "add" (func $add))
	(export "sub" (func $sub))
	(export "call_add" (func $call_add))
	(export "eq" (func $eq))
	(export "get_i32" (func $get_i32))
	)
