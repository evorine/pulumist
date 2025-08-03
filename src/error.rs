use thiserror::Error;

#[derive(Error, Debug)]
pub enum PulumistError {
    #[error("FFI error: {0}")]
    Ffi(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Stack operation failed: {0}")]
    StackOperation(String),
    
    #[error("Provider error: {0}")]
    Provider(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, PulumistError>;