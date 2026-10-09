use crate::{
    domain::{rules::commitment_gate::gate_decision, snapshot::SelfSnapshot},
    error::AppError,
    ports::{CommitmentStore, ModelDecision, ModelDecisionRequest, ModelPort},
};

const DECISION_PROTOCOL_VERSION: u32 = 2;
const COMMITMENT_GATE_NAME: &str = "commitment_gate";
const COMMITMENT_GATE_BLOCKED_REASON: &str = "commitment_gate_blocked_action";
const COMMITMENT_GATE_BLOCKED_SELECTED_REASON: &str = "commitment_gate_blocked_selected_action";
const DECISION_POLICY_SCOPE: &str = "server_commitment_gate_only";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecideWithSnapshotInput {
    pub task: String,
    pub action: String,
    pub snapshot: SelfSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAuthority {
    NotApplicableBlocked,
    ExperimentalNonAuthoritative,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecideWithSnapshotResult {
    pub blocked: bool,
    pub decision: Option<ModelDecision>,
    pub protocol_version: u32,
    pub decision_id: String,
    pub requested_action: String,
    pub selected_action: Option<String>,
    pub confidence: Option<String>,
    pub status: String,
    pub reason: Option<String>,
    /// 决策结果的权威性边界。当前 provider 只返回 action string，
    /// 因此非 blocked 结果也只能是 experimental / non-authoritative。
    pub decision_authority: DecisionAuthority,
    /// 当前 policy 元数据只覆盖服务端 commitment literal gate，
    /// 不能解释成完整 policy arbitration 已通过。
    pub policy_scope: String,
    pub gate: DecisionGateMetadata,
    pub policy_checks: Vec<DecisionGateMetadata>,
    /// envelope 自身的 provider 诊断承载等级（bounded 本地标量）：
    /// 显式声明本协议不携带 provider 原生结构化诊断对象，仅给出本地可解释分类。
    pub provider_diagnostics_class: String,
    pub non_claims: Vec<String>,
}

impl DecideWithSnapshotResult {
    fn blocked_by_commitment_gate(action: String) -> Self {
        let gate = DecisionGateMetadata::blocked(COMMITMENT_GATE_BLOCKED_REASON);
        Self {
            blocked: true,
            decision: None,
            protocol_version: DECISION_PROTOCOL_VERSION,
            decision_id: decision_id_for(&action),
            requested_action: action,
            selected_action: None,
            confidence: None,
            status: "blocked".to_string(),
            reason: Some(COMMITMENT_GATE_BLOCKED_REASON.to_string()),
            decision_authority: DecisionAuthority::NotApplicableBlocked,
            policy_scope: DECISION_POLICY_SCOPE.to_string(),
            gate: gate.clone(),
            policy_checks: vec![gate],
            provider_diagnostics_class: "not-applicable-gate-blocked".to_string(),
            non_claims: decision_non_claims(),
        }
    }

    fn model_decision(requested_action: String, decision: ModelDecision) -> Self {
        let selected_action = decision.action.clone();
        let gate = DecisionGateMetadata::passed();
        Self {
            blocked: false,
            decision: Some(decision),
            protocol_version: DECISION_PROTOCOL_VERSION,
            decision_id: decision_id_for(&requested_action),
            requested_action,
            selected_action: Some(selected_action),
            confidence: Some("bounded-local-metadata".to_string()),
            status: "model_decision".to_string(),
            reason: None,
            decision_authority: DecisionAuthority::ExperimentalNonAuthoritative,
            policy_scope: DECISION_POLICY_SCOPE.to_string(),
            gate: gate.clone(),
            policy_checks: vec![gate],
            provider_diagnostics_class: "bounded-local-only".to_string(),
            non_claims: decision_non_claims(),
        }
    }

    fn blocked_selected_action(requested_action: String, selected_action: String) -> Self {
        let gate = DecisionGateMetadata::blocked(COMMITMENT_GATE_BLOCKED_SELECTED_REASON);
        Self {
            blocked: true,
            decision: None,
            protocol_version: DECISION_PROTOCOL_VERSION,
            decision_id: decision_id_for(&requested_action),
            requested_action,
            selected_action: Some(selected_action),
            confidence: None,
            status: "blocked".to_string(),
            reason: Some(COMMITMENT_GATE_BLOCKED_SELECTED_REASON.to_string()),
            decision_authority: DecisionAuthority::NotApplicableBlocked,
            policy_scope: DECISION_POLICY_SCOPE.to_string(),
            gate: gate.clone(),
            policy_checks: vec![gate],
            provider_diagnostics_class: "bounded-local-policy-rejected".to_string(),
            non_claims: decision_non_claims(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecisionGateMetadata {
    pub name: String,
    pub blocked: bool,
    pub reason: Option<String>,
}

impl DecisionGateMetadata {
    fn blocked(reason: &str) -> Self {
        Self {
            name: COMMITMENT_GATE_NAME.to_string(),
            blocked: true,
            reason: Some(reason.to_string()),
        }
    }

    fn passed() -> Self {
        Self {
            name: COMMITMENT_GATE_NAME.to_string(),
            blocked: false,
            reason: None,
        }
    }
}

pub async fn execute<D>(
    deps: &D,
    mut input: DecideWithSnapshotInput,
) -> Result<DecideWithSnapshotResult, AppError>
where
    D: CommitmentStore + ModelPort + Sync,
{
    let trusted_commitments = deps
        .list_commitments()
        .await?
        .into_iter()
        .map(|commitment| commitment.description().to_string())
        .collect::<Vec<_>>();
    input.snapshot.commitments = trusted_commitments.clone();

    let gate = gate_decision(&input.action, &trusted_commitments);
    if gate.blocked {
        return Ok(DecideWithSnapshotResult::blocked_by_commitment_gate(
            input.action,
        ));
    }

    let requested_action = input.action.clone();
    let decision = deps
        .decide(ModelDecisionRequest::new(
            input.task,
            input.action,
            input.snapshot,
        ))
        .await?;

    let selected_gate = gate_decision(&decision.action, &trusted_commitments);
    if selected_gate.blocked {
        return Ok(DecideWithSnapshotResult::blocked_selected_action(
            requested_action,
            decision.action,
        ));
    }

    Ok(DecideWithSnapshotResult::model_decision(
        requested_action,
        decision,
    ))
}

fn decision_id_for(action: &str) -> String {
    format!("decision:{}", action.replace(char::is_whitespace, "_"))
}

fn decision_non_claims() -> Vec<String> {
    vec![
        "not a full planning engine".to_string(),
        "not policy arbitration".to_string(),
        "not provider-native structured decision JSON".to_string(),
        "not an authoritative policy decision".to_string(),
        "not confidence scoring beyond bounded local metadata".to_string(),
        "caller snapshot fields outside commitments remain untrusted".to_string(),
        "not a server-created snapshot handle".to_string(),
    ]
}
