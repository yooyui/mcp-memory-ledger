use crate::domain::{
    DomainError,
    snapshot::{SelfSnapshot, SnapshotRequest},
};

pub fn build_snapshot(input: SnapshotRequest) -> Result<SelfSnapshot, DomainError> {
    let input = input.validate()?;
    let mut evidence = Vec::new();
    for reference in input.evidence {
        if evidence.contains(&reference) {
            continue;
        }

        evidence.push(reference);
        if evidence.len() == input.budget.max(1) {
            break;
        }
    }

    if evidence.is_empty() && !input.allow_empty_evidence {
        return Err(DomainError::InsufficientEvidence);
    }

    Ok(SelfSnapshot {
        identity: input.identity,
        commitments: input.commitments,
        claims: input.claims,
        evidence,
        episodes: input.episodes,
    })
}
