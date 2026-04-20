use modkit_macros::domain_model;

/// Domain-level errors for nodes registry
#[domain_model]
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Node not found: {0}")]
    NodeNotFound(uuid::Uuid),

    #[error("Failed to collect system information: {0}")]
    SysInfoCollectionFailed(String),

    #[error("Failed to collect system capabilities: {0}")]
    SysCapCollectionFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for DomainError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<modkit_node_info::NodeInfoError> for DomainError {
    fn from(e: modkit_node_info::NodeInfoError) -> Self {
        use modkit_node_info::NodeInfoError;
        match e {
            NodeInfoError::SysInfoCollectionFailed(msg) => Self::SysInfoCollectionFailed(msg),
            NodeInfoError::SysCapCollectionFailed(source) => {
                Self::SysCapCollectionFailed(source.to_string())
            }
            NodeInfoError::HardwareUuidFailed(msg) => {
                Self::Internal(format!("Hardware UUID failed: {msg}"))
            }
            NodeInfoError::Internal(source) => Self::Internal(source.to_string()),
            other => Self::Internal(other.to_string()),
        }
    }
}

impl From<DomainError> for nodes_registry_sdk::NodesRegistryError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::NodeNotFound(id) => Self::NodeNotFound(id),
            DomainError::SysInfoCollectionFailed(msg) => Self::SysInfoCollectionFailed(msg),
            DomainError::SysCapCollectionFailed(msg) => Self::SysCapCollectionFailed(msg),
            DomainError::InvalidInput(msg) => Self::Validation(msg),
            DomainError::Internal(msg) => Self::Internal(msg),
        }
    }
}
