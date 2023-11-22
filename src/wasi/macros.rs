#[macro_export]
macro_rules! memory_load {
    ($memory: ident, $addr: expr, $align: expr, $offset: expr) => {{
        $memory.load(
            $addr,
            &MemoryArg {
                align: $align,
                offset: $offset as u32,
            },
        )?
    }};
}

#[macro_export]
macro_rules! memory_write {
    ($memory: ident, $addr: expr, $align: expr, $offset: expr, $size: expr) => {{
        $memory.write(
            $addr,
            &MemoryArg {
                align: $align,
                offset: $offset as u32,
            },
            $size as i32,
        )?;
    }};
}
