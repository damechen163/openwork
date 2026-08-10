//! Deterministic policy evaluation against the frozen action contract.

use crate::config::{ConfiguredDecision, POLICY_CONFIG_VERSION, PolicyConfig};
use openwork_core::OpenWorkError;
use openwork_execution::{
    ActionPolicy, ActionRequest, EXECUTION_SCHEMA_VERSION, PolicyDecision, PolicyEvaluation,
    RiskLevel, UtcTimestamp, sha256_bytes,
};

/// Resource matching extension point; M1 ships exact string matching only.
pub trait ResourceMatcher: Send + Sync {
    #[must_use]
    fn matches(&self, configured: &str, requested: &str) -> bool;
}

/// Case-sensitive exact resource matcher used by the M1 policy engine.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExactResourceMatcher;

impl ResourceMatcher for ExactResourceMatcher {
    fn matches(&self, configured: &str, requested: &str) -> bool {
        configured == requested
    }
}

/// Typed, fail-closed evaluator. It derives risk exclusively from policy rules.
pub struct PolicyEngine<M = ExactResourceMatcher> {
    config: PolicyConfig,
    matcher: M,
}

impl PolicyEngine<ExactResourceMatcher> {
    /// Builds the default exact-resource engine from YAML.
    ///
    /// # Errors
    ///
    /// Returns an error when the policy configuration is invalid.
    pub fn from_yaml(input: &str) -> Result<Self, OpenWorkError> {
        Ok(Self {
            config: PolicyConfig::from_yaml(input)?,
            matcher: ExactResourceMatcher,
        })
    }
}

impl<M: ResourceMatcher> PolicyEngine<M> {
    #[must_use]
    pub const fn with_matcher(config: PolicyConfig, matcher: M) -> Self {
        Self { config, matcher }
    }

    /// Evaluates at an explicit trusted time for deterministic orchestration and tests.
    #[must_use]
    pub fn evaluate_at(
        &self,
        request: &ActionRequest,
        evaluated_at: UtcTimestamp,
    ) -> PolicyEvaluation {
        let Some(rule) = self.config.action(&request.action) else {
            return evaluation(
                request,
                PolicyDecision::Deny,
                RiskLevel::DestructiveOrFinancial,
                "default:unknown",
                "unknown_action_denied",
                evaluated_at,
            );
        };
        if !request.parameters_match_hash() {
            return evaluation(
                request,
                PolicyDecision::Deny,
                rule.risk(),
                "contract:binding_mismatch",
                "action_binding_mismatch",
                evaluated_at,
            );
        }

        let (configured, rule_id, reason) = rule.resources().map_or_else(
            || {
                (
                    rule.decision(),
                    format!("action:{}:default", request.action),
                    "action_rule_match",
                )
            },
            |resources| {
                resources
                    .exact()
                    .iter()
                    .find(|(configured, _)| self.matcher.matches(configured, &request.resource))
                    .map_or_else(
                        || {
                            (
                                resources.default_decision(),
                                format!("action:{}:resource-default", request.action),
                                "resource_default_match",
                            )
                        },
                        |(resource, decision)| {
                            let digest = sha256_bytes(resource.as_bytes());
                            (
                                *decision,
                                format!(
                                    "action:{}:resource-exact:{}",
                                    request.action,
                                    digest.as_str()
                                ),
                                "resource_exact_match",
                            )
                        },
                    )
            },
        );
        evaluation(
            request,
            ConfiguredDecision::into(configured),
            rule.risk(),
            &rule_id,
            reason,
            evaluated_at,
        )
    }
}

impl<M: ResourceMatcher> ActionPolicy for PolicyEngine<M> {
    fn evaluate(&self, request: &ActionRequest) -> PolicyEvaluation {
        self.evaluate_at(request, UtcTimestamp::now())
    }
}

fn evaluation(
    request: &ActionRequest,
    decision: PolicyDecision,
    risk: RiskLevel,
    rule_id: &str,
    reason_code: &str,
    evaluated_at: UtcTimestamp,
) -> PolicyEvaluation {
    PolicyEvaluation {
        schema_version: EXECUTION_SCHEMA_VERSION,
        run_id: request.run_id.clone(),
        action_id: request.id.clone(),
        parameter_hash: request.parameter_hash().clone(),
        decision,
        effective_risk: risk,
        policy_version: format!("yaml-v{POLICY_CONFIG_VERSION}"),
        rule_id: rule_id.to_owned(),
        reason_code: reason_code.to_owned(),
        evaluated_at,
    }
}
