use crate::{
    domain::{
        event::{EventReference, MAX_EVIDENCE_MANIFEST_ITEMS},
        rules::snapshot_builder::build_snapshot,
        snapshot::{SelfSnapshot, SnapshotBudget, SnapshotRequest, SnapshotTimeWindow},
        types::MemoryScope,
    },
    error::AppError,
    ports::{ClaimStatus, ClaimStore, CommitmentStore, EpisodeStore, EventStore, IdentityStore},
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildSelfSnapshotInput {
    pub scope: MemoryScope,
    pub evidence_manifest: Option<Vec<EventReference>>,
    pub time_window: SnapshotTimeWindow,
    pub budget: SnapshotBudget,
}

impl BuildSelfSnapshotInput {
    pub fn for_revision_window(evidence_window_len: usize) -> Self {
        Self {
            scope: MemoryScope::legacy_unscoped(),
            evidence_manifest: None,
            time_window: SnapshotTimeWindow::unbounded(),
            budget: SnapshotBudget::new(evidence_window_len.max(1)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildSelfSnapshotResult {
    pub snapshot: SelfSnapshot,
}

pub async fn execute<D>(
    deps: &D,
    input: BuildSelfSnapshotInput,
) -> Result<BuildSelfSnapshotResult, AppError>
where
    D: IdentityStore + CommitmentStore + ClaimStore + EventStore + EpisodeStore + Sync,
{
    input.time_window.validate().map_err(|_| {
        AppError::InvalidParams(
            "recorded_after must be less than or equal to recorded_before".to_string(),
        )
    })?;
    if input
        .evidence_manifest
        .as_ref()
        .is_some_and(|manifest| manifest.len() > MAX_EVIDENCE_MANIFEST_ITEMS)
    {
        return Err(AppError::InvalidParams(format!(
            "evidence_manifest must contain at most {MAX_EVIDENCE_MANIFEST_ITEMS} entries"
        )));
    }
    if input.evidence_manifest.is_some() && !input.scope.is_explicitly_scoped() {
        return Err(AppError::InvalidParams(
            "evidence_manifest requires an explicit namespace".to_string(),
        ));
    }
    if !input.time_window.is_unbounded() && !input.scope.is_explicitly_scoped() {
        return Err(AppError::InvalidParams(
            "snapshot time window requires an explicit namespace".to_string(),
        ));
    }

    let allow_empty_evidence =
        input.evidence_manifest.is_some() || !input.time_window.is_unbounded();

    let identity = deps.load_identity().await?;
    let commitments = deps
        .list_commitments()
        .await?
        .into_iter()
        .map(|commitment| commitment.description().to_string())
        .collect();
    let claims = deps
        .list_active_claims_in_scope(&input.scope)
        .await?
        .into_iter()
        .filter(|claim| claim.status == ClaimStatus::Active)
        .map(|claim| claim.snapshot_value())
        .collect();
    let request = SnapshotRequest {
        identity: identity.canonical_claims().to_vec(),
        commitments,
        claims,
        evidence: deps
            .list_event_references_for_snapshot(
                &input.scope,
                input.evidence_manifest.as_deref(),
                &input.time_window,
            )
            .await?,
        episodes: deps
            .list_episode_references_for_snapshot(&input.scope, &input.time_window)
            .await?,
        budget: input.budget,
        allow_empty_evidence,
    };

    Ok(BuildSelfSnapshotResult {
        snapshot: build_snapshot(request)?,
    })
}
