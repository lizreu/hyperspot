//! Error types for the tenant resolver module.

use thiserror::Error;

use crate::TenantId;

/// Errors that can occur when using the tenant resolver API.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TenantResolverError {
    /// The requested target tenant was not found.
    #[error("tenant not found: {tenant_id}")]
    TenantNotFound {
        /// The tenant ID that was not found.
        tenant_id: TenantId,
    },

    /// The request is not authorized.
    ///
    /// Reserved for future plugins that implement access control.
    /// Built-in plugins currently use `TenantNotFound` for unauthorized access.
    #[error("unauthorized")]
    Unauthorized,

    /// No plugin is available to handle the request.
    #[error("no tenant resolver plugin available for vendor '{vendor}'")]
    NoPluginAvailable {
        /// Vendor for which no plugin instance could be resolved.
        vendor: String,
    },

    /// The plugin is not available yet.
    #[error("tenant resolver plugin '{gts_id}' unavailable: {reason}")]
    ServiceUnavailable {
        /// GTS identifier of the plugin instance.
        gts_id: String,
        /// Reason the plugin is unavailable.
        reason: String,
    },

    /// An internal error occurred.
    #[error("internal error: {0}")]
    Internal(String),
}
