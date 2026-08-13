//! Deckwatch plugin — provisions a Fluent Bit log-shipping sidecar via the Infrastructure tab.
//!
//! ## WASM entry points
//!
//! - [`metadata`] — declares the "log-shipper" provisionable resource; called once at load time.
//! - [`provision`] — injects a Fluent Bit sidecar when an operator adds the resource on the
//!   Application Infrastructure tab.
//! - [`deprovision`] — called when the resource is removed; returns a message explaining
//!   the reconcile behaviour.
//!
//! ## Architecture
//!
//! Association is managed at the application level via the Infrastructure tab —
//! no deployment annotation required. Deckwatch calls `provision()` to persist state
//! and sidecars, then injects them into all application deployments on the next
//! reconcile cycle.

#[cfg(target_arch = "wasm32")]
use deckwatch_plugin_sdk::PluginMetadata;
use deckwatch_plugin_sdk::{
    ConfigField, ConfigFieldType, EnvVarSpec, PluginResource, ResourceDeprovisionRequest,
    ResourceDeprovisionResult, ResourceProvisionRequest, ResourceProvisionResult, SidecarSpec,
};
#[cfg(target_arch = "wasm32")]
use extism_pdk::*;

const DEFAULT_IMAGE: &str = "fluent/fluent-bit:3.1";

// ── Resource declaration ───────────────────────────────────────────────────────

/// Declares the "log-shipper" provisionable resource shown on the Infrastructure tab.
///
/// Not `#[cfg(target_arch = "wasm32")]`-gated so it can be called in host-target
/// unit tests.
pub fn resource_decl() -> PluginResource {
    PluginResource {
        id: "log-shipper".into(),
        label: "Fluent Bit".into(),
        icon: "mdi-transfer".into(),
        description: "Provision a Fluent Bit log-shipping sidecar for this application.".into(),
        singleton: true,
        fields: vec![
            ConfigField {
                key: "FLUENT_BIT_IMAGE".into(),
                label: "Fluent Bit Image".into(),
                description: "Fluent Bit container image to inject as a sidecar.".into(),
                field_type: ConfigFieldType::String,
                default: Some(DEFAULT_IMAGE.into()),
                required: false,
                options: vec![],
                env_source: None,
            },
            ConfigField {
                key: "FLUENT_BIT_OUTPUT".into(),
                label: "Output Endpoint".into(),
                description: "Log output destination URL (e.g. an Elasticsearch endpoint). Injected as FLUENT_BIT_OUTPUT into the sidecar when non-empty.".into(),
                field_type: ConfigFieldType::String,
                default: None,
                required: false,
                options: vec![],
                env_source: None,
            },
        ],
        output_keys: vec![],
    }
}

// ── Pure core: provision ───────────────────────────────────────────────────────

/// Provision a Fluent Bit log-shipping sidecar for an application.
///
/// Returns a [`ResourceProvisionResult`] with:
/// - `sidecars`: the `fluent-bit` container; `FLUENT_BIT_OUTPUT` set as a sidecar
///   env var when the field is non-empty
/// - `deployment_annotations`: stamps `deckwatch.io/fluent-bit: "provisioned"`
///
/// Testable on the native target without extism.
pub fn provision_inner(req: &ResourceProvisionRequest) -> ResourceProvisionResult {
    let mut result = ResourceProvisionResult::default();

    let image = req
        .fields
        .get("FLUENT_BIT_IMAGE")
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| DEFAULT_IMAGE.to_string());

    let output_endpoint = req
        .fields
        .get("FLUENT_BIT_OUTPUT")
        .cloned()
        .unwrap_or_default();

    let mut sidecar_env: Vec<EnvVarSpec> = vec![];
    if !output_endpoint.is_empty() {
        sidecar_env.push(EnvVarSpec::value("FLUENT_BIT_OUTPUT", &output_endpoint));
    }

    result
        .deployment_annotations
        .insert("deckwatch.io/fluent-bit".into(), "provisioned".into());

    result.sidecars.push(SidecarSpec {
        name: "fluent-bit".into(),
        image,
        env: sidecar_env,
        port: None,
        cpu: Some("50m".into()),
        memory: Some("64Mi".into()),
    });

    result
}

// ── Pure core: deprovision ─────────────────────────────────────────────────────

/// Deprovision — Fluent Bit is stateless; the deckwatch record is removed and the
/// sidecar disappears from deployments on the next reconcile via SSA.
pub fn deprovision_inner(_req: &ResourceDeprovisionRequest) -> ResourceDeprovisionResult {
    ResourceDeprovisionResult {
        errors: vec![],
        message: "Fluent Bit log-shipping sidecar will be removed from all application deployments on the next reconcile.".into(),
    }
}

// ── WASM entry points ──────────────────────────────────────────────────────────

