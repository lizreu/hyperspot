/// Errors for node information collection
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum NodeInfoError {
    #[error("System information collection failed: {0}")]
    SysInfoCollectionFailed(String),

    #[error("System capabilities collection failed")]
    SysCapCollectionFailed(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Failed to get hardware UUID: {0}")]
    HardwareUuidFailed(String),

    #[error("Internal error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<anyhow::Error> for NodeInfoError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(e.into())
    }
}
