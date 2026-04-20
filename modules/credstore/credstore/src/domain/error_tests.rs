// Created: 2026-04-07 by Constructor Tech
use modkit::plugins::ChoosePluginError;

use super::*;

// ── From<TypesRegistryError> ─────────────────────────────────────────────

#[test]
fn from_types_registry_error_becomes_types_registry_unavailable() {
    let src = types_registry_sdk::TypesRegistryError::internal("oops");
    let dst = DomainError::from(src);
    match &dst {
        DomainError::TypesRegistryUnavailable(source) => {
            assert!(
                source.to_string().contains("oops"),
                "expected source to preserve 'oops', got: {source}"
            );
        }
        _ => panic!("expected TypesRegistryUnavailable, got: {dst:?}"),
    }
}

// ── From<ClientHubError> ─────────────────────────────────────────────────

#[test]
fn from_client_hub_error_becomes_internal() {
    // Trigger a real ClientHubError by requesting an unregistered type.
    let hub = modkit::client_hub::ClientHub::default();
    let src = hub
        .get::<dyn types_registry_sdk::TypesRegistryClient>()
        .err()
        .unwrap();
    let dst = DomainError::from(src);
    assert!(matches!(dst, DomainError::Internal(_)));
}

// ── From<serde_json::Error> ──────────────────────────────────────────────

#[test]
fn from_serde_json_error_becomes_internal() {
    let src: serde_json::Error = serde_json::from_str::<i32>("not-json").unwrap_err();
    let dst = DomainError::from(src);
    assert!(matches!(dst, DomainError::Internal(_)));
}

// ── From<ChoosePluginError> ──────────────────────────────────────────────

#[test]
fn from_choose_plugin_error_not_found_becomes_plugin_not_found() {
    let src = ChoosePluginError::PluginNotFound {
        schema_id: "gts.x.core.test.plugin.v1~".into(),
        vendor: "acme".into(),
    };
    let dst = DomainError::from(src);
    assert!(matches!(dst, DomainError::PluginNotFound { vendor } if vendor == "acme"));
}

#[test]
fn from_choose_plugin_error_invalid_instance_becomes_invalid_plugin_instance() {
    let src = ChoosePluginError::InvalidPluginInstance {
        gts_id: "gts.x.core.test.error.v1~".into(),
        reason: "bad content".into(),
    };
    let dst = DomainError::from(src);
    assert!(
        matches!(dst, DomainError::InvalidPluginInstance { gts_id, reason }
            if gts_id == "gts.x.core.test.error.v1~" && reason == "bad content")
    );
}

// ── From<CredStoreError> for DomainError ─────────────────────────────────

#[test]
fn from_credstore_error_not_found_becomes_not_found() {
    let dst = DomainError::from(CredStoreError::NotFound);
    assert!(matches!(dst, DomainError::NotFound));
}

#[test]
fn from_credstore_error_no_plugin_available_becomes_plugin_not_found() {
    let dst = DomainError::from(CredStoreError::NoPluginAvailable {
        vendor: "acme".into(),
    });
    assert!(matches!(dst, DomainError::PluginNotFound { vendor } if vendor == "acme"));
}

#[test]
fn from_credstore_error_service_unavailable_becomes_plugin_unavailable() {
    let dst = DomainError::from(CredStoreError::ServiceUnavailable {
        gts_id: "gts.x.core.creds.plugin.acme.v1~".into(),
        reason: "down".into(),
    });
    assert!(
        matches!(dst, DomainError::PluginUnavailable { gts_id, reason }
        if gts_id == "gts.x.core.creds.plugin.acme.v1~" && reason == "down")
    );
}

#[test]
fn from_credstore_error_invalid_secret_ref_becomes_internal() {
    let dst = DomainError::from(CredStoreError::InvalidSecretRef {
        reason: "bad".into(),
    });
    match &dst {
        DomainError::Internal(src) => assert_eq!(src.to_string(), "bad"),
        _ => panic!("expected Internal, got {dst:?}"),
    }
}

#[test]
fn from_credstore_error_internal_becomes_internal() {
    let dst = DomainError::from(CredStoreError::Internal("boom".into()));
    match &dst {
        DomainError::Internal(src) => assert_eq!(src.to_string(), "boom"),
        _ => panic!("expected Internal, got {dst:?}"),
    }
}

// ── From<DomainError> for CredStoreError ────────────────────────────────

#[test]
fn domain_plugin_not_found_becomes_no_plugin_available() {
    let src = DomainError::PluginNotFound {
        vendor: "acme".into(),
    };
    let dst = CredStoreError::from(src);
    assert!(matches!(dst, CredStoreError::NoPluginAvailable { vendor } if vendor == "acme"));
}

#[test]
fn domain_invalid_plugin_instance_becomes_internal() {
    let src = DomainError::InvalidPluginInstance {
        gts_id: "gts.x.core.test.error.v1~".into(),
        reason: "bad".into(),
    };
    let dst = CredStoreError::from(src);
    assert!(
        matches!(dst, CredStoreError::Internal(ref msg)
            if msg.contains("gts.x.core.test.error.v1~") && msg.contains("bad")),
        "expected Internal with gts_id and reason, got: {dst:?}"
    );
}

#[test]
fn domain_plugin_unavailable_becomes_service_unavailable() {
    let src = DomainError::PluginUnavailable {
        gts_id: "gts.x.core.test.error.v1~".into(),
        reason: "not ready".into(),
    };
    let dst = CredStoreError::from(src);
    assert!(
        matches!(dst, CredStoreError::ServiceUnavailable { ref gts_id, ref reason }
            if gts_id == "gts.x.core.test.error.v1~" && reason == "not ready"),
        "expected ServiceUnavailable with gts_id and reason, got: {dst:?}"
    );
}

#[test]
fn roundtrip_credstore_to_domain_preserves_vendor() {
    let src = CredStoreError::NoPluginAvailable {
        vendor: "acme".into(),
    };
    let domain = DomainError::from(src);
    let back = CredStoreError::from(domain);
    assert!(
        matches!(back, CredStoreError::NoPluginAvailable { vendor } if vendor == "acme"),
        "vendor must survive round-trip without 'unknown' fallback"
    );
}

#[test]
fn roundtrip_credstore_to_domain_preserves_gts_id_and_reason() {
    let src = CredStoreError::ServiceUnavailable {
        gts_id: "gts.x.core.creds.plugin.acme.v1~".into(),
        reason: "down".into(),
    };
    let domain = DomainError::from(src);
    let back = CredStoreError::from(domain);
    assert!(
        matches!(back, CredStoreError::ServiceUnavailable { gts_id, reason }
            if gts_id == "gts.x.core.creds.plugin.acme.v1~" && reason == "down"),
        "gts_id and reason must survive round-trip without 'unknown' fallback"
    );
}

#[test]
fn domain_not_found_becomes_not_found() {
    let dst = CredStoreError::from(DomainError::NotFound);
    assert!(matches!(dst, CredStoreError::NotFound));
}

#[test]
fn domain_types_registry_unavailable_becomes_internal() {
    let src = DomainError::TypesRegistryUnavailable(Box::new(std::io::Error::other("gone")));
    let dst = CredStoreError::from(src);
    assert!(matches!(dst, CredStoreError::Internal(msg) if msg == "gone"));
}

#[test]
fn domain_internal_becomes_internal() {
    let src = DomainError::Internal(Box::new(std::io::Error::other("err")));
    let dst = CredStoreError::from(src);
    assert!(matches!(dst, CredStoreError::Internal(msg) if msg == "err"));
}
