use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("integer divide by zero")]
    IntegerDivideByZero,
    #[error("integer overflow")]
    IntegerOverflow,
    #[error("integer overflow")]
    DivisionOverflow,
    #[error("cannot pop value from stack")]
    StackPopError,
    #[error("invalid count of memory, must be 1")]
    InvalidMemoryCountError,
}
