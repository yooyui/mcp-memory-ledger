use async_trait::async_trait;

use crate::{
    domain::{claim::ClaimDraft, types::MemoryScope},
    error::AppError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClaimStatus {
    Active,
    Disputed,
    Superseded,
}

impl ClaimStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disputed => "disputed",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StoredClaim {
    pub claim_id: String,
    pub claim: ClaimDraft,
    pub status: ClaimStatus,
}

impl StoredClaim {
    pub fn new(claim_id: String, claim: ClaimDraft, status: ClaimStatus) -> Self {
        Self {
            claim_id,
            claim,
            status,
        }
    }

    pub fn snapshot_value(&self) -> String {
        format!(
            "{}:{} {} {}",
            self.claim.namespace(),
            self.claim.subject(),
            self.claim.predicate(),
            self.claim.object()
        )
    }
}

#[async_trait]
pub trait ClaimStore {
    async fn upsert_claim(&self, claim: StoredClaim) -> Result<(), AppError>;
    async fn link_evidence(&self, claim_id: String, event_id: String) -> Result<(), AppError>;
    async fn list_active_claims(&self) -> Result<Vec<StoredClaim>, AppError>;
    async fn list_active_claims_in_scope(
        &self,
        scope: &MemoryScope,
    ) -> Result<Vec<StoredClaim>, AppError> {
        Ok(self
            .list_active_claims()
            .await?
            .into_iter()
            .filter(|claim| {
                scope
                    .owner()
                    .is_none_or(|owner| claim.claim.owner() == owner)
                    && scope
                        .namespace()
                        .is_none_or(|namespace| claim.claim.namespace() == namespace)
            })
            .collect())
    }
    async fn update_claim_status(
        &self,
        claim_id: &str,
        status: ClaimStatus,
    ) -> Result<(), AppError>;
}
