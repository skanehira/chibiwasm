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
	(export "add" (func $add))
	(export "sub" (func $sub))
	(export "eq" (func $eq))
	)
