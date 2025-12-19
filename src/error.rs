//! Centralized error handling for Ferrite
//!
//! This module provides a unified error type that covers all error scenarios
//! in the application: file I/O, configuration, and markdown parsing.

use log::warn;
use std::fmt;
use std::io;
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Custom Result Type Alias
// ─────────────────────────────────────────────────────────────────────────────

/// A specialized `Result` type for the application.
pub type Result<T> = std::result::Result<T, Error>;

/// The centralized error type for the application.
#[derive(Debug)]
pub enum Error {
    // ─────────────────────────────────────────────────────────────────────────
    // File I/O Errors
    // ─────────────────────────────────────────────────────────────────────────
    /// Generic I/O error wrapper
    Io(io::Error),

    /// Failed to write file contents
    FileWrite { path: PathBuf, source: io::Error },

    // ─────────────────────────────────────────────────────────────────────────
    // Configuration Errors
    // ─────────────────────────────────────────────────────────────────────────
    /// Failed to load configuration file
    ConfigLoad {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to save configuration file
    ConfigSave {
        path: PathBuf,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failed to parse configuration (invalid JSON/format)
    ConfigParse {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration directory not found or inaccessible
    ConfigDirNotFound,

    // ─────────────────────────────────────────────────────────────────────────
    // Application Errors
    // ─────────────────────────────────────────────────────────────────────────
    /// Generic application error with a message
    Application(String),
}

// Implement From traits for convenient error conversion
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::ConfigParse {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Display trait implementation for user-friendly error messages
// ─────────────────────────────────────────────────────────────────────────────
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // File I/O Errors
            Error::Io(err) => write!(f, "I/O error: {}", err),
            Error::FileWrite { path, source } => {
                write!(f, "Failed to write '{}': {}", path.display(), source)
            }

            // Configuration Errors
            Error::ConfigLoad { path, source } => {
                write!(
                    f,
                    "Failed to load configuration from '{}': {}",
                    path.display(),
                    source
                )
            }
            Error::ConfigSave { path, source } => {
                write!(
                    f,
                    "Failed to save configuration to '{}': {}",
                    path.display(),
                    source
                )
            }
            Error::ConfigParse { message, .. } => {
                write!(f, "Invalid configuration format: {}", message)
            }
            Error::ConfigDirNotFound => {
                write!(f, "Configuration directory not found")
            }

            // Application Errors
            Error::Application(msg) => write!(f, "{}", msg),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// std::error::Error trait implementation for error chaining
// ─────────────────────────────────────────────────────────────────────────────
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::FileWrite { source, .. } => Some(source),
            Error::ConfigLoad { source, .. } => Some(source.as_ref()),
            Error::ConfigSave { source, .. } => Some(source.as_ref()),
            Error::ConfigParse { source, .. } => source
                .as_ref()
                .map(|s| s.as_ref() as &(dyn std::error::Error + 'static)),
            Error::ConfigDirNotFound | Error::Application(_) => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Graceful Degradation Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Extension trait for Result to support graceful degradation.
pub trait ResultExt<T> {
    /// If the result is an error, log it at warning level and return the provided default.
    fn unwrap_or_warn_default(self, default: T, context: &str) -> T;
}

impl<T> ResultExt<T> for Result<T> {
    fn unwrap_or_warn_default(self, default: T, context: &str) -> T {
        match self {
            Ok(value) => value,
            Err(err) => {
                warn!("{}: {}. Using default.", context, err);
                default
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_creation() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "test error");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn test_file_write_error() {
        let path = PathBuf::from("/test/file.md");
        let io_err = io::Error::new(io::ErrorKind::Other, "write failed");
        let err = Error::FileWrite {
            path: path.clone(),
            source: io_err,
        };
        assert!(matches!(err, Error::FileWrite { path: p, .. } if p == path));
    }

    #[test]
    fn test_application_error() {
        let err = Error::Application("something went wrong".to_string());
        assert!(matches!(err, Error::Application(msg) if msg == "something went wrong"));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let json_result: std::result::Result<String, _> = serde_json::from_str("invalid json");
        let err = Error::from(json_result.unwrap_err());
        assert!(matches!(err, Error::ConfigParse { .. }));
    }

    #[test]
    fn test_display_io_error() {
        let io_err = io::Error::new(io::ErrorKind::Other, "disk full");
        let err = Error::Io(io_err);
        let msg = format!("{}", err);
        assert!(msg.contains("I/O error"));
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn test_display_config_dir_not_found() {
        let err = Error::ConfigDirNotFound;
        let msg = format!("{}", err);
        assert_eq!(msg, "Configuration directory not found");
    }

    #[test]
    fn test_error_source_io() {
        use std::error::Error as StdError;
        let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
        let err = Error::Io(io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn test_error_source_none_for_simple_variants() {
        use std::error::Error as StdError;
        let err = Error::Application("test".to_string());
        assert!(err.source().is_none());

        let err = Error::ConfigDirNotFound;
        assert!(err.source().is_none());
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> super::Result<i32> {
            Ok(42)
        }

        fn returns_err() -> super::Result<i32> {
            Err(Error::Application("test".to_string()))
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_unwrap_or_warn_default_ok() {
        use super::ResultExt;
        let result: super::Result<i32> = Ok(42);
        let value = result.unwrap_or_warn_default(0, "test context");
        assert_eq!(value, 42);
    }

    #[test]
    fn test_unwrap_or_warn_default_err() {
        use super::ResultExt;
        let result: super::Result<i32> = Err(Error::Application("test".to_string()));
        let value = result.unwrap_or_warn_default(0, "test context");
        assert_eq!(value, 0);
    }
}
