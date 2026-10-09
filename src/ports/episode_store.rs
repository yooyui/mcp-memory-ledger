use async_trait::async_trait;

use crate::{
    domain::{snapshot::SnapshotTimeWindow, types::MemoryScope},
    error::AppError,
};

#[async_trait]
pub trait EpisodeStore {
    async fn record_event_in_episode(
        &self,
        episode_reference: String,
        event_id: String,
    ) -> Result<(), AppError>;

    async fn list_episode_references(&self) -> Result<Vec<String>, AppError>;
    async fn list_episode_references_supporting_claims(
        &self,
        scope: &MemoryScope,
        claim_ids: &[String],
    ) -> Result<Vec<String>, AppError> {
        if !scope.is_explicitly_scoped() {
            return Err(AppError::InvalidParams(
                "claim-to-evidence-to-episode lookup requires an explicit namespace".to_string(),
            ));
        }
        if claim_ids.is_empty() {
            return Ok(Vec::new());
        }

        Err(AppError::InvalidParams(
            "claim-to-evidence-to-episode lookup is not supported by this store".to_string(),
        ))
    }
    async fn list_episode_references_in_scope(
        &self,
        scope: &MemoryScope,
    ) -> Result<Vec<String>, AppError> {
        if scope.is_legacy_unscoped() {
            self.list_episode_references().await
        } else {
            Err(AppError::InvalidParams(
                "scoped episode lookup is not supported by this store".to_string(),
            ))
        }
    }
    async fn list_episode_references_for_snapshot(
        &self,
        scope: &MemoryScope,
        time_window: &SnapshotTimeWindow,
    ) -> Result<Vec<String>, AppError> {
        time_window.validate().map_err(|_| {
            AppError::InvalidParams(
                "recorded_after must be less than or equal to recorded_before".to_string(),
            )
        })?;
        if !time_window.is_unbounded() && !scope.is_explicitly_scoped() {
            Err(AppError::InvalidParams(
                "snapshot time window requires an explicit namespace".to_string(),
            ))
        } else if time_window.is_unbounded() {
            self.list_episode_references_in_scope(scope).await
        } else {
            Err(AppError::InvalidParams(
                "time-bounded episode lookup is not supported by this store".to_string(),
            ))
        }
    }
}
