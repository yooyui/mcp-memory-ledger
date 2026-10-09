use std::collections::HashSet;

use crate::{domain::event::EventReference, error::AppError};

pub const EVIDENCE_RELATION_PROTOCOL_VERSION: u32 = 2;
pub const EVIDENCE_RELATION_NO_WIDENING_POLICY: &str = "selected_subset_of_trigger_window";
pub const EVIDENCE_RELATION_WEIGHT_POLICY: &str = "bounded_selected_binary_weight";
pub const EVIDENCE_RELATION_SELECTED_STATUS: &str = "selected";
pub const EVIDENCE_RELATION_AVAILABLE_NOT_SELECTED_STATUS: &str = "available_not_selected";
pub const EVIDENCE_RELATION_NOT_SELECTED_REASON: &str = "not_selected_by_current_policy";
pub const SELECTED_EVIDENCE_WEIGHT: u8 = 100;
pub const UNSELECTED_EVIDENCE_WEIGHT: u8 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRelationInput {
    pub trigger_window_event_ids: Vec<String>,
    pub selected_evidence_event_ids: Vec<String>,
    pub selection_basis: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct EvidenceRelationReport {
    pub protocol_version: u32,
    pub trigger_window_size: usize,
    pub selected_count: usize,
    pub rejected_count: usize,
    pub no_widening_policy: &'static str,
    pub weight_policy: &'static str,
    pub relations: Vec<EvidenceRelation>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceRelation {
    pub event_id: String,
    pub window_rank: usize,
    pub selected: bool,
    pub relation_status: String,
    pub selection_weight: u8,
    pub selection_basis: Option<String>,
    pub rejection_reason: Option<String>,
}

pub fn build_evidence_relation_report(
    input: EvidenceRelationInput,
) -> Result<EvidenceRelationReport, AppError> {
    let selected_ids = normalize_event_ids(input.selected_evidence_event_ids)?;
    let trigger_window_ids = normalize_event_ids(input.trigger_window_event_ids)?;
    let trigger_window_set = trigger_window_ids.iter().cloned().collect::<HashSet<_>>();

    if let Some(outside_id) = selected_ids
        .iter()
        .find(|event_id| !trigger_window_set.contains(*event_id))
    {
        return Err(AppError::InvalidParams(format!(
            "selected evidence {outside_id} is outside the trigger window"
        )));
    }

    let relations = trigger_window_ids
        .iter()
        .enumerate()
        .map(|(index, event_id)| {
            let selected = selected_ids.contains(event_id);
            EvidenceRelation {
                event_id: event_id.clone(),
                window_rank: index + 1,
                selected,
                relation_status: relation_status(selected).to_string(),
                selection_weight: selection_weight(selected),
                selection_basis: selected.then(|| input.selection_basis.clone()).flatten(),
                rejection_reason: rejection_reason(selected).map(str::to_string),
            }
        })
        .collect::<Vec<_>>();

    Ok(EvidenceRelationReport {
        protocol_version: EVIDENCE_RELATION_PROTOCOL_VERSION,
        trigger_window_size: trigger_window_ids.len(),
        selected_count: selected_ids.len(),
        rejected_count: trigger_window_ids.len().saturating_sub(selected_ids.len()),
        no_widening_policy: EVIDENCE_RELATION_NO_WIDENING_POLICY,
        weight_policy: EVIDENCE_RELATION_WEIGHT_POLICY,
        relations,
    })
}

fn relation_status(selected: bool) -> &'static str {
    if selected {
        EVIDENCE_RELATION_SELECTED_STATUS
    } else {
        EVIDENCE_RELATION_AVAILABLE_NOT_SELECTED_STATUS
    }
}

fn selection_weight(selected: bool) -> u8 {
    if selected {
        SELECTED_EVIDENCE_WEIGHT
    } else {
        UNSELECTED_EVIDENCE_WEIGHT
    }
}

fn rejection_reason(selected: bool) -> Option<&'static str> {
    (!selected).then_some(EVIDENCE_RELATION_NOT_SELECTED_REASON)
}

/// Projection `*_event_ids` keep raw compatibility readback, while accepting
/// either a raw id or an `event:<id>` reference at their boundary.
fn normalize_event_ids(values: Vec<String>) -> Result<Vec<String>, AppError> {
    let mut normalized = Vec::new();
    for value in values {
        let event_id = EventReference::parse(value)
            .map_err(AppError::from)?
            .event_id()
            .to_string();
        if !normalized.contains(&event_id) {
            normalized.push(event_id);
        }
    }
    Ok(normalized)
}
