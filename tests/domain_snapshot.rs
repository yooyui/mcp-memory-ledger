use agent_llm_mm::domain::{
    DomainError,
    claim::ClaimReference,
    event::EventReference,
    rules::{commitment_gate::gate_decision, snapshot_builder::build_snapshot},
    snapshot::{SnapshotBudget, SnapshotRequest, SnapshotTimeWindow},
};
use chrono::{DateTime, Utc};

#[test]
fn raw_and_prefixed_claim_ids_share_one_canonical_reference() {
    let raw = ClaimReference::parse("evt-1:claim:0").unwrap();
    let prefixed = ClaimReference::parse("claim:evt-1:claim:0").unwrap();

    assert_eq!(raw, prefixed);
    assert_eq!(raw.claim_id(), "evt-1:claim:0");
    assert_eq!(raw.canonical(), "claim:evt-1:claim:0");
    assert_eq!(serde_json::to_value(&raw).unwrap(), "claim:evt-1:claim:0");

    let prefixed_raw_id = ClaimReference::parse("claim:claim:legacy").unwrap();
    assert_eq!(prefixed_raw_id.claim_id(), "claim:legacy");
    assert_eq!(prefixed_raw_id.canonical(), "claim:claim:legacy");
}

#[test]
fn invalid_claim_references_are_rejected() {
    for value in ["", "claim:", " claim:evt-1", "evt 1"] {
        assert_eq!(
            ClaimReference::parse(value).unwrap_err(),
            DomainError::InvalidClaimReference
        );
        assert!(
            serde_json::from_value::<ClaimReference>(serde_json::json!(value)).is_err(),
            "serde should reject invalid claim reference {value:?}"
        );
    }
}

#[test]
fn raw_and_prefixed_event_ids_share_one_canonical_reference() {
    let raw = EventReference::parse("evt-1").unwrap();
    let prefixed = EventReference::parse("event:evt-1").unwrap();

    assert_eq!(raw, prefixed);
    assert_eq!(raw.event_id(), "evt-1");
    assert_eq!(raw.canonical(), "event:evt-1");
    assert_eq!(serde_json::to_value(&raw).unwrap(), "event:evt-1");
    assert_eq!(
        serde_json::from_value::<EventReference>(serde_json::json!("event:evt-1")).unwrap(),
        raw
    );
}

#[test]
fn invalid_event_references_are_rejected() {
    for value in ["", "event:", " event:evt-1", "event:event:evt-1", "evt 1"] {
        assert_eq!(
            EventReference::parse(value).unwrap_err(),
            DomainError::InvalidEventReference
        );
        assert!(
            serde_json::from_value::<EventReference>(serde_json::json!(value)).is_err(),
            "serde should reject invalid event reference {value:?}"
        );
    }
}

#[test]
fn snapshot_summary_includes_supporting_event_reference() {
    let snapshot = build_snapshot(SnapshotRequest::fixture_minimal()).unwrap();
    assert!(!snapshot.evidence.is_empty());
}

#[test]
fn hard_commitment_blocks_conflicting_action() {
    let result = gate_decision(
        "write_identity_core_directly",
        &SnapshotRequest::fixture_minimal().commitments,
    );
    assert!(result.blocked);
}

#[test]
fn budget_of_zero_still_recalls_one_evidence_item() {
    let mut request = SnapshotRequest::fixture_minimal();
    request.budget = SnapshotBudget::new(0);

    let snapshot = build_snapshot(request).unwrap();

    assert_eq!(snapshot.evidence.len(), 1);
}

#[test]
fn snapshot_without_evidence_reuses_insufficient_evidence_error() {
    let mut request = SnapshotRequest::fixture_minimal();
    request.evidence.clear();

    let error = build_snapshot(request).unwrap_err();

    assert_eq!(error, DomainError::InsufficientEvidence);
}

#[test]
fn snapshot_budget_truncates_evidence_to_limit() {
    let mut request = SnapshotRequest::fixture_minimal();
    request.evidence = vec![
        "event:evt-1".to_string(),
        "event:evt-2".to_string(),
        "event:evt-3".to_string(),
    ];
    request.budget = SnapshotBudget::new(2);

    let snapshot = build_snapshot(request).unwrap();

    assert_eq!(
        snapshot.evidence,
        vec!["event:evt-1".to_string(), "event:evt-2".to_string()]
    );
}

#[test]
fn snapshot_time_window_accepts_equal_boundaries_and_rejects_reversed_bounds() {
    let instant = DateTime::parse_from_rfc3339("2026-07-11T03:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    assert!(SnapshotTimeWindow::new(Some(instant), Some(instant)).is_ok());
    assert_eq!(
        SnapshotTimeWindow::new(Some(instant + chrono::Duration::seconds(1)), Some(instant),)
            .unwrap_err(),
        DomainError::InvalidSnapshotTimeWindow
    );
}

#[test]
fn unrelated_action_is_not_blocked_by_commitment_gate() {
    let result = gate_decision(
        "read_identity_core",
        &SnapshotRequest::fixture_minimal().commitments,
    );

    assert!(!result.blocked);
}
