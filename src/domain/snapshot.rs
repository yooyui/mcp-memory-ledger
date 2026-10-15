use chrono::{DateTime, Utc};

use crate::domain::DomainError;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SnapshotTimeWindow {
    pub recorded_after: Option<DateTime<Utc>>,
    pub recorded_before: Option<DateTime<Utc>>,
}

impl SnapshotTimeWindow {
    pub fn new(
        recorded_after: Option<DateTime<Utc>>,
        recorded_before: Option<DateTime<Utc>>,
    ) -> Result<Self, DomainError> {
        let window = Self {
            recorded_after,
            recorded_before,
        };
        window.validate()?;
        Ok(window)
    }

    pub fn unbounded() -> Self {
        Self::default()
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self
            .recorded_after
            .as_ref()
            .zip(self.recorded_before.as_ref())
            .is_some_and(|(after, before)| after > before)
        {
            return Err(DomainError::InvalidSnapshotTimeWindow);
        }

        Ok(())
    }

    pub fn is_unbounded(&self) -> bool {
        self.recorded_after.is_none() && self.recorded_before.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SnapshotBudget {
    limit: usize,
}

impl SnapshotBudget {
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }

    pub fn max(&self, minimum: usize) -> usize {
        self.limit.max(minimum)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SnapshotRequest {
    pub identity: Vec<String>,
    pub commitments: Vec<String>,
    pub claims: Vec<String>,
    pub evidence: Vec<String>,
    pub episodes: Vec<String>,
    pub budget: SnapshotBudget,
    pub allow_empty_evidence: bool,
}

impl SnapshotRequest {
    pub fn fixture_minimal() -> Self {
        Self {
            identity: vec!["identity:self=architect".to_string()],
            commitments: vec!["forbid:write_identity_core_directly".to_string()],
            claims: vec!["claim:self.role=architect".to_string()],
            evidence: vec!["event:evt-minimal".to_string()],
            episodes: vec!["episode:minimal".to_string()],
            budget: SnapshotBudget::new(1),
            allow_empty_evidence: false,
        }
    }

    pub fn validate(self) -> Result<Self, DomainError> {
        if self.evidence.is_empty() && !self.allow_empty_evidence {
            return Err(DomainError::InsufficientEvidence);
        }

        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SelfSnapshot {
    pub identity: Vec<String>,
    pub commitments: Vec<String>,
    pub claims: Vec<String>,
    pub evidence: Vec<String>,
    pub episodes: Vec<String>,
}
