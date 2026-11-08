use serde::{Deserialize, Serialize};
use std::fmt;

pub type CoreResult<T> = Result<T, EngineError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorDomain {
    Bluetooth,
    Audio,
    Input,
    Ime,
    Focus,
    Helper,
    Configuration,
    Permission,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineError {
    pub domain: ErrorDomain,
    pub code: String,
    pub retryable: bool,
}

impl EngineError {
    pub fn new(domain: ErrorDomain, code: impl Into<String>) -> Self {
        Self {
            domain,
            code: code.into(),
            retryable: false,
        }
    }

    pub fn retryable(domain: ErrorDomain, code: impl Into<String>) -> Self {
        Self {
            domain,
            code: code.into(),
            retryable: true,
        }
    }

    pub fn internal(code: impl Into<String>) -> Self {
        Self::new(ErrorDomain::Internal, code)
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}/{}", self.domain, self.code)
    }
}

impl std::error::Error for EngineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_keeps_stable_fields() {
        let error = EngineError::retryable(ErrorDomain::Bluetooth, "connect-timeout");
        assert_eq!(error.domain, ErrorDomain::Bluetooth);
        assert_eq!(error.code, "connect-timeout");
        assert!(error.retryable);
        assert_eq!(error.to_string(), "Bluetooth/connect-timeout");
    }
}
