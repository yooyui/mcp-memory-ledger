use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use agent_llm_mm::{
    application::decide_with_snapshot::{DecideWithSnapshotInput, DecisionAuthority, execute},
    domain::{
        commitment::Commitment,
        self_revision::{SelfRevisionProposal, SelfRevisionRequest},
        snapshot::SelfSnapshot,
        types::Owner,
    },
    error::AppError,
    ports::{CommitmentStore, ModelDecision, ModelDecisionRequest, ModelPort},
};
use async_trait::async_trait;
use serde_json::json;

#[tokio::test]
async fn decision_returns_blocked_without_calling_model_when_gate_fails() {
    let deps = test_support::deps_with_blocking_commitment();

    let result = execute(&deps, test_support::blocked_decision_input())
        .await
        .unwrap();

    assert!(result.blocked);
    assert!(result.decision.is_none());
    assert_eq!(result.protocol_version, 2);
    assert!(result.decision_id.starts_with("decision:"));
    assert_eq!(result.requested_action, "write_identity_core_directly");
    assert_eq!(result.selected_action, None);
    assert_eq!(result.confidence, None);
    assert_eq!(result.status, "blocked");
    assert_eq!(
        result.reason.as_deref(),
        Some("commitment_gate_blocked_action")
    );
    assert_eq!(result.gate.name, "commitment_gate");
    assert!(result.gate.blocked);
    assert_eq!(
        result.decision_authority,
        DecisionAuthority::NotApplicableBlocked
    );
    assert_eq!(result.policy_scope, "server_commitment_gate_only");
    assert_eq!(
        result.provider_diagnostics_class,
        "not-applicable-gate-blocked"
    );
    assert_eq!(result.policy_checks[0].name, "commitment_gate");
    assert!(result.policy_checks[0].blocked);
    assert!(
        result
            .non_claims
            .contains(&"not a full planning engine".to_string())
    );

    let serialized = serde_json::to_value(&result).unwrap();
    assert_eq!(serialized["blocked"], true);
    assert_eq!(serialized["decision"], serde_json::Value::Null);
    assert_eq!(serialized["protocol_version"], 2);
    assert_eq!(
        serialized["requested_action"],
        "write_identity_core_directly"
    );
    assert_eq!(serialized["selected_action"], serde_json::Value::Null);
    assert!(
        serialized["decision_id"]
            .as_str()
            .unwrap()
            .starts_with("decision:")
    );
    assert_eq!(serialized["status"], "blocked");
    assert_eq!(serialized["reason"], "commitment_gate_blocked_action");
    assert_eq!(serialized["decision_authority"], "not_applicable_blocked");
    assert_eq!(serialized["policy_scope"], "server_commitment_gate_only");
    assert_eq!(
        serialized["provider_diagnostics_class"],
        "not-applicable-gate-blocked"
    );
    assert_eq!(
        serialized["gate"],
        json!({
            "name": "commitment_gate",
            "blocked": true,
            "reason": "commitment_gate_blocked_action"
        })
    );
    assert_eq!(deps.model_call_count(), 0);
    assert!(deps.last_request().is_none());
}

#[tokio::test]
async fn caller_snapshot_cannot_remove_server_commitment() {
    let deps = test_support::deps_with_blocking_commitment();
    let mut input = test_support::blocked_decision_input();
    input.snapshot.commitments.clear();

    let result = execute(&deps, input).await.unwrap();

    assert!(result.blocked);
    assert!(result.decision.is_none());
    assert_eq!(result.requested_action, "write_identity_core_directly");
    assert_eq!(result.selected_action, None);
    assert_eq!(deps.model_call_count(), 0);
}

#[tokio::test]
async fn provider_selected_action_is_rechecked_against_server_commitments() {
    let deps = test_support::deps_with_forbidden_selected_action();

    let result = execute(&deps, test_support::decision_input())
        .await
        .unwrap();

    assert!(result.blocked);
    assert!(result.decision.is_none());
    assert_eq!(result.requested_action, "read_identity_core");
    assert_eq!(
        result.selected_action.as_deref(),
        Some("write_identity_core_directly")
    );
    assert_eq!(result.status, "blocked");
    assert_eq!(
        result.reason.as_deref(),
        Some("commitment_gate_blocked_selected_action")
    );
    assert!(result.gate.blocked);
    assert_eq!(
        result.decision_authority,
        DecisionAuthority::NotApplicableBlocked
    );
    assert_eq!(result.policy_scope, "server_commitment_gate_only");
    assert_eq!(
        result.provider_diagnostics_class,
        "bounded-local-policy-rejected"
    );
    assert_eq!(deps.model_call_count(), 1);

    let request = deps.last_request().expect("model should receive request");
    assert_eq!(
        request.snapshot.commitments,
        vec!["forbid:write_identity_core_directly".to_string()]
    );
}

