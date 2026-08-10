//! Shared result, error, event, and redaction primitives for `OpenWork`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The stable product name used by every renderer.
pub const PRODUCT_NAME: &str = "OpenWork";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidArguments,
    UnsupportedPlatform,
    PreflightFailed,
    RuntimeNotFound,
    RuntimeUnhealthy,
    InstallFailed,
    ConfigInvalid,
    Io,
    Internal,
}

impl ErrorCode {
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::InvalidArguments => 2,
            Self::UnsupportedPlatform => 10,
            Self::PreflightFailed => 11,
            Self::RuntimeNotFound => 20,
            Self::RuntimeUnhealthy => 21,
            Self::InstallFailed => 30,
            Self::ConfigInvalid => 40,
            Self::Io => 74,
            Self::Internal => 70,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OpenWorkError {
    pub code: ErrorCode,
    pub message: String,
    pub remediation: Option<String>,
}

impl OpenWorkError {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: redact_text(&message.into()),
            remediation: None,
        }
    }

    #[must_use]
    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(redact_text(&remediation.into()));
        self
    }

    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.code.exit_code()
    }
}

impl fmt::Display for OpenWorkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)?;
        if let Some(remediation) = &self.remediation {
            write!(formatter, "; remediation: {remediation}")?;
        }
        Ok(())
    }
}

impl std::error::Error for OpenWorkError {}

/// Redacts common credential assignments and token prefixes from diagnostic text.
#[must_use]
pub fn redact_text(input: &str) -> String {
    input
        .split_whitespace()
        .map(redact_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_word(word: &str) -> String {
    const SECRET_KEYS: [&str; 6] = [
        "TOKEN",
        "API_KEY",
        "PASSWORD",
        "SECRET",
        "AUTHORIZATION",
        "COOKIE",
    ];
    let upper = word.to_ascii_uppercase();
    if SECRET_KEYS
        .iter()
        .any(|key| upper.starts_with(&format!("{key}=")))
        || ["sk-", "ghp_", "gho_", "github_pat_"]
            .iter()
            .any(|prefix| word.starts_with(prefix))
    {
        "[REDACTED]".to_owned()
    } else {
        word.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_are_stable() {
        assert_eq!(ErrorCode::InvalidArguments.exit_code(), 2);
        assert_eq!(ErrorCode::UnsupportedPlatform.exit_code(), 10);
        assert_eq!(ErrorCode::RuntimeNotFound.exit_code(), 20);
        assert_eq!(ErrorCode::Internal.exit_code(), 70);
    }

    #[test]
    fn errors_redact_credentials() {
        let error = OpenWorkError::new(ErrorCode::RuntimeUnhealthy, "TOKEN=visible failed")
            .with_remediation("retry with sk-not-a-real-secret");
        assert_eq!(error.message, "[REDACTED] failed");
        assert!(!error.to_string().contains("not-a-real-secret"));
    }
}
