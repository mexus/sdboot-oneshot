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
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for EfiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use efivar::Error;
        match &self.0 {
            Error::VarParseError
            | Error::InvalidVarName { .. }
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
