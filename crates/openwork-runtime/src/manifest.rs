use crate::{RuntimeCapabilities, RuntimeMetadata, RuntimeResult};
use openwork_core::{ErrorCode, OpenWorkError};
use serde::{Deserialize, Serialize};

pub const RUNTIME_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationPolicy {
    OfficialChecksumRequired,
    ObservedChecksum,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstallerSource {
    pub os: String,
    pub architecture: String,
    pub environment: String,
    pub url: String,
    pub verification: VerificationPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeManifest {
    pub schema_version: u32,
    pub metadata: RuntimeMetadata,
    pub capabilities: RuntimeCapabilities,
    pub installer_sources: Vec<InstallerSource>,
}

/// Parses and validates the stable invariants of a runtime manifest.
///
/// # Errors
///
/// Returns `config_invalid` when JSON is malformed, the schema version is not
/// supported, identity is empty, or an installer source is not HTTPS.
pub fn parse_manifest_json(input: &str) -> RuntimeResult<RuntimeManifest> {
    let manifest: RuntimeManifest = serde_json::from_str(input).map_err(|error| {
        OpenWorkError::new(
            ErrorCode::ConfigInvalid,
            format!("runtime manifest JSON is invalid: {error}"),
        )
        .with_remediation("Validate the manifest against runtime-manifest.v1.schema.json.")
    })?;
    if manifest.schema_version != RUNTIME_MANIFEST_SCHEMA_VERSION {
        return Err(OpenWorkError::new(
            ErrorCode::ConfigInvalid,
            format!(
                "unsupported runtime manifest schema {}",
                manifest.schema_version
            ),
        )
        .with_remediation("Migrate the manifest to schema version 1."));
    }
    if manifest.metadata.id.0.trim().is_empty() || manifest.metadata.name.trim().is_empty() {
        return Err(OpenWorkError::new(
            ErrorCode::ConfigInvalid,
            "runtime manifest identity cannot be empty",
        ));
    }
    if manifest
        .installer_sources
        .iter()
        .any(|source| !source.url.starts_with("https://"))
    {
        return Err(OpenWorkError::new(
            ErrorCode::ConfigInvalid,
            "runtime installer sources must use HTTPS",
        ));
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DistributionModel, RuntimeId};

    #[test]
    fn committed_mock_manifest_matches_contract() {
        let manifest =
            parse_manifest_json(include_str!("../../../runtime/manifests/mock.json")).unwrap();
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.metadata.id, RuntimeId::from("mock"));
        assert_eq!(manifest.metadata.distribution, DistributionModel::Embedded);

        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../contracts/schemas/runtime-manifest.v1.schema.json"
        ))
        .unwrap();
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);

        let claude =
            parse_manifest_json(include_str!("../../../runtime/manifests/claude-code.json"))
                .unwrap();
        assert_eq!(claude.metadata.id, RuntimeId::from("claude-code"));
        assert_eq!(claude.installer_sources.len(), 2);

        let codex =
            parse_manifest_json(include_str!("../../../runtime/manifests/codex.json")).unwrap();
        assert_eq!(codex.metadata.id, RuntimeId::from("codex"));
        assert_eq!(codex.installer_sources.len(), 2);
    }

    #[test]
    fn rejects_insecure_installer_sources() {
        let input = include_str!("../../../runtime/manifests/mock.json")
            .replace("https://example.invalid", "http://example.invalid");
        assert!(
            parse_manifest_json(&input)
                .unwrap_err()
                .message
                .contains("HTTPS")
        );
    }
}
