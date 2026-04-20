// Updated: 2026-04-07 by Constructor Tech
//! Domain errors for the credstore module.

use credstore_sdk::CredStoreError;
use modkit_macros::domain_model;

/// Internal domain errors.
#[domain_model]
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum DomainError {
    #[error("types registry unavailable")]
    TypesRegistryUnavailable(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("no plugin instances found for vendor '{vendor}'")]
    PluginNotFound { vendor: String },

    #[error("invalid plugin instance content for '{gts_id}': {reason}")]
    InvalidPluginInstance { gts_id: String, reason: String },

    #[error("plugin not available for '{gts_id}': {reason}")]
    PluginUnavailable { gts_id: String, reason: String },

    #[error("secret not found")]
    NotFound,

    #[error("internal error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<types_registry_sdk::TypesRegistryError> for DomainError {
    fn from(e: types_registry_sdk::TypesRegistryError) -> Self {
        Self::TypesRegistryUnavailable(Box::new(e))
    }
}

impl From<modkit::client_hub::ClientHubError> for DomainError {
    fn from(e: modkit::client_hub::ClientHubError) -> Self {
        Self::Internal(Box::new(e))
    }
}

impl From<serde_json::Error> for DomainError {
    fn from(e: serde_json::Error) -> Self {
        Self::Internal(Box::new(e))
    }
}

impl From<modkit::plugins::ChoosePluginError> for DomainError {
    fn from(e: modkit::plugins::ChoosePluginError) -> Self {
        match e {
            modkit::plugins::ChoosePluginError::InvalidPluginInstance { gts_id, reason } => {
                Self::InvalidPluginInstance { gts_id, reason }
            }
            modkit::plugins::ChoosePluginError::PluginNotFound { vendor, .. } => {
                Self::PluginNotFound { vendor }
            }
            other => Self::Internal(Box::new(other)),
        }
    }
}

impl From<CredStoreError> for DomainError {
    fn from(e: CredStoreError) -> Self {
        match e {
            CredStoreError::NotFound => Self::NotFound,
            CredStoreError::NoPluginAvailable { vendor } => Self::PluginNotFound { vendor },
            CredStoreError::ServiceUnavailable { gts_id, reason } => {
                Self::PluginUnavailable { gts_id, reason }
            }
            CredStoreError::InvalidSecretRef { reason } => Self::Internal(reason.into()),
            CredStoreError::Internal(msg) => Self::Internal(msg.into()),
            // `CredStoreError` is `#[non_exhaustive]`; future variants collapse to Internal.
            other => Self::Internal(Box::new(other)),
        }
    }
}

impl From<DomainError> for CredStoreError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::PluginNotFound { vendor } => Self::NoPluginAvailable { vendor },
            DomainError::InvalidPluginInstance { gts_id, reason } => {
                Self::Internal(format!("invalid plugin instance '{gts_id}': {reason}"))
            }
            DomainError::PluginUnavailable { gts_id, reason } => {
                Self::ServiceUnavailable { gts_id, reason }
            }
            DomainError::NotFound => Self::NotFound,
            DomainError::TypesRegistryUnavailable(src) | DomainError::Internal(src) => {
                Self::Internal(src.to_string())
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "error_tests.rs"]
mod error_tests;
