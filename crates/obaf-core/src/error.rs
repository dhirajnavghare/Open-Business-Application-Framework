use thiserror::Error;
use utoipa::ToSchema;

pub type Result<T> = std::result::Result<T, ObafError>;

#[derive(Debug, Error, ToSchema)]
pub enum ObafError {
    #[error("Resource not found: {message}")]
    NotFound { message: String },

    #[error("Validation failed: {message}")]
    Validation { message: String },

    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    #[error("Forbidden: {message}")]
    Forbidden { message: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Database error: {message}")]
    Database { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl ObafError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound {
            message: msg.into(),
        }
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation {
            message: msg.into(),
        }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: msg.into(),
        }
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden {
            message: msg.into(),
        }
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict {
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal {
            message: msg.into(),
        }
    }
}