#[tokio::test]
async fn mock_model_receives_snapshot_context_when_gate_passes() {
    let deps = test_support::deps_with_mock_model();
    let input = test_support::decision_input();

    let result = execute(&deps, input.clone()).await.unwrap();

    assert_eq!(
        result.decision,
        Some(ModelDecision::new("summarize_memory_state".to_string()))
    );
    assert_eq!(result.protocol_version, 2);
    assert!(result.decision_id.starts_with("decision:"));
    assert_eq!(result.requested_action, "read_identity_core");
    assert_eq!(
        result.selected_action.as_deref(),
        Some("summarize_memory_state")
    );
    assert_eq!(
        result.confidence,
        Some("bounded-local-metadata".to_string())
    );
    assert_eq!(result.status, "model_decision");
    assert!(result.reason.is_none());
    assert_eq!(
        result.decision_authority,
        DecisionAuthority::ExperimentalNonAuthoritative
    );
    assert_eq!(result.policy_scope, "server_commitment_gate_only");
    assert_eq!(result.provider_diagnostics_class, "bounded-local-only");
    assert_eq!(result.gate.name, "commitment_gate");
    assert!(!result.gate.blocked);

    let serialized = serde_json::to_value(&result).unwrap();
    assert_eq!(serialized["blocked"], false);
    assert_eq!(
        serialized["decision"],
        json!({ "action": "summarize_memory_state" })
    );
    assert_eq!(serialized["protocol_version"], 2);
    assert_eq!(serialized["requested_action"], "read_identity_core");
    assert_eq!(serialized["selected_action"], "summarize_memory_state");
    assert_eq!(serialized["status"], "model_decision");
    assert_eq!(serialized["reason"], serde_json::Value::Null);
    assert_eq!(
        serialized["decision_authority"],
        "experimental_non_authoritative"
    );
    assert_eq!(serialized["policy_scope"], "server_commitment_gate_only");
    assert_eq!(
        serialized["provider_diagnostics_class"],
        "bounded-local-only"
    );
    assert_eq!(
        serialized["gate"],
        json!({
            "name": "commitment_gate",
            "blocked": false,
            "reason": null
        })
    );
    assert!(
        serialized["non_claims"]
            .as_array()
            .expect("non_claims array")
            .iter()
            .any(|claim| claim == "not an authoritative policy decision")
    );

    let request = deps.last_request().expect("model should receive request");
    assert_eq!(request.task, input.task);
    assert_eq!(request.action, input.action);
    assert_eq!(request.snapshot, input.snapshot);
}

mod test_support {
    use super::*;

    use agent_llm_mm::adapters::model::mock::MockModel;

    pub fn deps_with_blocking_commitment() -> DecisionDeps {
        DecisionDeps {
            model: Arc::new(MockModel),
            model_calls: Arc::new(AtomicUsize::new(0)),
            last_request: Arc::new(Mutex::new(None)),
            server_commitments: vec![Commitment::new(
                Owner::Self_,
                "forbid:write_identity_core_directly",
            )],
            selected_action_override: None,
            snapshot: SelfSnapshot {
                identity: vec!["identity:self=architect".to_string()],
                commitments: vec!["forbid:write_identity_core_directly".to_string()],
                claims: vec!["self.role is architect".to_string()],
                evidence: vec!["event:evt-1".to_string()],
                episodes: vec!["episode:task-6".to_string()],
            },
        }
    }

    pub fn deps_with_mock_model() -> DecisionDeps {
        DecisionDeps {
            model: Arc::new(MockModel),
            model_calls: Arc::new(AtomicUsize::new(0)),
            last_request: Arc::new(Mutex::new(None)),
            server_commitments: Vec::new(),
            selected_action_override: None,
            snapshot: SelfSnapshot {
                identity: vec!["identity:self=architect".to_string()],
                commitments: Vec::new(),
                claims: vec!["self.role is architect".to_string()],
                evidence: vec!["event:evt-1".to_string()],
                episodes: vec!["episode:task-6".to_string()],
            },
        }
    }

    pub fn deps_with_forbidden_selected_action() -> DecisionDeps {
        let mut deps = deps_with_blocking_commitment();
        deps.snapshot.commitments.clear();
        deps.selected_action_override = Some("write_identity_core_directly".to_string());
        deps
    }

    pub fn blocked_decision_input() -> DecideWithSnapshotInput {
        DecideWithSnapshotInput {
            task: "summarize current memory".to_string(),
            action: "write_identity_core_directly".to_string(),
            snapshot: deps_with_blocking_commitment().snapshot,
        }
    }

    pub fn decision_input() -> DecideWithSnapshotInput {
        DecideWithSnapshotInput {
            task: "summarize current memory".to_string(),
            action: "read_identity_core".to_string(),
            snapshot: deps_with_mock_model().snapshot,
        }
    }

    #[derive(Clone)]
    pub struct DecisionDeps {
        pub model: Arc<MockModel>,
        pub model_calls: Arc<AtomicUsize>,
        pub last_request: Arc<Mutex<Option<ModelDecisionRequest>>>,
        pub server_commitments: Vec<Commitment>,
        pub selected_action_override: Option<String>,
        pub snapshot: SelfSnapshot,
    }

    impl DecisionDeps {
        pub fn model_call_count(&self) -> usize {
            self.model_calls.load(Ordering::SeqCst)
        }

        pub fn last_request(&self) -> Option<ModelDecisionRequest> {
            self.last_request.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl CommitmentStore for DecisionDeps {
        async fn list_commitments(&self) -> Result<Vec<Commitment>, AppError> {
            Ok(self.server_commitments.clone())
        }
    }

    #[async_trait]
    impl ModelPort for DecisionDeps {
        async fn decide(&self, request: ModelDecisionRequest) -> Result<ModelDecision, AppError> {
            self.model_calls.fetch_add(1, Ordering::SeqCst);
            *self.last_request.lock().unwrap() = Some(request.clone());
            if let Some(selected_action) = &self.selected_action_override {
                return Ok(ModelDecision::new(selected_action.clone()));
            }
            self.model.decide(request).await
        }

        async fn propose_self_revision(
            &self,
            request: SelfRevisionRequest,
        ) -> Result<SelfRevisionProposal, AppError> {
            self.model.propose_self_revision(request).await
        }
    }
}