/// Return static plugin metadata including the "log-shipper" resource declaration.
/// Called once by deckwatch at load time.
#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn metadata() -> FnResult<Json<PluginMetadata>> {
    Ok(Json(PluginMetadata {
        name: "fluent-bit".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description:
            "Provisions a Fluent Bit log-shipping sidecar via the Application Infrastructure tab"
                .into(),
        provides: vec!["sidecar:fluent-bit".into()],
        depends_on: vec![],
        optional_depends_on: vec![],
        config_schema: vec![ConfigField {
            key: "FLUENT_BIT_IMAGE".into(),
            label: "Fluent Bit Image".into(),
            description: "Default Fluent Bit container image (can be overridden per-resource)."
                .into(),
            field_type: ConfigFieldType::String,
            default: Some(DEFAULT_IMAGE.into()),
            required: false,
            options: vec![],
            env_source: None,
        }],
        resources: vec![resource_decl()],
    }))
}

/// Provision a Fluent Bit sidecar. Called when an operator clicks "Add Fluent Bit"
/// and submits the provisioning form on the Application Infrastructure tab.
#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn provision(
    Json(req): Json<ResourceProvisionRequest>,
) -> FnResult<Json<ResourceProvisionResult>> {
    Ok(Json(provision_inner(&req)))
}

/// Deprovision the Fluent Bit resource. The sidecar is removed from all deployments
/// on the next reconcile cycle via server-side apply.
#[cfg(target_arch = "wasm32")]
#[plugin_fn]
pub fn deprovision(
    Json(req): Json<ResourceDeprovisionRequest>,
) -> FnResult<Json<ResourceDeprovisionResult>> {
    Ok(Json(deprovision_inner(&req)))
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn provision_req(fields: &[(&str, &str)]) -> ResourceProvisionRequest {
        ResourceProvisionRequest {
            application_name: "my-app".into(),
            namespace: "production".into(),
            resource_id: "log-shipper".into(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn deprovision_req() -> ResourceDeprovisionRequest {
        ResourceDeprovisionRequest {
            application_name: "my-app".into(),
            namespace: "production".into(),
            resource_id: "log-shipper".into(),
            state: HashMap::new(),
            fields: HashMap::new(),
        }
    }

    #[test]
    fn resource_decl_has_expected_id_and_fields() {
        let r = resource_decl();
        assert_eq!(r.id, "log-shipper");
        assert_eq!(r.label, "Fluent Bit");
        assert!(r.singleton, "fluent-bit must be a singleton resource");
        assert!(
            r.fields.iter().any(|f| f.key == "FLUENT_BIT_IMAGE"),
            "FLUENT_BIT_IMAGE field must be declared"
        );
        assert!(
            r.fields.iter().any(|f| f.key == "FLUENT_BIT_OUTPUT"),
            "FLUENT_BIT_OUTPUT field must be declared"
        );
    }

    #[test]
    fn provision_defaults_inject_sidecar_with_no_output_env() {
        let result = provision_inner(&provision_req(&[]));
        assert!(
            result.errors.is_empty(),
            "no errors expected: {:?}",
            result.errors
        );
        assert_eq!(result.sidecars.len(), 1);
        let sc = &result.sidecars[0];
        assert_eq!(sc.name, "fluent-bit");
        assert_eq!(sc.image, DEFAULT_IMAGE);
        assert!(sc.port.is_none(), "fluent-bit has no fixed port");
        assert!(
            sc.env.is_empty(),
            "no sidecar env when FLUENT_BIT_OUTPUT is not set"
        );
    }

    #[test]
    fn provision_output_endpoint_set_on_sidecar_env() {
        let result = provision_inner(&provision_req(&[(
            "FLUENT_BIT_OUTPUT",
            "http://es.example.com:9200",
        )]));
        let sidecar_env = &result.sidecars[0].env;
        assert!(
            sidecar_env
                .iter()
                .any(|e| e.name == "FLUENT_BIT_OUTPUT" && e.value == "http://es.example.com:9200"),
            "FLUENT_BIT_OUTPUT must be set in sidecar env"
        );
    }

    #[test]
    fn provision_empty_output_not_injected() {
        let result = provision_inner(&provision_req(&[("FLUENT_BIT_OUTPUT", "")]));
        assert!(
            result.sidecars[0].env.is_empty(),
            "empty output must not inject sidecar env var"
        );
    }

    #[test]
    fn provision_custom_image_is_used() {
        let result = provision_inner(&provision_req(&[(
            "FLUENT_BIT_IMAGE",
            "fluent/fluent-bit:3.2",
        )]));
        assert_eq!(result.sidecars[0].image, "fluent/fluent-bit:3.2");
    }

    #[test]
    fn provision_stamps_annotation() {
        let result = provision_inner(&provision_req(&[]));
        assert_eq!(
            result
                .deployment_annotations
                .get("deckwatch.io/fluent-bit")
                .map(String::as_str),
            Some("provisioned")
        );
    }

    #[test]
    fn provision_injects_no_application_state() {
        let result = provision_inner(&provision_req(&[]));
        assert!(
            result.state.is_empty(),
            "fluent-bit should not inject application-level env vars"
        );
    }

    #[test]
    fn deprovision_returns_non_empty_message() {
        let result = deprovision_inner(&deprovision_req());
        assert!(result.errors.is_empty());
        assert!(
            !result.message.is_empty(),
            "deprovision must return a user-visible message"
        );
    }
}
