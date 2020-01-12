#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),

    #[error("Pointer overflow")]
    PointerOverflow,
}

#[derive(Debug, thiserror::Error)]
pub enum VMError {
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),

    #[error("Compile: {0}")]
    Compile(#[from] crate::bfir::CompileError),

    #[error("Runtime: {0}")]
    Runtime(#[from] RuntimeError),
}

pub type Result<T> = std::result::Result<T, VMError>;
