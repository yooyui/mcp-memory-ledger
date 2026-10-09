use chrono::{Duration, Utc};

use crate::{
    application::{
        build_self_snapshot::{self, BuildSelfSnapshotInput},
        run_reflection::{self, ReflectionInput},
    },
    domain::{
        commitment::Commitment,
        event::EventReference,
        reflection::Reflection,
        self_revision::{
            AutoReflectDiagnosticInput, AutoReflectDiagnosticSummary, AutoReflectOutcome,
            SelfRevisionProposal, SelfRevisionRequest, SuppressionCategory, TriggerType,
        },
        snapshot::SnapshotTimeWindow,
        types::{MemoryScope, Namespace, Owner},
    },
    error::AppError,
    ports::{
        ClaimStore, Clock, CommitmentStore, EpisodeStore, EventStore, EvidenceQuery, IdGenerator,
        IdentityStore, ModelPort, ReflectionTransactionRunner, StoredTriggerLedgerEntry,
        TriggerLedgerStatus, TriggerLedgerStore,
    },
};

const FAILURE_TRIGGER_THRESHOLD: usize = 2;
const AUTO_REFLECTION_COOLDOWN_HOURS: i64 = 24;
const DEFAULT_SNAPSHOT_BUDGET: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum RecursionGuard {
    #[default]
    Allow,
    SkipAutoReflection,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AutoReflectInput {
    pub namespace: Namespace,
    pub trigger_type: TriggerType,
    #[serde(default)]
    pub trigger_hints: Vec<String>,
    #[serde(default)]
    pub recursion_guard: RecursionGuard,
}

impl AutoReflectInput {
    pub fn for_failure(namespace: Namespace, trigger_hints: Vec<String>) -> Self {
        Self::new(namespace, TriggerType::Failure, trigger_hints)
    }

    pub fn for_conflict(namespace: Namespace, trigger_hints: Vec<String>) -> Self {
        Self::new(namespace, TriggerType::Conflict, trigger_hints)
    }

    pub fn for_periodic(namespace: Namespace, trigger_hints: Vec<String>) -> Self {
        Self::new(namespace, TriggerType::Periodic, trigger_hints)
    }

    pub fn new(
        namespace: Namespace,
        trigger_type: TriggerType,
        trigger_hints: Vec<String>,
    ) -> Self {
        Self {
            namespace,
            trigger_type,
            trigger_hints,
            recursion_guard: RecursionGuard::Allow,
        }
    }

    pub fn with_recursion_guard(mut self, recursion_guard: RecursionGuard) -> Self {
        self.recursion_guard = recursion_guard;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AutoReflectResult {
    pub triggered: bool,
    pub trigger_type: Option<TriggerType>,
    pub namespace: Option<String>,
    pub reflection_id: Option<String>,
    pub ledger_status: Option<TriggerLedgerStatus>,
    pub reason: Option<String>,
    pub trigger_key: Option<String>,
    #[serde(default)]
    pub evidence_event_ids: Vec<String>,
    pub cooldown_until: Option<chrono::DateTime<Utc>>,
    pub suppression_reason: Option<String>,
    pub diagnostics: AutoReflectDiagnosticSummary,
}

impl AutoReflectResult {
    fn skipped(input: &AutoReflectInput, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let evidence_event_ids = Vec::new();
        Self {
            triggered: false,
            trigger_type: Some(input.trigger_type),
            namespace: Some(input.namespace.as_str().to_string()),
            reflection_id: None,
            ledger_status: None,
            reason: Some(reason.clone()),
            trigger_key: Some(input.trigger_key()),
            evidence_event_ids: evidence_event_ids.clone(),
            cooldown_until: None,
            suppression_reason: None,
            diagnostics: AutoReflectDiagnosticSummary::new(AutoReflectDiagnosticInput {
                trigger_type: input.trigger_type,
                namespace: input.namespace.as_str().to_string(),
                trigger_key: input.trigger_key(),
                outcome: AutoReflectOutcome::Skipped,
                suppression_reason: None,
                suppression_category: None,
                rejection_reason: None,
                cooldown_boundary: None,
                evidence_window_size: 0,
                selected_evidence_event_ids: evidence_event_ids,
            }),
        }
    }

    fn not_triggered(candidate: &TriggerCandidate) -> Self {
        let evidence_event_ids = candidate.evidence_event_ids.clone();
        Self {
            triggered: false,
            trigger_type: Some(candidate.trigger_type),
            namespace: Some(candidate.namespace.as_str().to_string()),
            reflection_id: None,
            ledger_status: None,
            reason: None,
            trigger_key: Some(candidate.trigger_key.clone()),
            evidence_event_ids: evidence_event_ids.clone(),
            cooldown_until: None,
            suppression_reason: None,
            diagnostics: AutoReflectDiagnosticSummary::new(AutoReflectDiagnosticInput {
                trigger_type: candidate.trigger_type,
                namespace: candidate.namespace.as_str().to_string(),
                trigger_key: candidate.trigger_key.clone(),
                outcome: AutoReflectOutcome::NotTriggered,
                suppression_reason: None,
                suppression_category: None,
                rejection_reason: None,
                cooldown_boundary: None,
                evidence_window_size: evidence_event_ids.len(),
                selected_evidence_event_ids: Vec::new(),
            }),
        }
    }

    fn rejected(candidate: &TriggerCandidate, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let evidence_event_ids = candidate.evidence_event_ids.clone();
        Self {
            triggered: false,
            trigger_type: Some(candidate.trigger_type),
            namespace: Some(candidate.namespace.as_str().to_string()),
            reflection_id: None,
            ledger_status: Some(TriggerLedgerStatus::Rejected),
            reason: Some(reason.clone()),
            trigger_key: Some(candidate.trigger_key.clone()),
            evidence_event_ids: evidence_event_ids.clone(),
            cooldown_until: None,
            suppression_reason: None,
            diagnostics: AutoReflectDiagnosticSummary::new(AutoReflectDiagnosticInput {
                trigger_type: candidate.trigger_type,
                namespace: candidate.namespace.as_str().to_string(),
                trigger_key: candidate.trigger_key.clone(),
                outcome: AutoReflectOutcome::Rejected,
                suppression_reason: None,
                suppression_category: None,
                rejection_reason: Some(reason),
                cooldown_boundary: None,
                evidence_window_size: evidence_event_ids.len(),
                selected_evidence_event_ids: Vec::new(),
            }),
        }
    }

    fn suppressed(
        candidate: &TriggerCandidate,
        entry: &StoredTriggerLedgerEntry,
        suppression_category: SuppressionCategory,
    ) -> Self {
        let suppression_reason = suppression_category.as_str().to_string();
        let evidence_event_ids = candidate.evidence_event_ids.clone();
        Self {
            triggered: false,
            trigger_type: Some(candidate.trigger_type),
            namespace: Some(candidate.namespace.as_str().to_string()),
            reflection_id: entry.reflection_id.clone(),
            ledger_status: Some(entry.status),
            reason: None,
            trigger_key: Some(entry.trigger_key.clone()),
            evidence_event_ids: evidence_event_ids.clone(),
            cooldown_until: entry.cooldown_until,
            suppression_reason: Some(suppression_reason.clone()),
            diagnostics: AutoReflectDiagnosticSummary::new(AutoReflectDiagnosticInput {
                trigger_type: candidate.trigger_type,
                namespace: candidate.namespace.as_str().to_string(),
                trigger_key: entry.trigger_key.clone(),
                outcome: AutoReflectOutcome::Suppressed,
                suppression_reason: Some(suppression_reason),
                suppression_category: Some(suppression_category),
                rejection_reason: None,
                cooldown_boundary: entry.cooldown_until,
                evidence_window_size: evidence_event_ids.len(),
                selected_evidence_event_ids: Vec::new(),
            }),
        }
    }

    fn handled(
        candidate: &TriggerCandidate,
        evidence_event_ids: Vec<String>,
        reflection_id: String,
        cooldown_until: Option<chrono::DateTime<Utc>>,
    ) -> Self {
        let selected_evidence_event_ids = evidence_event_ids.clone();
        let evidence_window_size = candidate.evidence_event_ids.len();
        Self {
            triggered: true,
            trigger_type: Some(candidate.trigger_type),
            namespace: Some(candidate.namespace.as_str().to_string()),
            reflection_id: Some(reflection_id),
            ledger_status: Some(TriggerLedgerStatus::Handled),
            reason: None,
            trigger_key: Some(candidate.trigger_key.clone()),
            evidence_event_ids,
            cooldown_until,
            suppression_reason: None,
            diagnostics: AutoReflectDiagnosticSummary::new(AutoReflectDiagnosticInput {
                trigger_type: candidate.trigger_type,
                namespace: candidate.namespace.as_str().to_string(),
                trigger_key: candidate.trigger_key.clone(),
                outcome: AutoReflectOutcome::Handled,
                suppression_reason: None,
                suppression_category: None,
                rejection_reason: None,
                cooldown_boundary: cooldown_until,
                evidence_window_size,
                selected_evidence_event_ids,
            }),
        }
    }
}

#[derive(Debug, Clone)]
struct TriggerCandidate {
    trigger_type: TriggerType,
    namespace: Namespace,
    scope: MemoryScope,
    trigger_hints: Vec<String>,
    trigger_key: String,
    evidence_event_ids: Vec<String>,
    time_window: SnapshotTimeWindow,
    should_consider: bool,
    episode_watermark: Option<u64>,
}

#[derive(Debug, Clone)]
struct ValidatedSelfRevision {
    identity_claims: Option<Vec<String>>,
    commitments: Option<Vec<Commitment>>,
}

#[derive(Debug, Clone)]
struct IdentityRevisionContext {
    proposed_values: Vec<String>,
    supporting_claim_count: usize,
    cross_episode_support_count: usize,
    has_high_conflict: bool,
    now: chrono::DateTime<Utc>,
    latest_handled_at: Option<chrono::DateTime<Utc>>,
    cooldown_until: Option<chrono::DateTime<Utc>>,
    patch_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SuppressionDecision {
    reason: SuppressionCategory,
}

pub async fn execute<D>(deps: &D, input: AutoReflectInput) -> Result<AutoReflectResult, AppError>
where
    D: TriggerLedgerStore
        + EventStore
        + ClaimStore
        + CommitmentStore
        + IdentityStore
        + EpisodeStore
        + ReflectionTransactionRunner
        + ModelPort
        + Clock
        + IdGenerator
        + Sync,
{
    if input.recursion_guard == RecursionGuard::SkipAutoReflection {
        return Ok(AutoReflectResult::skipped(
            &input,
            "recursion guard enabled",
        ));
    }

    let candidate = detect_trigger_candidate(deps, &input).await?;
    if !candidate.should_consider {
        return Ok(AutoReflectResult::not_triggered(&candidate));
    }

    if let Some(suppression) = evaluate_trigger_suppression(deps, &candidate).await? {
        let entry = record_suppressed_trigger(deps, &candidate).await?;
        return Ok(AutoReflectResult::suppressed(
            &candidate,
            &entry,
            suppression.reason,
        ));
    }

    let snapshot = build_revision_snapshot(deps, &candidate).await?;
    let proposal = deps
        .propose_self_revision(SelfRevisionRequest::new(
            candidate.trigger_type,
            candidate.namespace.clone(),
            snapshot,
            candidate.evidence_event_ids.clone(),
            candidate.trigger_hints.clone(),
        ))
        .await?;

    if !proposal.should_reflect {
        record_rejected_trigger(deps, &candidate, None).await?;
        return Ok(AutoReflectResult::rejected(&candidate, proposal.rationale));
    }

    let governed_evidence_event_ids = match resolve_governed_evidence_window(
        deps,
        &candidate.evidence_event_ids,
        &proposal,
    )
    .await
    {
        Ok(governed_evidence_event_ids) => governed_evidence_event_ids,
        Err(error) => return record_rejection_then_propagate(deps, &candidate, error).await,
    };

    let validated =
        match validate_self_revision(deps, &candidate, &proposal, &governed_evidence_event_ids)
            .await
        {
            Ok(validated) => validated,
            Err(error) => return record_rejection_then_propagate(deps, &candidate, error).await,
        };

    match apply_validated_self_revision(
        deps,
        &candidate,
        &proposal,
        &governed_evidence_event_ids,
        validated,
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(error) => record_rejection_then_propagate(deps, &candidate, error).await,
    }
}

/// 统一收口治理校验失败的拒绝语义：先按既有规则落一条 Rejected ledger 记录
/// （保持 reflection_id 为 None 的现有约定），再把携带原始可解释原因的 error 原样向上传播。
/// 三条治理校验失败路径共用这一个出口，避免重复并保证失败语义一致。
async fn record_rejection_then_propagate<D>(
    deps: &D,
    candidate: &TriggerCandidate,
    error: AppError,
) -> Result<AutoReflectResult, AppError>
where
    D: TriggerLedgerStore + Clock + IdGenerator + Sync,
{
    record_rejected_trigger(deps, candidate, None).await?;
    Err(error)
}

async fn detect_trigger_candidate<D>(
    deps: &D,
    input: &AutoReflectInput,
) -> Result<TriggerCandidate, AppError>
where
    D: EventStore + EpisodeStore + Sync,
{
    let scope = MemoryScope::for_namespace(input.namespace.clone());
    // Freeze the trigger window before applying the authorized scope. Querying
    // the scope first would let older in-scope rows refill a five-row window
    // after newer out-of-scope rows were excluded.
    let trigger_window_event_ids = match input.trigger_type {
        TriggerType::Failure => {
            deps.query_evidence_event_ids(EvidenceQuery {
                namespace: None,
                owner: None,
                kind: Some(crate::domain::types::EventKind::Action),
                limit: Some(5),
                recorded_after: None,
                recorded_before: None,
                event_id_prefix: None,
            })
            .await?
        }
        TriggerType::Conflict | TriggerType::Periodic => {
            deps.query_evidence_event_ids(EvidenceQuery {
                namespace: None,
                owner: None,
                kind: None,
                limit: Some(5),
                recorded_after: None,
                recorded_before: None,
                event_id_prefix: None,
            })
            .await?
        }
    };
    let trigger_window_event_ids = normalize_event_ids(trigger_window_event_ids)?;
    let trigger_manifest = trigger_window_event_ids
        .iter()
        .cloned()
        .map(EventReference::from_event_id)
        .collect::<Vec<_>>();
    let evidence_event_ids = normalize_event_ids(
        deps.list_event_references_for_snapshot(
            &scope,
            Some(&trigger_manifest),
            &SnapshotTimeWindow::unbounded(),
        )
        .await?,
    )?;
    let evidence_manifest = evidence_event_ids
        .iter()
        .cloned()
        .map(EventReference::from_event_id)
        .collect::<Vec<_>>();
    let timestamps = deps
        .list_recorded_at_for_snapshot_manifest(&scope, &evidence_manifest)
        .await?;
    let time_window = match (timestamps.iter().min(), timestamps.iter().max()) {
        (Some(recorded_after), Some(recorded_before)) => {
            SnapshotTimeWindow::new(Some(*recorded_after), Some(*recorded_before))
                .map_err(AppError::from)?
        }
        _ => SnapshotTimeWindow::unbounded(),
    };
    let episode_watermark = dedupe_strings(
        deps.list_episode_references_for_snapshot(&scope, &time_window)
            .await?,
    )
    .len() as u64;
    let should_consider = match input.trigger_type {
        TriggerType::Failure => {
            has_any_hint(&input.trigger_hints, &["failure", "rollback"])
                && evidence_event_ids.len() >= FAILURE_TRIGGER_THRESHOLD
        }
        TriggerType::Conflict => {
            !evidence_event_ids.is_empty()
                && has_any_hint(&input.trigger_hints, &["conflict", "rollback", "identity"])
        }
        TriggerType::Periodic => !evidence_event_ids.is_empty() && episode_watermark > 0,
    };

    Ok(TriggerCandidate {
        trigger_type: input.trigger_type,
        namespace: input.namespace.clone(),
        scope,
        trigger_hints: input.trigger_hints.clone(),
        trigger_key: canonical_trigger_key(&input.namespace, input.trigger_type),
        evidence_event_ids,
        time_window,
        should_consider,
        episode_watermark: Some(episode_watermark),
    })
}

async fn evaluate_trigger_suppression<D>(
    deps: &D,
    candidate: &TriggerCandidate,
) -> Result<Option<SuppressionDecision>, AppError>
where
    D: TriggerLedgerStore + Clock + Sync,
{
    let Some(latest) = deps.latest_trigger_entry(&candidate.trigger_key).await? else {
        return Ok(None);
    };

    let now = deps.now().await?;
    if matches!(
        latest.status,
        TriggerLedgerStatus::Handled | TriggerLedgerStatus::Suppressed
    ) && latest
        .cooldown_until
        .is_some_and(|cooldown_until| cooldown_until > now)
    {
        return Ok(Some(SuppressionDecision {
            reason: SuppressionCategory::CooldownActive,
        }));
    }

    let latest_handled = deps
        .latest_handled_trigger_entry(&candidate.trigger_key)
        .await?;

    if !candidate.evidence_event_ids.is_empty()
        && latest_handled
            .as_ref()
            .is_some_and(|entry| entry.evidence_window == candidate.evidence_event_ids)
    {
        return Ok(Some(SuppressionDecision {
            reason: SuppressionCategory::EvidenceWindowUnchanged,
        }));
    }

    if candidate.trigger_type == TriggerType::Periodic
        && latest_handled.as_ref().is_some_and(|entry| {
            entry.episode_watermark.unwrap_or_default()
                >= candidate.episode_watermark.unwrap_or_default()
        })
    {
        return Ok(Some(SuppressionDecision {
            reason: SuppressionCategory::EpisodeWatermarkUnchanged,
        }));
    }

    Ok(None)
}

async fn build_revision_snapshot<D>(
    deps: &D,
    candidate: &TriggerCandidate,
) -> Result<crate::domain::snapshot::SelfSnapshot, AppError>
where
    D: ClaimStore + CommitmentStore + IdentityStore + EventStore + EpisodeStore + Sync,
{
    Ok(build_self_snapshot::execute(
        deps,
        BuildSelfSnapshotInput {
            scope: candidate.scope.clone(),
            evidence_manifest: Some(
                candidate
                    .evidence_event_ids
                    .iter()
                    .cloned()
                    .map(EventReference::from_event_id)
                    .collect(),
            ),
            time_window: candidate.time_window.clone(),
            budget: crate::domain::snapshot::SnapshotBudget::new(
                candidate
                    .evidence_event_ids
                    .len()
                    .max(DEFAULT_SNAPSHOT_BUDGET),
            ),
        },
    )
    .await?
    .snapshot)
}

async fn validate_self_revision<D>(
    deps: &D,
    candidate: &TriggerCandidate,
    proposal: &SelfRevisionProposal,
    governed_evidence_event_ids: &[String],
) -> Result<ValidatedSelfRevision, AppError>
where
    D: ClaimStore + EpisodeStore + TriggerLedgerStore + Clock + Sync,
{
    let identity_claims = if proposal.machine_patch.identity_patch.is_some() {
        Some(validate_identity_patch(
            proposal,
            &build_identity_revision_context(deps, candidate, proposal).await?,
        )?)
    } else {
        None
    };

    let commitments = if let Some(commitment_patch) = &proposal.machine_patch.commitment_patch {
        if governed_evidence_event_ids.is_empty() {
            return Err(AppError::InvalidParams(
                "commitment auto-reflection updates require supporting evidence".to_string(),
            ));
        }

        if commitment_patch.commitments.is_empty() {
            return Err(AppError::InvalidParams(
                "commitment auto-reflection patch must include at least one commitment".to_string(),
            ));
        }

        Some(
            commitment_patch
                .commitments
                .iter()
                .cloned()
                .map(|commitment| Commitment::new(Owner::Self_, commitment))
                .collect(),
        )
    } else {
        None
    };

    if identity_claims.is_none() && commitments.is_none() {
        return Err(AppError::InvalidParams(
            "auto-reflection proposals must include at least one governed patch".to_string(),
        ));
    }

    Ok(ValidatedSelfRevision {
        identity_claims,
        commitments,
    })
}

async fn build_identity_revision_context<D>(
    deps: &D,
    candidate: &TriggerCandidate,
    proposal: &SelfRevisionProposal,
) -> Result<IdentityRevisionContext, AppError>
where
    D: ClaimStore + EpisodeStore + TriggerLedgerStore + Clock + Sync,
{
    let active_claims = deps.list_active_claims().await?;
    let proposed_identity_claims = proposal
        .machine_patch
        .identity_patch
        .as_ref()
        .map(|patch| patch.canonical_claims.clone())
        .unwrap_or_default();
    let proposed_values = dedupe_strings(
        proposed_identity_claims
            .iter()
            .filter_map(|claim| claim.rsplit_once('=').map(|(_, value)| value.to_string()))
            .collect(),
    );
    let supporting_claims: Vec<_> = active_claims
        .iter()
        .filter(|claim| {
            claim.claim.namespace() == &candidate.namespace
                && proposed_values
                    .iter()
                    .any(|value| value == claim.claim.object())
        })
        .cloned()
        .collect();
    let supporting_claim_count = supporting_claims.len();
    let supporting_shapes = supporting_claims
        .iter()
        .fold(Vec::new(), |mut shapes, claim| {
            let shape = (
                claim.claim.subject().to_string(),
                claim.claim.predicate().to_string(),
            );
            if !shapes.contains(&shape) {
                shapes.push(shape);
            }
            shapes
        });
    let has_high_conflict = active_claims.iter().any(|claim| {
        claim.claim.namespace() == &candidate.namespace
            && supporting_shapes.iter().any(|(subject, predicate)| {
                claim.claim.subject() == subject
                    && claim.claim.predicate() == predicate
                    && !proposed_values
                        .iter()
                        .any(|value| value == claim.claim.object())
            })
    });
    let latest_entry = deps.latest_trigger_entry(&candidate.trigger_key).await?;
    let supporting_claim_ids = supporting_claims
        .iter()
        .map(|claim| claim.claim_id.clone())
        .collect::<Vec<_>>();
    let cross_episode_support_count = dedupe_strings(
        deps.list_episode_references_supporting_claims(&candidate.scope, &supporting_claim_ids)
            .await?,
    )
    .len();
    let now = deps.now().await?;

    Ok(IdentityRevisionContext {
        proposed_values,
        supporting_claim_count,
        cross_episode_support_count,
        has_high_conflict,
        now,
        latest_handled_at: latest_entry.as_ref().and_then(|entry| {
            (entry.status == TriggerLedgerStatus::Handled)
                .then_some(entry.handled_at)
                .flatten()
        }),
        cooldown_until: latest_entry.as_ref().and_then(|entry| {
            matches!(
                entry.status,
                TriggerLedgerStatus::Handled | TriggerLedgerStatus::Suppressed
            )
            .then_some(entry.cooldown_until)
            .flatten()
        }),
        patch_size: proposed_identity_claims.len(),
    })
}

fn validate_identity_patch(
    proposal: &SelfRevisionProposal,
    context: &IdentityRevisionContext,
) -> Result<Vec<String>, AppError> {
    ensure_min_supporting_claims(context, 3)?;
    ensure_cross_episode_support(context, 2)?;
    ensure_no_high_conflict(context)?;
    ensure_identity_cooldown_elapsed(context)?;
    ensure_identity_patch_limit(context, 2)?;
    Ok(materialize_identity_claims(proposal))
}

fn ensure_min_supporting_claims(
    context: &IdentityRevisionContext,
    minimum: usize,
) -> Result<(), AppError> {
    if context.supporting_claim_count < minimum {
        return Err(AppError::InvalidParams(format!(
            "identity auto-reflection requires at least {minimum} supporting claims"
        )));
    }

    Ok(())
}

fn ensure_cross_episode_support(
    context: &IdentityRevisionContext,
    minimum: usize,
) -> Result<(), AppError> {
    if context.cross_episode_support_count < minimum {
        return Err(AppError::InvalidParams(format!(
            "identity auto-reflection requires support across at least {minimum} episodes"
        )));
    }

    Ok(())
}

fn ensure_no_high_conflict(context: &IdentityRevisionContext) -> Result<(), AppError> {
    if context.has_high_conflict {
        return Err(AppError::InvalidParams(format!(
            "identity auto-reflection cannot proceed while high-conflict evidence remains active for {:?}",
            context.proposed_values
        )));
    }

    Ok(())
}

fn ensure_identity_cooldown_elapsed(context: &IdentityRevisionContext) -> Result<(), AppError> {
    if context
        .cooldown_until
        .is_some_and(|cooldown_until| cooldown_until > context.now)
    {
        return Err(AppError::InvalidParams(
            "identity auto-reflection cooldown has not elapsed".to_string(),
        ));
    }

    if context.latest_handled_at.is_some_and(|handled_at| {
        handled_at + Duration::hours(AUTO_REFLECTION_COOLDOWN_HOURS) > context.now
    }) {
        return Err(AppError::InvalidParams(
            "identity auto-reflection handled too recently".to_string(),
        ));
    }

    Ok(())
}

fn ensure_identity_patch_limit(
    context: &IdentityRevisionContext,
    maximum: usize,
) -> Result<(), AppError> {
    if context.patch_size == 0 || context.patch_size > maximum {
        return Err(AppError::InvalidParams(format!(
            "identity auto-reflection patch must contain between 1 and {maximum} claims"
        )));
    }

    Ok(())
}

fn materialize_identity_claims(proposal: &SelfRevisionProposal) -> Vec<String> {
    proposal
        .machine_patch
        .identity_patch
        .as_ref()
        .map(|patch| patch.canonical_claims.clone())
        .unwrap_or_default()
}

async fn apply_validated_self_revision<D>(
    deps: &D,
    candidate: &TriggerCandidate,
    proposal: &SelfRevisionProposal,
    governed_evidence_event_ids: &[String],
    validated: ValidatedSelfRevision,
) -> Result<AutoReflectResult, AppError>
where
    D: EventStore + ReflectionTransactionRunner + Clock + IdGenerator + Sync,
{
    let handled_trigger_ledger_entry = build_trigger_entry(
        deps,
        candidate,
        &candidate.evidence_event_ids,
        TriggerLedgerStatus::Handled,
        None,
        None,
    )
    .await?;
    let reflection_input = ReflectionInput::record_only(
        Reflection::new(proposal.rationale.clone()),
        governed_evidence_event_ids.to_vec(),
    )
    .with_optional_replacement_evidence_query(None)
    .with_handled_trigger_ledger_entry(handled_trigger_ledger_entry.clone());
    let reflection_input = if let Some(identity_claims) = validated.identity_claims {
        reflection_input.with_identity_update(identity_claims)
    } else {
        reflection_input
    };
    let reflection_input = if let Some(commitments) = validated.commitments {
        reflection_input.with_commitment_updates(commitments)
    } else {
        reflection_input
    };
    let reflection = run_reflection::execute(deps, reflection_input).await?;

    Ok(AutoReflectResult::handled(
        candidate,
        governed_evidence_event_ids.to_vec(),
        reflection.reflection_id,
        handled_trigger_ledger_entry.cooldown_until,
    ))
}

async fn record_suppressed_trigger<D>(
    deps: &D,
    candidate: &TriggerCandidate,
) -> Result<StoredTriggerLedgerEntry, AppError>
where
    D: TriggerLedgerStore + Clock + IdGenerator + Sync,
{
    let preserved_reflection_id =
        latest_suppression_reflection_id(deps, &candidate.trigger_key).await?;
    let preserved_entry = latest_live_suppression_entry(deps, &candidate.trigger_key).await?;
    record_trigger_entry(
        deps,
        candidate,
        TriggerLedgerStatus::Suppressed,
        preserved_reflection_id,
        preserved_entry.and_then(|entry| entry.cooldown_until),
    )
    .await
}

async fn record_rejected_trigger<D>(
    deps: &D,
    candidate: &TriggerCandidate,
    reflection_id: Option<String>,
) -> Result<StoredTriggerLedgerEntry, AppError>
where
    D: TriggerLedgerStore + Clock + IdGenerator + Sync,
{
    record_trigger_entry(
        deps,
        candidate,
        TriggerLedgerStatus::Rejected,
        reflection_id,
        None,
    )
    .await
}

async fn build_trigger_entry<D>(
    deps: &D,
    candidate: &TriggerCandidate,
    evidence_event_ids: &[String],
    status: TriggerLedgerStatus,
    reflection_id: Option<String>,
    cooldown_until_override: Option<chrono::DateTime<Utc>>,
) -> Result<StoredTriggerLedgerEntry, AppError>
where
    D: Clock + IdGenerator + Sync,
{
    let now = deps.now().await?;
    let handled_at = (status == TriggerLedgerStatus::Handled).then_some(now);
    let cooldown_until = cooldown_until_override.or(match status {
        TriggerLedgerStatus::Handled | TriggerLedgerStatus::Suppressed => {
            Some(now + Duration::hours(AUTO_REFLECTION_COOLDOWN_HOURS))
        }
        TriggerLedgerStatus::Pending | TriggerLedgerStatus::Rejected => None,
    });

    Ok(StoredTriggerLedgerEntry {
        ledger_id: deps.next_id().await?,
        trigger_type: candidate.trigger_type,
        namespace: candidate.namespace.clone(),
        trigger_key: candidate.trigger_key.clone(),
        status,
        evidence_window: evidence_event_ids.to_vec(),
        handled_at,
        cooldown_until,
        episode_watermark: candidate.episode_watermark,
        reflection_id,
    })
}

async fn record_trigger_entry<D>(
    deps: &D,
    candidate: &TriggerCandidate,
    status: TriggerLedgerStatus,
    reflection_id: Option<String>,
    cooldown_until_override: Option<chrono::DateTime<Utc>>,
) -> Result<StoredTriggerLedgerEntry, AppError>
where
    D: TriggerLedgerStore + Clock + IdGenerator + Sync,
{
    let entry = build_trigger_entry(
        deps,
        candidate,
        &candidate.evidence_event_ids,
        status,
        reflection_id,
        cooldown_until_override,
    )
    .await?;
    deps.record_trigger_attempt(entry.clone()).await?;
    Ok(entry)
}

async fn latest_live_suppression_entry<D>(
    deps: &D,
    trigger_key: &str,
) -> Result<Option<StoredTriggerLedgerEntry>, AppError>
where
    D: TriggerLedgerStore + Clock + Sync,
{
    let Some(latest) = deps.latest_trigger_entry(trigger_key).await? else {
        return Ok(None);
    };
    let now = deps.now().await?;

    Ok(matches!(
        latest.status,
        TriggerLedgerStatus::Handled | TriggerLedgerStatus::Suppressed
    )
    .then_some(latest)
    .filter(|entry| {
        entry
            .cooldown_until
            .is_some_and(|cooldown_until| cooldown_until > now)
    }))
}

async fn latest_suppression_reflection_id<D>(
    deps: &D,
    trigger_key: &str,
) -> Result<Option<String>, AppError>
where
    D: TriggerLedgerStore + Sync,
{
    let latest_reflection_id = deps
        .latest_trigger_entry(trigger_key)
        .await?
        .filter(|entry| {
            matches!(
                entry.status,
                TriggerLedgerStatus::Handled | TriggerLedgerStatus::Suppressed
            )
        })
        .and_then(|entry| entry.reflection_id);

    if latest_reflection_id.is_some() {
        return Ok(latest_reflection_id);
    }

    Ok(deps
        .latest_handled_trigger_entry(trigger_key)
        .await?
        .and_then(|entry| entry.reflection_id))
}

fn canonical_trigger_key(namespace: &Namespace, trigger_type: TriggerType) -> String {
    format!(
        "{}:{}",
        namespace.as_str(),
        trigger_type_label(trigger_type)
    )
}

impl AutoReflectInput {
    pub fn trigger_key(&self) -> String {
        canonical_trigger_key(&self.namespace, self.trigger_type)
    }
}

fn trigger_type_label(trigger_type: TriggerType) -> &'static str {
    match trigger_type {
        TriggerType::Conflict => "conflict",
        TriggerType::Failure => "failure",
        TriggerType::Periodic => "periodic",
    }
}

fn has_any_hint(hints: &[String], expected: &[&str]) -> bool {
    hints.iter().any(|hint| {
        expected
            .iter()
            .any(|expected_hint| hint.eq_ignore_ascii_case(expected_hint))
    })
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for value in values {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    deduped
}

/// The model proposal boundary follows the same raw-or-`event:<id>` contract as
/// explicit reflection input. Downstream store calls and diagnostic `*_event_ids`
/// stay raw for compatibility.
fn normalize_event_ids(event_ids: Vec<String>) -> Result<Vec<String>, AppError> {
    let mut normalized = Vec::new();
    for event_id in event_ids {
        let event_id = EventReference::parse(event_id)
            .map_err(AppError::from)?
            .event_id()
            .to_string();
        if !normalized.contains(&event_id) {
            normalized.push(event_id);
        }
    }
    Ok(normalized)
}

async fn resolve_governed_evidence_window<D>(
    deps: &D,
    candidate_evidence_event_ids: &[String],
    proposal: &SelfRevisionProposal,
) -> Result<Vec<String>, AppError>
where
    D: EventStore + Sync,
{
    let candidate_evidence_event_ids = normalize_event_ids(candidate_evidence_event_ids.to_vec())?;
    let query_constrained_candidate_ids =
        if let Some(proposed_evidence_query) = proposal.proposed_evidence_query.clone() {
            let query_limit = proposed_evidence_query.limit;
            // 与 DTO / SQLite chokepoint 的 limit==0 早拒对称：model 提议的 evidence
            // query limit==0 必须在解析入口拒绝为 InvalidParams，不得被后置 take(0)
            // 静默收窄为空集、当成确定性空匹配掩盖（违反 C1「不得被当成空匹配掩盖」意图）。
            if query_limit == Some(0) {
                return Err(AppError::InvalidParams(
                    "proposed evidence query limit must be at least 1".to_string(),
                ));
            }
            let proposed_query_event_ids = normalize_event_ids(
                deps.query_evidence_event_ids_unbounded(EvidenceQuery {
                    namespace: proposed_evidence_query.namespace,
                    owner: proposed_evidence_query.owner,
                    kind: proposed_evidence_query.kind,
                    limit: None,
                    recorded_after: proposed_evidence_query.recorded_after,
                    recorded_before: proposed_evidence_query.recorded_before,
                    event_id_prefix: proposed_evidence_query.event_id_prefix,
                })
                .await?,
            );
            let proposed_query_event_ids = proposed_query_event_ids?;
            let filtered_candidate_ids = candidate_evidence_event_ids
                .iter()
                .filter(|event_id| proposed_query_event_ids.contains(event_id))
                .cloned()
                .collect::<Vec<_>>();

            Some((filtered_candidate_ids, query_limit))
        } else {
            None
        };

    if proposal.proposed_evidence_event_ids.is_empty() {
        let Some((filtered_candidate_ids, query_limit)) = query_constrained_candidate_ids else {
            return Ok(candidate_evidence_event_ids);
        };

        if filtered_candidate_ids.is_empty() {
            return Err(AppError::InvalidParams(
                "proposed evidence query did not match the current trigger window".to_string(),
            ));
        }

        let governed_evidence_event_ids = if let Some(limit) = query_limit {
            filtered_candidate_ids.into_iter().take(limit).collect()
        } else {
            filtered_candidate_ids
        };
        return Ok(governed_evidence_event_ids);
    }

    let proposed_evidence_event_ids =
        normalize_event_ids(proposal.proposed_evidence_event_ids.clone())?;
    if proposed_evidence_event_ids
        .iter()
        .any(|event_id| !candidate_evidence_event_ids.contains(event_id))
    {
        return Err(AppError::InvalidParams(
            "model proposed evidence outside the current trigger window".to_string(),
        ));
    }

    if let Some((eligible_query_event_ids, _)) = query_constrained_candidate_ids
        && proposed_evidence_event_ids
            .iter()
            .any(|event_id| !eligible_query_event_ids.contains(event_id))
    {
        return Err(AppError::InvalidParams(
            "model proposed evidence ids do not satisfy the proposed evidence query within the current trigger window".to_string(),
        ));
    }

    Ok(proposed_evidence_event_ids)
}
