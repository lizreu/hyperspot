//! Error types for the `AuthN` resolver module.

use thiserror::Error;

/// Errors that can occur when using the `AuthN` resolver API.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AuthNResolverError {
    /// The token is invalid, expired, or malformed.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// No `AuthN` plugin is available to handle the request.
    #[error("no authn plugin available for vendor '{vendor}'")]
    NoPluginAvailable {
        /// Vendor for which no plugin instance could be resolved.
        vendor: String,
    },

    /// The plugin is not available yet.
    #[error("authn plugin '{gts_id}' unavailable: {reason}")]
    ServiceUnavailable {
        /// GTS identifier of the plugin instance.
        gts_id: String,
        /// Reason the plugin is unavailable.
        reason: String,
    },

    /// Client credentials exchange failed (e.g., invalid credentials,
    /// `IdP` unreachable, token endpoint error).
    #[error("token acquisition failed: {0}")]
    TokenAcquisitionFailed(String),

    /// An internal error occurred.
    #[error("internal error: {0}")]
    Internal(String),
}
