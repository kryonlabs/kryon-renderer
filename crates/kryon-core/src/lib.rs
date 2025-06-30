// crates/kryon-core/src/lib.rs
pub mod krb;
pub mod elements;
pub mod properties;
pub mod resources;
pub mod events;

pub use elements::*;
pub use properties::*;
pub use krb::*;
pub use resources::*;
pub use events::*;

#[derive(Debug, thiserror::Error)]
pub enum KryonError {
    #[error("Invalid KRB file: {0}")]
    InvalidKRB(String),
    
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u16),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Missing section: {0}")]
    MissingSection(String),
    
    #[error("Invalid element type: {0}")]
    InvalidElementType(u8),
    
    #[error("Component not found: {0}")]
    ComponentNotFound(String),
}

pub type Result<T> = std::result::Result<T, KryonError>;