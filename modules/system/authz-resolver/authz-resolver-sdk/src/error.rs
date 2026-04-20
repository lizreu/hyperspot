//! Error types for the `AuthZ` resolver module.

use thiserror::Error;

/// Errors that can occur when using the `AuthZ` resolver API.
///
/// These represent infrastructure/transport failures only.
/// Access denial is expressed via `EvaluationResponse.decision == false`,
/// not as an error variant.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AuthZResolverError {
    /// No `AuthZ` plugin is available to handle the request.
    #[error("no authz plugin available for vendor '{vendor}'")]
    NoPluginAvailable {
        /// Vendor for which no plugin instance could be resolved.
        vendor: String,
    },

    /// The plugin is not available yet.
    #[error("authz plugin '{gts_id}' unavailable: {reason}")]
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
