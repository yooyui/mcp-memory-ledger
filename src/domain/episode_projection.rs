use std::collections::HashSet;

use crate::{
    domain::{event::EventReference, self_revision::SELF_REVISION_DURABLE_WRITE_PATH},
    error::AppError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeProjectionInput {
    pub episode_reference: String,
    pub episode_event_ids: Vec<String>,
    pub objective: Option<String>,
    pub outcome: Option<String>,
    pub lesson: Option<String>,
    pub linked_evidence_ids: Vec<String>,
}

/// 有界生命周期分类，派生自 outcome 是否存在；只读、不写 identity/commitments。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeLifecycleStatus {
    Open,
    Concluded,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EpisodeSummaryProjection {
    pub episode_reference: String,
    pub objective: Option<String>,
    pub outcome: Option<String>,
    pub lesson: Option<String>,
    pub linked_evidence_ids: Vec<String>,
    pub event_count: usize,
    pub lifecycle_status: EpisodeLifecycleStatus,
    pub writes_performed: bool,
    pub durable_self_model_write_path: String,
    pub identity_or_commitment_updates: Vec<String>,
}

pub fn build_episode_summary_projection(
    input: EpisodeProjectionInput,
) -> Result<EpisodeSummaryProjection, AppError> {
    if input.episode_reference.trim().is_empty() {
        return Err(AppError::InvalidParams(
            "episode projection requires an episode reference".to_string(),
        ));
    }
    let episode_event_ids = normalize_event_ids(input.episode_event_ids)?;
    let linked_evidence_ids = normalize_event_ids(input.linked_evidence_ids)?;
    let event_set = episode_event_ids.iter().cloned().collect::<HashSet<_>>();
    if let Some(outside_id) = linked_evidence_ids
        .iter()
        .find(|event_id| !event_set.contains(*event_id))
    {
        return Err(AppError::InvalidParams(format!(
            "linked evidence {outside_id} is outside the episode event set"
        )));
    }

    let lifecycle_status = if input.outcome.is_some() {
        EpisodeLifecycleStatus::Concluded
    } else {
        EpisodeLifecycleStatus::Open
    };

    Ok(EpisodeSummaryProjection {
        episode_reference: input.episode_reference,
        objective: input.objective,
        outcome: input.outcome,
        lesson: input.lesson,
        linked_evidence_ids,
        event_count: episode_event_ids.len(),
        lifecycle_status,
        writes_performed: false,
        durable_self_model_write_path: SELF_REVISION_DURABLE_WRITE_PATH.to_string(),
        identity_or_commitment_updates: Vec::new(),
    })
}

/// Episode projection `*_event_ids` retain raw compatibility readback after
/// accepting raw ids or `event:<id>` references at the input boundary.
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
