use serde::Serialize;

use crate::{
    domain::{
        event::{EventReference, MAX_EVIDENCE_MANIFEST_ITEMS},
        evidence_relation::{EvidenceRelationInput, build_evidence_relation_report},
        types::{MemoryScope, Namespace, Owner},
    },
    error::AppError,
    ports::{MemoryReadStore, ScopedEventIdQuery},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetEvidenceRelationInput {
    pub namespace: Namespace,
    pub trigger_window: Vec<EventReference>,
    pub selected_evidence: Vec<EventReference>,
    pub selection_basis: Option<String>,
}

impl GetEvidenceRelationInput {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.trigger_window.len() > MAX_EVIDENCE_MANIFEST_ITEMS {
            return Err(AppError::InvalidParams(format!(
                "trigger_window_event_ids must contain at most {MAX_EVIDENCE_MANIFEST_ITEMS} entries"
            )));
        }
        if self.selected_evidence.len() > MAX_EVIDENCE_MANIFEST_ITEMS {
            return Err(AppError::InvalidParams(format!(
                "selected_evidence_event_ids must contain at most {MAX_EVIDENCE_MANIFEST_ITEMS} entries"
            )));
        }
        if let Some(basis) = self.selection_basis.as_deref()
            && (basis.is_empty() || basis.trim() != basis)
        {
            return Err(AppError::InvalidParams(
                "selection_basis must be non-empty and have no leading or trailing whitespace"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetEvidenceRelationResult {
    pub owner: Owner,
    pub namespace: String,
    pub protocol_version: u32,
    pub trigger_window_size: usize,
    pub selected_count: usize,
    pub rejected_count: usize,
    pub no_widening_policy: &'static str,
    pub weight_policy: &'static str,
    pub relations: Vec<EvidenceRelationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceRelationRecord {
    pub event_reference: String,
    pub window_rank: usize,
    pub selected: bool,
    pub relation_status: String,
    pub selection_weight: u8,
    pub selection_basis: Option<String>,
    pub rejection_reason: Option<String>,
}

pub async fn execute<D>(
    deps: &D,
    input: GetEvidenceRelationInput,
) -> Result<GetEvidenceRelationResult, AppError>
where
    D: MemoryReadStore + Sync,
{
    input.validate()?;
    let scope = MemoryScope::for_namespace(input.namespace.clone());
    let owner = scope
        .owner()
        .expect("namespace-derived memory scope must have an owner");
    let trigger_ids = raw_event_ids(&input.trigger_window);
    let selected_ids = raw_event_ids(&input.selected_evidence);
    let scoped_ids = deps
        .query_scoped_event_ids(ScopedEventIdQuery {
            scope,
            event_ids: trigger_ids.clone(),
        })
        .await?;
    let scoped_trigger_ids = trigger_ids
        .into_iter()
        .filter(|event_id| scoped_ids.contains(event_id))
        .collect::<Vec<_>>();
    let report = build_evidence_relation_report(EvidenceRelationInput {
        trigger_window_event_ids: scoped_trigger_ids,
        selected_evidence_event_ids: selected_ids,
        selection_basis: input.selection_basis,
    })?;

    Ok(GetEvidenceRelationResult {
        owner,
        namespace: input.namespace.as_str().to_string(),
        protocol_version: report.protocol_version,
        trigger_window_size: report.trigger_window_size,
        selected_count: report.selected_count,
        rejected_count: report.rejected_count,
        no_widening_policy: report.no_widening_policy,
        weight_policy: report.weight_policy,
        relations: report
            .relations
            .into_iter()
            .map(|relation| EvidenceRelationRecord {
                event_reference: EventReference::from_event_id(relation.event_id).canonical(),
                window_rank: relation.window_rank,
                selected: relation.selected,
                relation_status: relation.relation_status,
                selection_weight: relation.selection_weight,
                selection_basis: relation.selection_basis,
                rejection_reason: relation.rejection_reason,
            })
            .collect(),
    })
}

fn raw_event_ids(references: &[EventReference]) -> Vec<String> {
    references
        .iter()
        .map(|reference| reference.event_id().to_string())
        .collect()
}
