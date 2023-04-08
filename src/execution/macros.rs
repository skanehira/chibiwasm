#[macro_export]
macro_rules! load {
    ($runtime: expr, $ty: ty, $arg: expr) => {{
        let addr = $runtime.stack.pop1::<i32>()? as usize;
        let value = $runtime.store.memory.load::<$ty>(addr, $arg)?;
        $runtime.stack.push(value.into());
    }};
    ($runtime: expr, $ty: ty, $arg: expr, $tz: ty) => {{
        let addr = $runtime.stack.pop1::<i32>()? as usize;
        let value = $runtime.store.memory.load::<$ty>(addr, $arg)? as $tz;
        $runtime.stack.push(value.into());
    }};
}

#[macro_export]
macro_rules! store {
    ($runtime: expr, $ty: ty, $arg: expr) => {{
        let value = $runtime.stack.pop1::<$ty>()?;
        let addr = $runtime.stack.pop1::<i32>()? as usize;
        $runtime.store.memory.write(addr, $arg, value)?;
    }};
    ($runtime: expr, $ty: ty, $arg: expr, $tz: ty) => {{
        let value = $runtime.stack.pop1::<$ty>()? as $tz;
        let addr = $runtime.stack.pop1::<i32>()? as usize;
        $runtime.store.memory.write(addr, $arg, value)?;
    }};
}
