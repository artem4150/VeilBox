use serde::Serialize;
use std::fmt::{Display, Formatter};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl AppError {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        details: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new("VALIDATION_ERROR", message, None)
    }

    pub fn state(message: impl Into<String>) -> Self {
        Self::new("STATE_ERROR", message, None)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", message, None)
    }

    pub fn process(message: impl Into<String>, details: Option<String>) -> Self {
        Self::new("PROCESS_ERROR", message, details)
    }

    pub fn storage(message: impl Into<String>, details: Option<String>) -> Self {
        Self::new("STORAGE_ERROR", message, details)
    }

    pub fn proxy(message: impl Into<String>, details: Option<String>) -> Self {
        Self::new("PROXY_ERROR", message, details)
    }

    pub fn internal(message: impl Into<String>, details: Option<String>) -> Self {
        Self::new("INTERNAL_ERROR", message, details)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::storage("I/O operation failed", Some(value.to_string()))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::storage("JSON serialization failed", Some(value.to_string()))
    }
}

impl From<url::ParseError> for AppError {
    fn from(value: url::ParseError) -> Self {
        Self::validation(format!("Invalid VLESS URI: {}", value))
    }
}

impl From<uuid::Error> for AppError {
    fn from(value: uuid::Error) -> Self {
        Self::validation(format!("Invalid UUID: {}", value))
    }
}

impl From<regex::Error> for AppError {
    fn from(value: regex::Error) -> Self {
        Self::internal("Regex initialization failed", Some(value.to_string()))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        Self::process("Network request failed", Some(value.to_string()))
    }
}
