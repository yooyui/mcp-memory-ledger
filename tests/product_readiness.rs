use std::fs;

use agent_llm_mm::support::product_readiness::{
    ProductReadinessOptions, remote_team_readiness_gate_from_inventory, summarize_product_readiness,
};
use agent_llm_mm::support::product_wording::{
    ClaimGateState, ProductClaimGuardInput, check_product_claims,
};
use agent_llm_mm::support::remote_team::{
    RemoteTeamCapability, RemoteTeamCapabilityInventory, RemoteTeamCapabilityState,
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn product_readiness_blocks_simulation_windows_and_missing_release_decision() {
    let temp_dir = tempdir().expect("temp dir");
    write_satisfied_product_smoke(temp_dir.path());
    write_first_run_simulation(temp_dir.path());
    write_support_bundle(temp_dir.path());

    let summary = summarize_product_readiness(ProductReadinessOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        release_candidate: "local-alpha-20260524.1-rc.1".to_string(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("product readiness summary should be generated");

    assert!(!summary.ready);
    assert_eq!(summary.overall_status, "blocked");
    assert_gate(
        &summary,
        "real_fresh_machine",
        "blocked",
        "real fresh-machine evidence is false",
    );
    assert_gate(
        &summary,
        "windows_parity",
        "blocked",
        "missing Windows runtime parity evidence",
    );
    assert_gate(
        &summary,
        "release_decision",
        "blocked",
        "missing release decision summary",
    );
    assert_gate(
        &summary,
        "release_engineering",
        "blocked",
        "missing source-only release engineering artifacts",
    );
    assert_gate(
        &summary,
        "product_wording",
        "satisfied",
        "candidate wording contains no blocked product claims",
    );
    assert_gate(
        &summary,
        "remote_team",
        "blocked",
        "remote/team capability inventory keeps remote and team features blocked",
    );
    assert_gate(
        &summary,
        "security_auth",
        "blocked",
        "blocked security gates",
    );
    assert!(
        summary
            .non_claims
            .contains(&"not Local Alpha certification".to_string())
    );
    assert!(
        summary.non_claims.contains(
            &"not physics-informed runtime / solver / controller / scientific validation evidence"
                .to_string()
        )
    );

    let summary_json = serde_json::to_value(&summary).expect("summary serializes");
    assert_structured_blocker(
        &summary_json,
        "external_blockers",
        "fresh_machine",
        "blocked",
        "real fresh-machine evidence is false",
    );
    assert_structured_blocker(
        &summary_json,
        "external_blockers",
        "windows_parity",
        "blocked",
        "missing Windows runtime parity evidence",
    );
    assert_structured_blocker(
        &summary_json,
        "human_blockers",
        "release_decision",
        "blocked",
        "missing release decision summary",
    );
    assert_structured_blocker(
        &summary_json,
        "unimplemented_capability_blockers",
        "remote_team",
        "blocked",
        "remote/team capability inventory keeps remote and team features blocked",
    );
    assert_structured_blocker(
        &summary_json,
        "unimplemented_capability_blockers",
        "security_auth",
        "blocked",
        "blocked security gates",
    );
    assert_structured_blocker(
        &summary_json,
        "unimplemented_capability_blockers",
        "daemon_writes",
        "blocked",
        "daemon write capability remains disabled",
    );
}

#[test]
fn product_readiness_blocks_overstated_release_candidate_wording() {
    let temp_dir = tempdir().expect("temp dir");
    write_satisfied_product_smoke(temp_dir.path());
    write_first_run_simulation(temp_dir.path());
    write_support_bundle(temp_dir.path());

    let summary = summarize_product_readiness(ProductReadinessOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        release_candidate: "ga-production-ready-remote-team".to_string(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("product readiness summary should be generated");

    assert!(!summary.ready);
    assert_gate(
        &summary,
        "product_wording",
        "blocked",
        "blocked product claims",
    );
}

#[test]
fn product_readiness_accepts_source_only_release_engineering_artifacts() {
    let temp_dir = tempdir().expect("temp dir");
    let release_candidate = "local-alpha-20260524.1-rc.1";
    write_satisfied_product_smoke(temp_dir.path());
    write_first_run_simulation(temp_dir.path());
    write_support_bundle(temp_dir.path());
    write_release_engineering_artifacts(temp_dir.path(), release_candidate);

    let summary = summarize_product_readiness(ProductReadinessOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        release_candidate: release_candidate.to_string(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("product readiness summary should be generated");

    assert_gate(
        &summary,
        "release_engineering",
        "satisfied",
        "source-only release engineering artifacts are present",
    );
    assert!(
        summary
            .unimplemented_capability_blockers
            .iter()
            .any(|blocker| blocker.subject == "release_packaging"),
        "release packaging must remain blocked even when source-only artifacts exist"
    );
}

#[test]
fn product_readiness_rejects_incomplete_release_boundary_blockers() {
    let temp_dir = tempdir().expect("temp dir");
    let release_candidate = "local-alpha-20260524.1-rc.1";
    write_satisfied_product_smoke(temp_dir.path());
    write_first_run_simulation(temp_dir.path());
    write_support_bundle(temp_dir.path());
    write_incomplete_release_engineering_artifacts(temp_dir.path(), release_candidate);

    let summary = summarize_product_readiness(ProductReadinessOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        release_candidate: release_candidate.to_string(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("product readiness summary should be generated");

    assert_gate(
        &summary,
        "release_engineering",
        "blocked",
        "remote/team blocker is missing",
    );
    assert_gate(
        &summary,
        "release_engineering",
        "blocked",
        "security/auth blocker is missing",
    );
    assert_gate(
        &summary,
        "release_engineering",
        "blocked",
        "daemon write blocker is missing",
    );
}

#[test]
fn product_readiness_keeps_rejected_or_deferred_release_decision_blocked() {
    for decision in ["rejected", "deferred"] {
        let temp_dir = tempdir().expect("temp dir");
        let release_candidate = format!("local-alpha-20260531.1-{decision}");
        write_release_decision_artifact(
            temp_dir.path(),
            &release_candidate,
            decision,
            Some("reviewer@example.test"),
            Some("Keep candidate source-only and do not publish."),
        );

        let summary = summarize_product_readiness(ProductReadinessOptions {
            evidence_root: temp_dir.path().to_path_buf(),
            release_candidate,
            output_json_path: None,
            output_markdown_path: None,
        })
        .expect("product readiness should summarize non-approval decisions");

        assert_gate(
            &summary,
            "release_decision",
            "blocked",
            "release decision is not approved",
        );
        assert!(
            summary
                .human_blockers
                .iter()
                .any(|blocker| blocker.subject == "release_decision"
                    && blocker.status == "blocked"),
            "product readiness should expose human release decision blocker"
        );
        assert!(!summary.ready);
    }
}

#[test]
fn product_readiness_rejects_contradictory_release_decision_artifacts() {
    for (field, value) in [
        ("approved", json!(false)),
        ("evidence_status", json!("in_progress")),
        ("kind", json!("not_release_decision")),
    ] {
        let temp_dir = tempdir().expect("temp dir");
        let release_candidate = format!("local-alpha-20260531.1-contradictory-{field}");
        write_release_decision_artifact(
            temp_dir.path(),
            &release_candidate,
            "approved",
            Some("reviewer@example.test"),
            Some("Keep candidate source-only and do not publish."),
        );
        let decision_path = temp_dir
            .path()
            .join("target/reports/releases")
            .join(&release_candidate)
            .join("release-decision.json");
        let mut artifact: serde_json::Value =
            serde_json::from_slice(&fs::read(&decision_path).expect("read decision artifact"))
                .expect("decision artifact json");
        artifact[field] = value;
        fs::write(
            &decision_path,
            serde_json::to_vec_pretty(&artifact).expect("json"),
        )
        .expect("write contradictory decision artifact");

        let summary = summarize_product_readiness(ProductReadinessOptions {
            evidence_root: temp_dir.path().to_path_buf(),
            release_candidate,
            output_json_path: None,
            output_markdown_path: None,
        })
        .expect("product readiness should summarize contradictory artifacts");

        assert_gate(&summary, "release_decision", "blocked", "release decision");
        assert!(!summary.ready);
    }
}

#[test]
fn product_readiness_rejects_approved_release_decision_with_open_artifact_blockers() {
    let temp_dir = tempdir().expect("temp dir");
    let release_candidate = "local-alpha-20260531.1-contradictory-open-gates";
    write_release_decision_artifact(
        temp_dir.path(),
        release_candidate,
        "approved",
        Some("reviewer@example.test"),
        Some("Keep candidate source-only and do not publish."),
    );
    let decision_path = temp_dir
        .path()
        .join("target/reports/releases")
        .join(release_candidate)
        .join("release-decision.json");
    let mut artifact: serde_json::Value =
        serde_json::from_slice(&fs::read(&decision_path).expect("read decision artifact"))
            .expect("decision artifact json");
    artifact["open_gates"] = json!([
        {
            "name": "windows_parity",
            "status": "not_verified",
            "reason": "missing Windows runtime parity evidence"
        }
    ]);
    artifact["external_blockers"] = json!([
        {
            "subject": "windows_parity",
            "status": "not_verified",
            "reason": "missing Windows runtime parity evidence"
        }
    ]);
    artifact["human_blockers"] = json!([
        {
            "subject": "release_approval",
            "status": "blocked",
            "reason": "human approval is still blocked"
        }
    ]);
    fs::write(
        &decision_path,
        serde_json::to_vec_pretty(&artifact).expect("json"),
    )
    .expect("write contradictory decision artifact");

    let summary = summarize_product_readiness(ProductReadinessOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        release_candidate: release_candidate.to_string(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("product readiness should summarize contradictory artifacts");

    assert_gate(
        &summary,
        "release_decision",
        "blocked",
        "release decision artifact still lists open gates",
    );
    assert_gate(
        &summary,
        "release_decision",
        "blocked",
        "release decision artifact still lists external blockers",
    );
    assert_gate(
        &summary,
        "release_decision",
        "blocked",
        "release decision artifact still lists human blockers",
    );
    assert!(!summary.ready);
}

#[test]
fn product_wording_blocks_physics_informed_capability_claims_without_gate() {
    for claimed_text in [
        "physics-informed runtime",
        "physics solver",
        "physics-informed solver",
        "physics informed controller",
        "constraint solver",
        "constraint optimizer",
        "physical controller",
        "scientific validation",
    ] {
        let report = check_product_claims(ProductClaimGuardInput {
            text: format!("Agent LLM MM provides {claimed_text}."),
            gate_state: ClaimGateState::default(),
        });

        assert!(
            !report.allowed,
            "{claimed_text:?} should be blocked as a product capability claim"
        );
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.claim == "physics_informed_runtime"),
            "{claimed_text:?} should report physics_informed_runtime claim; got {:?}",
            report.violations
        );
    }
}

#[test]
fn product_wording_blocks_local_alpha_and_future_product_claims_without_gates() {
    let cases = [
        ("Local Alpha complete", "local_alpha_complete"),
        ("full Local Product Alpha release", "local_alpha_complete"),
        ("full Local Alpha release complete", "local_alpha_complete"),
        ("local-alpha-complete", "local_alpha_complete"),
        ("multi-tenant service", "multi_tenancy"),
        ("multi-tenancy", "multi_tenancy"),
        ("write-capable daemon service", "daemon_writes"),
        ("daemon writes", "daemon_writes"),
        (
            "all-entry automatic self-revision",
            "all_entry_auto_reflection",
        ),
        ("replace run_reflection", "run_reflection_replacement"),
        ("replaces run_reflection", "run_reflection_replacement"),
        ("provider gateway", "provider_gateway"),
        ("provider-gateway", "provider_gateway"),
        ("model-gateway", "provider_gateway"),
        ("openrouter-live-provider-certification", "provider_gateway"),
    ];

    for (claimed_text, expected_claim) in cases {
        let report = check_product_claims(ProductClaimGuardInput {
            text: format!("Agent LLM MM is a {claimed_text}."),
            gate_state: ClaimGateState::default(),
        });

        assert!(
            !report.allowed,
            "{claimed_text:?} should be blocked as a future product claim"
        );
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.claim == expected_claim),
            "{claimed_text:?} should report {expected_claim} claim; got {:?}",
            report.violations
        );
    }
}

#[test]
fn product_readiness_blocks_physics_informed_release_candidate_wording() {
    let temp_dir = tempdir().expect("temp dir");
    write_satisfied_product_smoke(temp_dir.path());
    write_first_run_simulation(temp_dir.path());
    write_support_bundle(temp_dir.path());

    let summary = summarize_product_readiness(ProductReadinessOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        release_candidate: "physics-informed-runtime-rc.1".to_string(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("product readiness summary should be generated");

    assert!(!summary.ready);
    assert_gate(
        &summary,
        "product_wording",
        "blocked",
        "physics_informed_runtime",
    );
}

#[test]
fn product_readiness_blocks_multi_tenant_release_candidate_wording() {
    let temp_dir = tempdir().expect("temp dir");
    write_satisfied_product_smoke(temp_dir.path());
    write_first_run_simulation(temp_dir.path());
    write_support_bundle(temp_dir.path());

    let summary = summarize_product_readiness(ProductReadinessOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        release_candidate: "multi-tenant-service-rc.1".to_string(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("product readiness summary should be generated");

    assert!(!summary.ready);
    assert_gate(&summary, "product_wording", "blocked", "multi_tenancy");
}

#[test]
fn product_readiness_blocks_local_alpha_and_daemon_write_release_candidate_wording() {
    let temp_dir = tempdir().expect("temp dir");
    write_satisfied_product_smoke(temp_dir.path());
    write_first_run_simulation(temp_dir.path());
    write_support_bundle(temp_dir.path());

    for (release_candidate, expected_claim) in [
        ("local-alpha-complete-rc.1", "local_alpha_complete"),
        ("write-capable-daemon-service-rc.1", "daemon_writes"),
    ] {
        let summary = summarize_product_readiness(ProductReadinessOptions {
            evidence_root: temp_dir.path().to_path_buf(),
            release_candidate: release_candidate.to_string(),
            output_json_path: None,
            output_markdown_path: None,
        })
        .expect("product readiness summary should be generated");

        assert!(!summary.ready);
        assert_gate(&summary, "product_wording", "blocked", expected_claim);
    }
}

#[test]
fn product_readiness_rejects_unsafe_release_candidate_names_before_path_lookup() {
    let temp_dir = tempdir().expect("temp dir");
    write_satisfied_product_smoke(temp_dir.path());
    write_first_run_simulation(temp_dir.path());
    write_support_bundle(temp_dir.path());

    let error = summarize_product_readiness(ProductReadinessOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        release_candidate: "../local-alpha".to_string(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect_err("unsafe candidate names should be rejected");

    assert!(
        error
            .to_string()
            .contains("candidate name must contain only letters"),
        "unexpected error: {error}"
    );
}

#[test]
fn product_readiness_script_exposes_candidate_evidence_gate() {
    let script =
        fs::read_to_string("scripts/product-readiness-check.sh").expect("script should exist");
    let mode = fs::metadata("scripts/product-readiness-check.sh")
        .expect("script metadata")
        .permissions();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(
            mode.mode() & 0o111,
            0,
            "script should be directly executable"
        );
    }

    assert!(script.contains("usage: ./scripts/product-readiness-check.sh <release-candidate>"));
    assert!(script.contains("candidate name must contain only letters"));
    assert!(
        script.contains(
            "cargo run --quiet --features release-tools --bin product_readiness_check --"
        )
    );
    assert!(script.contains("--release-candidate"));
    assert!(
        !script.contains(" ssh "),
        "product readiness check must not call ssh"
    );
    assert!(
        !script.contains(" scp "),
        "product readiness check must not call scp"
    );
    assert!(
        !script.contains(" rsync "),
        "product readiness check must not call rsync"
    );
}

#[test]
fn remote_team_readiness_gate_stays_blocked_when_inventory_exposes_write_or_upload_paths() {
    let gate = remote_team_readiness_gate_from_inventory(&RemoteTeamCapabilityInventory {
        local_only: false,
        support_bundle_upload_available: true,
        capabilities: vec![RemoteTeamCapability {
            name: "remote_write_admin",
            state: RemoteTeamCapabilityState::NotImplemented,
            write_capable_route_exposed: true,
            blocker: "test inventory exposed a write route",
        }],
    });

    assert_eq!(gate.name, "remote_team");
    assert_eq!(gate.status, "blocked");
    assert!(
        gate.reason.contains("support bundle upload is available"),
        "reason should preserve upload blocker detail: {}",
        gate.reason
    );
    assert!(
        gate.reason
            .contains("write-capable remote/team routes are exposed"),
        "reason should preserve write-route blocker detail: {}",
        gate.reason
    );
    assert!(
        gate.reason
            .contains("remote/team capability state is not blocked"),
        "reason should preserve unblocked capability detail: {}",
        gate.reason
    );
}

fn assert_gate(
    summary: &agent_llm_mm::support::product_readiness::ProductReadinessSummary,
    name: &str,
    status: &str,
    reason_contains: &str,
) {
    let gate = summary
        .gates
        .iter()
        .find(|gate| gate.name == name)
        .unwrap_or_else(|| panic!("missing gate {name}; gates={:?}", summary.gates));

    assert_eq!(gate.status, status);
    assert!(
        gate.reason.contains(reason_contains),
        "gate {name} reason should contain {reason_contains:?}; got {:?}",
        gate.reason
    );
}

fn assert_structured_blocker(
    summary: &serde_json::Value,
    field: &str,
    subject: &str,
    status: &str,
    reason_contains: &str,
) {
    let blockers = summary[field].as_array().expect("blockers array");
    let blocker = blockers
        .iter()
        .find(|blocker| blocker["subject"] == subject)
        .unwrap_or_else(|| panic!("missing blocker {subject} in {field}; blockers={blockers:?}"));

    assert_eq!(blocker["status"], status);
    let reason = blocker["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains(reason_contains),
        "blocker {subject} reason should contain {reason_contains:?}; got {reason:?}"
    );
}

fn write_satisfied_product_smoke(root: &std::path::Path) {
    let output_dir = root.join("target/reports/self-revision-demo/latest");
    fs::create_dir_all(&output_dir).expect("create product smoke dir");
    for file in [
        "doctor.json",
        "snapshot-before.json",
        "snapshot-after.json",
        "decision-before.json",
        "decision-after.json",
        "timeline.json",
        "sqlite-summary.json",
        "report.md",
    ] {
        fs::write(output_dir.join(file), "evidence").expect("write product smoke evidence");
    }
}

fn write_first_run_simulation(root: &std::path::Path) {
    let output_dir = root.join("target/first-run-bootstrap-smoke/local-alpha-gate");
    fs::create_dir_all(&output_dir).expect("create first-run dir");
    fs::write(
        output_dir.join("summary.json"),
        serde_json::to_vec_pretty(&json!({
            "kind": "local_first_run_bootstrap_simulation",
            "local_only": true,
            "fresh_machine_simulation": true,
            "real_fresh_machine_evidence": false,
            "doctor_status": "ok",
            "self_revision_write_path": "run_reflection",
            "daemon_enabled": false,
            "daemon_writes_allowed": false,
            "sqlite_database_exists": true,
            "started_serve": false,
            "ran_product_smoke": false
        }))
        .expect("json"),
    )
    .expect("write first-run summary");
}

fn write_support_bundle(root: &std::path::Path) {
    let output_dir = root.join("target/support-bundles/local-alpha-gate");
    fs::create_dir_all(&output_dir).expect("create support bundle dir");
    for (file, value) in [
        (
            "manifest.json",
            json!({
                "bundle_format": "agent-llm-mm-local-alpha-support-bundle-v1",
                "local_only": true,
                "upload_performed": false
            }),
        ),
        (
            "doctor.json",
            json!({
                "status": "config-shape-ok",
                "self_revision_write_path": "run_reflection",
                "runtime_bootstrap_performed": false
            }),
        ),
        (
            "product-smoke-summary.json",
            json!({
                "required_artifacts": [
                    "doctor.json",
                    "snapshot-before.json",
                    "snapshot-after.json",
                    "decision-before.json",
                    "decision-after.json",
                    "timeline.json",
                    "sqlite-summary.json",
                    "report.md"
                ],
                "self_revision_write_path_expected": "run_reflection"
            }),
        ),
        ("config-shape.json", json!({ "ok": true })),
        ("operation-summaries.json", json!({ "ok": true })),
        ("release-metadata.json", json!({ "ok": true })),
        ("local-log-excerpts.json", json!({ "ok": true })),
    ] {
        fs::write(
            output_dir.join(file),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .expect("write support bundle file");
    }
}

fn write_release_engineering_artifacts(root: &std::path::Path, candidate: &str) {
    let output_dir = root.join("target/reports/releases").join(candidate);
    fs::create_dir_all(&output_dir).expect("create release engineering dir");
    fs::write(
        output_dir.join("compatibility-matrix.json"),
        serde_json::to_vec_pretty(&json!({
            "kind": "release_compatibility_matrix",
            "candidate": candidate,
            "local_only": true,
            "rows": [
                {
                    "platform": "macOS",
                    "result": "passed"
                },
                {
                    "platform": "Windows",
                    "result": "not_checked",
                    "reason": "release-soak-local.sh does not create Windows runner evidence"
                }
            ]
        }))
        .expect("json"),
    )
    .expect("write compatibility matrix");
    fs::write(
        output_dir.join("release-boundaries.json"),
        serde_json::to_vec_pretty(&json!({
            "kind": "release_boundaries",
            "candidate": candidate,
            "local_only": true,
            "product_boundary": "local Rust MCP stdio memory MVP / technical demo entering productization",
            "external_blockers": [
                {
                    "subject": "fresh_machine",
                    "status": "blocked"
                },
                {
                    "subject": "windows_parity",
                    "status": "not_checked"
                }
            ],
            "human_blockers": [
                {
                    "subject": "release_decision",
                    "status": "required"
                }
            ],
            "unimplemented_capability_blockers": [
                {
                    "subject": "remote_team",
                    "status": "blocked"
                },
                {
                    "subject": "security_auth",
                    "status": "blocked"
                },
                {
                    "subject": "daemon_writes",
                    "status": "blocked"
                },
                {
                    "subject": "release_packaging",
                    "status": "blocked"
                }
            ]
        }))
        .expect("json"),
    )
    .expect("write release boundaries");
}

fn write_release_decision_artifact(
    root: &std::path::Path,
    candidate: &str,
    decision: &str,
    human_reviewer: Option<&str>,
    rollback_note: Option<&str>,
) {
    let output_dir = root.join("target/reports/releases").join(candidate);
    fs::create_dir_all(&output_dir).expect("create release decision dir");
    fs::write(
        output_dir.join("release-decision.json"),
        serde_json::to_vec_pretty(&json!({
            "kind": "release_decision",
            "release_candidate": candidate,
            "decision": decision,
            "approved": decision == "approved",
            "human_reviewer": human_reviewer,
            "rollback_note": rollback_note,
            "evidence_status": "ready_for_human_review"
        }))
        .expect("json"),
    )
    .expect("write release decision");
}

fn write_incomplete_release_engineering_artifacts(root: &std::path::Path, candidate: &str) {
    let output_dir = root.join("target/reports/releases").join(candidate);
    fs::create_dir_all(&output_dir).expect("create release engineering dir");
    fs::write(
        output_dir.join("compatibility-matrix.json"),
        serde_json::to_vec_pretty(&json!({
            "kind": "release_compatibility_matrix",
            "candidate": candidate,
            "local_only": true,
            "rows": [
                {
                    "platform": "macOS",
                    "result": "passed"
                },
                {
                    "platform": "Windows",
                    "result": "not_checked"
                }
            ]
        }))
        .expect("json"),
    )
    .expect("write compatibility matrix");
    fs::write(
        output_dir.join("release-boundaries.json"),
        serde_json::to_vec_pretty(&json!({
            "kind": "release_boundaries",
            "candidate": candidate,
            "local_only": true,
            "product_boundary": "local Rust MCP stdio memory MVP / technical demo entering productization",
            "external_blockers": [
                {
                    "subject": "fresh_machine",
                    "status": "blocked"
                },
                {
                    "subject": "windows_parity",
                    "status": "not_checked"
                }
            ],
            "human_blockers": [
                {
                    "subject": "release_decision",
                    "status": "required"
                }
            ],
            "unimplemented_capability_blockers": [
                {
                    "subject": "release_packaging",
                    "status": "blocked"
                }
            ]
        }))
        .expect("json"),
    )
    .expect("write release boundaries");
}
