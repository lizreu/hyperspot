// Updated: 2026-04-20 by Constructor Tech
use thiserror::Error;

/// Errors that can occur during credential store operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CredStoreError {
    #[error("invalid secret reference: {reason}")]
    InvalidSecretRef { reason: String },

    #[error("secret not found")]
    NotFound,

    #[error("no credential plugin available for vendor '{vendor}'")]
    NoPluginAvailable { vendor: String },

    #[error("credential plugin '{gts_id}' unavailable: {reason}")]
    ServiceUnavailable { gts_id: String, reason: String },

    #[error("internal error: {0}")]
    Internal(String),
}

impl CredStoreError {
    #[must_use]
    pub fn invalid_ref(reason: impl Into<String>) -> Self {
        Self::InvalidSecretRef {
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn service_unavailable(gts_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            gts_id: gts_id.into(),
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "error_tests.rs"]
mod error_tests;
