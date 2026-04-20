//! Domain errors for the tenant resolver module.

use modkit_macros::domain_model;
use tenant_resolver_sdk::TenantResolverError;
use uuid::Uuid;

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

    #[error("tenant not found: {tenant_id}")]
    TenantNotFound { tenant_id: Uuid },

    /// Reserved for future plugins that implement access control.
    #[error("unauthorized")]
    Unauthorized,

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

impl From<TenantResolverError> for DomainError {
    fn from(e: TenantResolverError) -> Self {
        match e {
            TenantResolverError::TenantNotFound { tenant_id } => Self::TenantNotFound {
                tenant_id: tenant_id.0,
            },
            TenantResolverError::Unauthorized => Self::Unauthorized,
            TenantResolverError::NoPluginAvailable { vendor } => Self::PluginNotFound { vendor },
            TenantResolverError::ServiceUnavailable { gts_id, reason } => {
                Self::PluginUnavailable { gts_id, reason }
            }
            TenantResolverError::Internal(msg) => Self::Internal(msg.into()),
            // `TenantResolverError` is `#[non_exhaustive]`; future variants collapse to Internal.
            other => Self::Internal(Box::new(other)),
        }
    }
}

impl From<DomainError> for TenantResolverError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::PluginNotFound { vendor } => Self::NoPluginAvailable { vendor },
            DomainError::InvalidPluginInstance { gts_id, reason } => {
                Self::Internal(format!("invalid plugin instance '{gts_id}': {reason}"))
            }
            DomainError::PluginUnavailable { gts_id, reason } => {
                Self::ServiceUnavailable { gts_id, reason }
            }
            DomainError::TenantNotFound { tenant_id } => Self::TenantNotFound {
                tenant_id: tenant_resolver_sdk::TenantId(tenant_id),
            },
            DomainError::Unauthorized => Self::Unauthorized,
            DomainError::TypesRegistryUnavailable(src) | DomainError::Internal(src) => {
                Self::Internal(src.to_string())
            }
        }
    }
}
