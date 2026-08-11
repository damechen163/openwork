//! Versioned YAML policy configuration.

use openwork_core::{ErrorCode, OpenWorkError};
use openwork_execution::{PolicyDecision, RiskLevel};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

/// Frozen M1 policy configuration version.
pub const POLICY_CONFIG_VERSION: u32 = 1;
const MAX_POLICY_BYTES: usize = 1024 * 1024;

/// Validated, deterministic policy configuration.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    version: u32,
    defaults: PolicyDefaults,
    #[serde(deserialize_with = "unique_string_map")]
    actions: BTreeMap<String, ActionRule>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyDefaults {
    unknown: ConfiguredDecision,
}

/// One policy-derived risk and its resource-aware decisions.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionRule {
    risk: RiskLevel,
    decision: ConfiguredDecision,
    #[serde(default, deserialize_with = "non_null_option")]
    resources: Option<ResourceRules>,
}

/// Exact resource overrides followed by one explicit default.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRules {
    #[serde(deserialize_with = "unique_string_map")]
    exact: BTreeMap<String, ConfiguredDecision>,
    default: ConfiguredDecision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConfiguredDecision {
    Allow,
    Deny,
    Approval,
}

impl PolicyConfig {
    /// Parses and semantically validates one bounded YAML document.
    ///
    /// # Errors
    ///
    /// Returns a redacted error for malformed, duplicate, unknown, or unsafe configuration.
    pub fn from_yaml(input: &str) -> Result<Self, OpenWorkError> {
        if input.len() > MAX_POLICY_BYTES {
            return Err(config_error("policy configuration exceeds the 1 MiB limit"));
        }
        let config = yaml_serde::from_str::<Self>(input)
            .map_err(|_| config_error("policy YAML is invalid"))?;
        config.validate()?;
        Ok(config)
    }

    pub(crate) fn action(&self, name: &str) -> Option<&ActionRule> {
        self.actions.get(name)
    }

    fn validate(&self) -> Result<(), OpenWorkError> {
        if self.version != POLICY_CONFIG_VERSION
            || self.defaults.unknown != ConfiguredDecision::Deny
            || self.actions.iter().any(|(name, rule)| {
                !valid_action_name(name) || !rule.is_safe() || !rule.resources_are_valid()
            })
        {
            return Err(config_error(
                "policy configuration violates M1 safety invariants",
            ));
        }
        Ok(())
    }
}

impl ActionRule {
    pub(crate) const fn risk(&self) -> RiskLevel {
        self.risk
    }

    pub(crate) const fn decision(&self) -> ConfiguredDecision {
        self.decision
    }

    pub(crate) const fn resources(&self) -> Option<&ResourceRules> {
        self.resources.as_ref()
    }

    fn is_safe(&self) -> bool {
        safe_decision(self.risk, self.decision)
            && self.resources.as_ref().is_none_or(|resources| {
                safe_decision(self.risk, resources.default)
                    && resources
                        .exact
                        .values()
                        .all(|decision| safe_decision(self.risk, *decision))
            })
    }

    fn resources_are_valid(&self) -> bool {
        self.resources.as_ref().is_none_or(|resources| {
            resources
                .exact
                .keys()
                .all(|resource| !resource.trim().is_empty() && resource.len() <= 2048)
        })
    }
}

impl ResourceRules {
    pub(crate) fn exact(&self) -> &BTreeMap<String, ConfiguredDecision> {
        &self.exact
    }

    pub(crate) const fn default_decision(&self) -> ConfiguredDecision {
        self.default
    }
}

impl From<ConfiguredDecision> for PolicyDecision {
    fn from(value: ConfiguredDecision) -> Self {
        match value {
            ConfiguredDecision::Allow => Self::Allow,
            ConfiguredDecision::Deny => Self::Deny,
            ConfiguredDecision::Approval => Self::RequireApproval,
        }
    }
}

fn safe_decision(risk: RiskLevel, decision: ConfiguredDecision) -> bool {
    match risk {
        RiskLevel::Read | RiskLevel::LocalWrite => true,
        RiskLevel::InternalMutation | RiskLevel::ExternalEffect => {
            decision != ConfiguredDecision::Allow
        }
        RiskLevel::DestructiveOrFinancial => decision == ConfiguredDecision::Deny,
    }
}

fn valid_action_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    name.len() <= 128
        && first.is_ascii_lowercase()
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

fn unique_string_map<'de, D, V>(deserializer: D) -> Result<BTreeMap<String, V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct UniqueMapVisitor<V>(PhantomData<V>);
    impl<'de, V: Deserialize<'de>> Visitor<'de> for UniqueMapVisitor<V> {
        type Value = BTreeMap<String, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a mapping with unique string keys")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<String, V>()? {
                if values.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate policy key"));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(UniqueMapVisitor(PhantomData))
}

fn non_null_option<'de, D, V>(deserializer: D) -> Result<Option<V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    V::deserialize(deserializer).map(Some)
}

fn config_error(message: &str) -> OpenWorkError {
    OpenWorkError::new(ErrorCode::ConfigInvalid, message)
}
