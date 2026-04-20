use thiserror::Error;

/// Errors that can occur during JWT claims validation and processing
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ClaimsError {
    #[error("Invalid signature or key")]
    InvalidSignature,

    #[error("Invalid issuer: expected one of {expected:?}, got {actual}")]
    InvalidIssuer {
        expected: Vec<String>,
        actual: String,
    },

    #[error("Invalid audience: expected one of {expected:?}, got {actual:?}")]
    InvalidAudience {
        expected: Vec<String>,
        actual: Vec<String>,
    },

    #[error("Token expired")]
    Expired,

    #[error("Token not yet valid (nbf check failed)")]
    NotYetValid,

    #[error("Malformed claims: {0}")]
    Malformed(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Missing required claim: {0}")]
    MissingClaim(String),

    #[error("Invalid claim format: {field} - {reason}")]
    InvalidClaimFormat { field: String, reason: String },

    #[error("Unknown key ID after refresh")]
    UnknownKidAfterRefresh,

    #[error("JWT decode failed: {0}")]
    DecodeFailed(String),

    #[error("JWKS fetch failed: {0}")]
    JwksFetchFailed(String),

    #[error("Unknown key ID: {0}")]
    UnknownKeyId(String),
}

// Conversion from ClaimsError to AuthError for backward compatibility.
//
// Every known `ClaimsError` variant maps to a specific `AuthError` variant so
// that authentication failures produced by the JWT pipeline surface with the
// correct HTTP semantics (typically 401) instead of being funneled into the
// generic `Internal` (500) fault.
//
// Note: the match is intentionally exhaustive with no wildcard arm. Because
// `ClaimsError` and this impl live in the same crate, `#[non_exhaustive]` does
// not force a fallback here; adding a new variant will be a compile error,
// which is exactly what we want - every new variant must be explicitly mapped
// to the most appropriate `AuthError`.
impl From<ClaimsError> for crate::errors::AuthError {
    fn from(err: ClaimsError) -> Self {
        use crate::errors::AuthError;
        match err {
            ClaimsError::Expired => AuthError::TokenExpired,
            ClaimsError::InvalidSignature => AuthError::InvalidToken("Invalid signature".into()),
            ClaimsError::InvalidIssuer { expected, actual } => AuthError::IssuerMismatch {
                expected: expected.join(", "),
                actual,
            },
            ClaimsError::InvalidAudience { expected, actual } => {
                AuthError::AudienceMismatch { expected, actual }
            }
            ClaimsError::NotYetValid => {
                AuthError::ValidationFailed("token not yet valid (nbf check failed)".into())
            }
            ClaimsError::Malformed(msg) => AuthError::InvalidToken(format!("malformed: {msg}")),
            ClaimsError::Provider(msg) => {
                AuthError::ValidationFailed(format!("provider error: {msg}"))
            }
            ClaimsError::MissingClaim(name) => {
                AuthError::ValidationFailed(format!("missing required claim: {name}"))
            }
            ClaimsError::InvalidClaimFormat { field, reason } => {
                AuthError::ValidationFailed(format!("invalid claim format: {field} - {reason}"))
            }
            ClaimsError::UnknownKidAfterRefresh => {
                AuthError::ValidationFailed("unknown key ID after refresh".into())
            }
            ClaimsError::DecodeFailed(msg) => {
                AuthError::InvalidToken(format!("decode failed: {msg}"))
            }
            ClaimsError::JwksFetchFailed(msg) => AuthError::JwksFetchFailed(msg),
            ClaimsError::UnknownKeyId(kid) => {
                AuthError::ValidationFailed(format!("unknown key ID: {kid}"))
            }
        }
    }
}
