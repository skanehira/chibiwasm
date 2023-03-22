use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid count of memory, must be 1")]
    InvalidMemoryCountError,
}
