use std::fmt;

/// A thin wrapper around [efivar::Error] to provide [std::error::Error]
/// implementation.
pub struct EfiError(pub efivar::Error);

impl fmt::Debug for EfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for EfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use efivar::Error;
        match &self.0 {
            Error::InvalidVarName { name } => write!(f, "Invalid variable name: '{}'", name),
            Error::VarNotFound { name } => write!(f, "Variable '{}' not found", name),
            Error::PermissionDenied { name } => {
                write!(f, "Permission denied while accessing variable '{}'", name)
            }
            Error::VarUnknownError { name, error: _ } => {
                write!(f, "Unknown error while accessing variable '{}'", name)
            }
            Error::UnknownIoError { error: _ } => write!(f, "Unknown I/O error"),
            Error::UnknownFlag { flag } => write!(f, "Unknown flag '{}'", flag),
            Error::InvalidUTF8 => write!(f, "Invalid UTF8"),
            Error::BufferTooSmall { name } => {
                write!(f, "Buffer is too small (variable '{}')", name)
            }
            Error::UuidError { error: _ } => write!(f, "UUID error"),
        }
    }
}

impl std::error::Error for EfiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use efivar::Error;
        match &self.0 {
            Error::InvalidVarName { .. }
            | Error::VarNotFound { .. }
            | Error::PermissionDenied { .. }
            | Error::UnknownFlag { .. }
            | Error::InvalidUTF8
            | Error::BufferTooSmall { .. } => None,
            Error::VarUnknownError { name: _, error } | Error::UnknownIoError { error } => {
                Some(error)
            }
            Error::UuidError { error } => Some(error),
        }
    }
}
