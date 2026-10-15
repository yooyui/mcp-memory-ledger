use std::{fs, os::unix::fs::PermissionsExt};

use agent_llm_mm::support::local_alpha_evidence::{
    LocalAlphaEvidenceOptions, summarize_local_alpha_evidence,
};
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn missing_required_evidence_artifacts_keep_gate_open() {
    let temp_dir = tempdir().expect("temp dir");
    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated from missing evidence");

    assert_eq!(summary.overall_status, "in_progress");
    assert_gate(
        &summary_json(&summary),
        "product_smoke",
        "open",
        "missing or empty product smoke latest evidence",
    );
    assert_gate(
        &summary_json(&summary),
        "first_run_bootstrap",
        "open",
        "missing first-run bootstrap summary",
    );
}

#[test]
fn gate_status_counts_match_gate_states_and_overall_status() {
    let temp_dir = tempdir().expect("temp dir");
    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated from missing evidence");

    // 派生计数三者之和必须等于 gate 总数，且与逐个 gate 状态精确吻合。
    let total =
        summary.satisfied_gate_count + summary.open_gate_count + summary.not_verified_gate_count;
    assert_eq!(total, summary.gates.len());
    assert_eq!(
        summary.satisfied_gate_count,
        summary
            .gates
            .iter()
            .filter(|g| g.status == "satisfied")
            .count()
    );
    assert_eq!(
        summary.open_gate_count,
        summary.gates.iter().filter(|g| g.status == "open").count()
    );

    // 缺失全部证据时没有 satisfied gate，存在 open gate，整体仍为 in_progress（非 ready_for_human_review）。
    assert_eq!(summary.satisfied_gate_count, 0);
    assert!(summary.open_gate_count > 0);
    assert_eq!(summary.overall_status, "in_progress");
}

#[test]
fn first_run_simulation_without_real_fresh_machine_evidence_cannot_complete_local_alpha() {
    let temp_dir = tempdir().expect("temp dir");
    write_first_run_summary(temp_dir.path(), false);
    write_product_smoke_latest(temp_dir.path());
    write_windows_parity(temp_dir.path(), "verified");
    write_support_bundle(temp_dir.path(), &allowed_support_bundle_files());

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");
    let summary_json = summary_json(&summary);

    assert_ne!(summary.overall_status, "complete");
    assert_eq!(summary.overall_status, "in_progress");
    assert_gate(
        &summary_json,
        "first_run_bootstrap",
        "open",
        "real fresh-machine evidence is false",
    );
    assert_gate(
        &summary_json,
        "first_run_simulation",
        "satisfied",
        "local first-run bootstrap simulation evidence is present",
    );
    assert!(
        summary.markdown.contains("Local Alpha is not complete"),
        "markdown must use conservative wording"
    );
}

#[test]
fn missing_windows_parity_is_not_verified() {
    let temp_dir = tempdir().expect("temp dir");
    write_first_run_summary(temp_dir.path(), true);
    write_product_smoke_latest(temp_dir.path());
    write_support_bundle(temp_dir.path(), &allowed_support_bundle_files());

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");
    let summary_json = summary_json(&summary);

    assert_gate(
        &summary_json,
        "windows_parity",
        "not_verified",
        "missing Windows runtime parity evidence",
    );
    assert_ne!(summary.overall_status, "complete");
}

#[test]
fn local_alpha_summary_exposes_machine_readable_external_and_human_blockers() {
    let temp_dir = tempdir().expect("temp dir");
    write_first_run_summary(temp_dir.path(), false);
    write_product_smoke_latest(temp_dir.path());
    write_support_bundle(temp_dir.path(), &allowed_support_bundle_files());

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");
    let summary_json = summary_json(&summary);

    assert_blocker(
        &summary_json,
        "external_blockers",
        "fresh_machine",
        "blocked",
        "real fresh-machine evidence is false",
    );
    assert_blocker(
        &summary_json,
        "external_blockers",
        "windows_parity",
        "not_verified",
        "missing Windows runtime parity evidence",
    );
    assert_blocker(
        &summary_json,
        "human_blockers",
        "release_decision",
        "required",
        "human release decision is required",
    );
}

#[test]
fn incomplete_first_run_summary_does_not_satisfy_read_only_boundary_gate() {
    let temp_dir = tempdir().expect("temp dir");
    let output_dir = temp_dir.path().join("first-run-bootstrap");
    fs::create_dir_all(&output_dir).expect("create first-run dir");
    fs::write(
        output_dir.join("summary.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "kind": "local_first_run_bootstrap_simulation"
        }))
        .expect("json"),
    )
    .expect("write incomplete first-run summary");

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");

    assert_gate(
        &summary_json(&summary),
        "local_read_only_boundary",
        "not_verified",
        "first-run boundary evidence is incomplete",
    );
}

#[test]
fn boundary_gate_requires_explicit_local_only_and_daemon_disabled_evidence() {
    let temp_dir = tempdir().expect("temp dir");
    let output_dir = temp_dir.path().join("first-run-bootstrap");
    fs::create_dir_all(&output_dir).expect("create first-run dir");
    fs::write(
        output_dir.join("summary.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "kind": "local_first_run_bootstrap_simulation",
            "started_serve": false,
            "ran_product_smoke": false,
            "self_revision_write_path": "run_reflection",
            "daemon_writes_allowed": false
        }))
        .expect("json"),
    )
    .expect("write incomplete first-run summary");

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");

    assert_gate(
        &summary_json(&summary),
        "local_read_only_boundary",
        "not_verified",
        "first-run boundary evidence is incomplete",
    );
}

#[test]
fn windows_parity_without_windows_runner_evidence_is_not_verified() {
    let temp_dir = tempdir().expect("temp dir");
    let output_dir = temp_dir.path().join("windows-parity");
    fs::create_dir_all(&output_dir).expect("create windows dir");
    fs::write(
        output_dir.join("summary.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "status": "verified",
            "runtime_parity": true
        }))
        .expect("json"),
    )
    .expect("write windows parity");

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");

    assert_gate(
        &summary_json(&summary),
        "windows_parity",
        "not_verified",
        "missing Windows runner or platform evidence",
    );
}

#[test]
fn empty_product_smoke_artifact_keeps_gate_open() {
    let temp_dir = tempdir().expect("temp dir");
    write_product_smoke_latest(temp_dir.path());
    let empty_artifact = temp_dir
        .path()
        .join("target/reports/self-revision-demo/latest/report.md");
    fs::write(empty_artifact, "").expect("empty report artifact");

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");

    assert_gate(
        &summary_json(&summary),
        "product_smoke",
        "open",
        "missing or empty product smoke latest evidence",
    );
}

#[test]
fn support_bundle_with_only_allowed_diagnostic_files_is_satisfied() {
    let temp_dir = tempdir().expect("temp dir");
    write_support_bundle(temp_dir.path(), &allowed_support_bundle_files());

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");

    assert_gate(
        &summary_json(&summary),
        "support_bundle",
        "satisfied",
        "allowed local diagnostic files present",
    );
}

#[test]
fn support_bundle_placeholder_files_do_not_satisfy_gate() {
    let temp_dir = tempdir().expect("temp dir");
    write_placeholder_support_bundle(temp_dir.path(), &allowed_support_bundle_files());

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");

    assert_gate(
        &summary_json(&summary),
        "support_bundle",
        "open",
        "support bundle manifest is not valid local alpha evidence",
    );
}

#[test]
fn satisfied_evidence_keeps_overall_review_ready_not_automatically_complete() {
    let temp_dir = tempdir().expect("temp dir");
    write_first_run_summary(temp_dir.path(), true);
    write_first_run_summary_at(
        &temp_dir
            .path()
            .join("target/first-run-bootstrap-smoke/local-alpha-gate"),
        false,
    );
    write_product_smoke_latest(temp_dir.path());
    write_windows_parity(temp_dir.path(), "verified");
    write_support_bundle(temp_dir.path(), &allowed_support_bundle_files());

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");

    assert_eq!(summary.overall_status, "ready_for_human_review");
    assert!(
        summary
            .markdown
            .contains("human release decision is still required"),
        "markdown must not imply automatic Local Alpha certification"
    );
}

#[test]
fn summary_reads_documented_target_evidence_paths_by_default() {
    let temp_dir = tempdir().expect("temp dir");
    write_first_run_summary_at(
        &temp_dir
            .path()
            .join("target/first-run-bootstrap-smoke/local-alpha-gate"),
        true,
    );
    write_product_smoke_latest(temp_dir.path());
    write_windows_parity_at(
        &temp_dir
            .path()
            .join("target/windows-parity/local-alpha-gate"),
        "verified",
    );
    write_support_bundle_at(
        &temp_dir
            .path()
            .join("target/support-bundles/local-alpha-gate"),
        &allowed_support_bundle_files(),
    );

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");
    let summary_json = summary_json(&summary);

    assert_gate(
        &summary_json,
        "first_run_bootstrap",
        "satisfied",
        "real fresh-machine first-run evidence is present and local-only",
    );
    assert_gate(
        &summary_json,
        "support_bundle",
        "satisfied",
        "allowed local diagnostic files present",
    );
    assert_gate(
        &summary_json,
        "windows_parity",
        "satisfied",
        "Windows runtime parity evidence is verified",
    );
}

#[test]
fn first_run_simulation_prefers_documented_simulation_path_when_real_evidence_exists() {
    let temp_dir = tempdir().expect("temp dir");
    write_real_first_run_summary_at(&temp_dir.path().join("first-run-bootstrap"));
    write_first_run_summary_at(
        &temp_dir
            .path()
            .join("target/first-run-bootstrap-smoke/local-alpha-gate"),
        false,
    );
    write_product_smoke_latest(temp_dir.path());
    write_windows_parity(temp_dir.path(), "verified");
    write_support_bundle(temp_dir.path(), &allowed_support_bundle_files());

    let summary = summarize_local_alpha_evidence(LocalAlphaEvidenceOptions {
        evidence_root: temp_dir.path().to_path_buf(),
        output_json_path: None,
        output_markdown_path: None,
    })
    .expect("summary should be generated");
    let summary_json = summary_json(&summary);

    assert_gate(
        &summary_json,
        "first_run_bootstrap",
        "satisfied",
        "real fresh-machine first-run evidence is present and local-only",
    );
    assert_gate(
        &summary_json,
        "first_run_simulation",
        "satisfied",
        "local first-run bootstrap simulation evidence is present",
    );
}

#[test]
fn summary_capability_is_read_only_and_does_not_call_runtime_write_or_remote_paths() {
    let script = fs::read_to_string("scripts/local-alpha-evidence-summary.sh")
        .expect("summary wrapper should be readable");
    let binary = fs::read_to_string("src/bin/local_alpha_evidence_summary.rs")
        .expect("summary binary should be readable");
    let module = fs::read_to_string("src/support/local_alpha_evidence.rs")
        .expect("summary module should be readable");
    let combined = format!("{script}\n{binary}\n{module}");

    assert!(
        !combined.contains("agent-llm-mm.sh serve") && !combined.contains(" serve "),
        "summary capability must not start serve"
    );
    assert!(
        !combined.contains("product-smoke-local.sh"),
        "summary capability must not execute product smoke"
    );
    assert!(
        !combined.contains("run-self-revision-demo.sh"),
        "summary capability must not execute demo smoke"
    );
    assert!(
        !combined.contains(" ssh "),
        "summary capability must not call ssh"
    );
    assert!(
        !combined.contains(" scp "),
        "summary capability must not call scp"
    );
    assert!(
        !combined.contains(" rsync "),
        "summary capability must not call rsync"
    );
    assert!(
        !combined.contains("daemon start"),
        "summary capability must not start daemon behavior"
    );
    assert!(
        !combined.contains("run_reflection(")
            && !combined.contains("AppCommand::Serve")
            && !combined.contains("run_command("),
        "summary capability must not invoke runtime serve/reflection write paths"
    );
}

#[test]
fn binary_rejects_missing_option_values_before_generating_summary() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_local_alpha_evidence_summary"))
        .arg("--output-json")
        .output()
        .expect("summary binary should run");

    assert!(
        !output.status.success(),
        "missing option value should fail instead of silently falling back"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing value for --output-json"),
        "stderr should name the missing option value"
    );
}

#[test]
fn release_gate_refresh_script_runs_local_gates_without_faking_external_evidence() {
    let script = fs::read_to_string("scripts/local-alpha-release-gate-refresh.sh")
        .expect("release gate refresh script should exist");
    let mode = fs::metadata("scripts/local-alpha-release-gate-refresh.sh")
        .expect("script metadata")
        .permissions()
        .mode();

    assert!(script.contains("scripts/product-smoke-local.sh"));
    assert!(script.contains("scripts/first-run-bootstrap-smoke-local.sh"));
    assert!(script.contains("scripts/generate-support-bundle.sh"));
    assert!(script.contains("scripts/local-alpha-evidence-summary.sh"));
    assert!(script.contains("target/first-run-bootstrap-smoke/local-alpha-gate"));
    assert!(script.contains("target/support-bundles/local-alpha-gate"));
    assert!(script.contains("target/reports/local-alpha/evidence-summary.json"));
    assert!(script.contains("target/reports/local-alpha/evidence-summary.md"));
    assert!(script.contains("--output-json"));
    assert!(script.contains("--output-md"));
    assert!(
        !script.contains("target/windows-parity/local-alpha-gate"),
        "local refresh script must not manufacture Windows runner evidence"
    );
    assert!(
        !script.contains("real_fresh_machine_evidence\": true"),
        "local refresh script must not manufacture real fresh-machine evidence"
    );
    assert!(
        !script.contains(" ssh "),
        "refresh script must not call ssh"
    );
    assert!(
        !script.contains(" scp "),
        "refresh script must not call scp"
    );
    assert!(
        !script.contains(" rsync "),
        "refresh script must not call rsync"
    );
    assert_ne!(mode & 0o111, 0, "script should be directly executable");
}

#[test]
fn release_gate_refresh_script_rejects_extra_args_before_running_gates() {
    let output = std::process::Command::new("bash")
        .args(["scripts/local-alpha-release-gate-refresh.sh", "one", "two"])
        .output()
        .expect("release gate refresh script should run");

    assert!(
        !output.status.success(),
        "extra args must fail before running gate commands\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("usage: ./scripts/local-alpha-release-gate-refresh.sh [config_path]"),
        "stderr should print usage for extra args"
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("product smoke:"),
        "argument validation must happen before product smoke runs"
    );
}

#[test]
fn release_soak_script_captures_local_evidence_without_remote_or_write_claims() {
    let script =
        fs::read_to_string("scripts/release-soak-local.sh").expect("release soak script exists");
    let mode = fs::metadata("scripts/release-soak-local.sh")
        .expect("script metadata")
        .permissions()
        .mode();

    assert!(
        script.contains("usage: ./scripts/release-soak-local.sh <candidate-name> [config_path]")
    );
    assert!(script.contains("target/reports/releases"));
    assert!(script.contains("target/release-soak-runtime"));
    assert!(script.contains("isolated_database_path"));
    assert!(script.contains("AGENT_LLM_MM_DATABASE_URL=${isolated_database_url}"));
    assert!(script.contains("formal_database_path_accepted"));
    assert!(script.contains("database-init"));
    assert!(script.contains("./scripts/agent-llm-mm.sh"));
    assert!(script.contains("doctor --read-only"));
    assert!(script.contains("cargo test --test dashboard_http -v"));
    assert!(script.contains("scripts/product-smoke-local.sh"));
    assert!(script.contains("scripts/generate-support-bundle.sh"));
    assert!(script.contains("scripts/local-alpha-evidence-summary.sh"));
    assert!(script.contains("secret-scan.log"));
    assert!(script.contains("artifact-scan.log"));
    assert!(script.contains("support-bundle-sha256.txt"));
    assert!(script.contains("product-smoke-latest-sha256.txt"));
    assert!(script.contains("shasum -a 256"));
    assert!(script.contains("sha256sum"));
    assert!(script.contains("release-soak-summary.md"));
    assert!(script.contains("compatibility-matrix.json"));
    assert!(script.contains("release-boundaries.json"));
    assert!(script.contains("redacted_config_path"));
    assert!(script.contains("<local-path>/<redacted-name>"));
    assert!(script.contains("config_path_shape"));
    assert!(script.contains("scanning release evidence for secret-like markers"));
    assert!(script.contains(r#"local_platform="macOS""#));
    assert!(script.contains(r#""platform": "${local_platform}""#));
    assert!(script.contains(r#""platform": "Windows""#));
    assert!(script.contains(r#""result": "not_checked""#));
    assert!(script.contains(r#""fresh_machine""#));
    assert!(script.contains(r#""daemon_writes""#));
    assert!(script.contains(r#""status": "blocked""#));
    assert!(script.contains("git rev-parse HEAD"));
    assert!(script.contains("git status --short --branch"));
    assert!(
        !script.contains("- config_path: `${resolved_config_path:-default config}`"),
        "release soak summary must not embed raw resolved config paths"
    );
    assert!(
        !script.contains("quote_command \"$@\" >> \"${log_path}\""),
        "release soak command logs must not persist raw argv with config paths"
    );
    assert!(
        !script.contains(" ssh "),
        "release soak script must not call ssh"
    );
    assert!(
        !script.contains(" scp "),
        "release soak script must not call scp"
    );
    assert!(
        !script.contains(" rsync "),
        "release soak script must not call rsync"
    );
    assert!(
        !script.contains("agent-llm-mm.sh serve"),
        "release soak script must not start the MCP service"
    );
    assert!(
        !script.contains("run_reflection("),
        "release soak script must not call durable self-revision writes directly"
    );
    assert_ne!(mode & 0o111, 0, "script should be directly executable");
}

#[test]
fn release_soak_script_rejects_unsafe_candidate_before_running_evidence_steps() {
    let output = std::process::Command::new("bash")
        .args(["scripts/release-soak-local.sh", "../bad"])
        .output()
        .expect("release soak script should run");

    assert_eq!(
        output.status.code(),
        Some(2),
        "unsafe candidate names should be rejected with usage error; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("candidate name"),
        "stderr should explain candidate-name safety; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("product smoke"),
        "argument validation must happen before product smoke runs"
    );
}

fn summary_json(
    summary: &agent_llm_mm::support::local_alpha_evidence::LocalAlphaEvidenceSummary,
) -> Value {
    serde_json::to_value(summary).expect("summary serializes")
}

fn assert_gate(summary: &Value, name: &str, status: &str, reason_contains: &str) {
    let gates = summary["gates"].as_array().expect("gates array");
    let gate = gates
        .iter()
        .find(|gate| gate["name"] == name)
        .unwrap_or_else(|| panic!("missing gate {name}; gates={gates:?}"));

    assert_eq!(
        gate["status"], status,
        "gate {name} status mismatch; gate={gate:?}"
    );
    let reason = gate["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains(reason_contains),
        "gate {name} reason should contain {reason_contains:?}; got {reason:?}"
    );
}

fn assert_blocker(
    summary: &Value,
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

fn write_first_run_summary(root: &std::path::Path, real_fresh_machine_evidence: bool) {
    let output_dir = root.join("first-run-bootstrap");
    write_first_run_summary_at(&output_dir, real_fresh_machine_evidence);
}

fn write_first_run_summary_at(output_dir: &std::path::Path, real_fresh_machine_evidence: bool) {
    fs::create_dir_all(output_dir).expect("create first-run dir");
    let mut summary = serde_json::json!({
        "local_only": true,
        "real_fresh_machine_evidence": real_fresh_machine_evidence,
        "doctor_status": "ok",
        "self_revision_write_path": "run_reflection",
        "daemon_enabled": false,
        "daemon_writes_allowed": false,
        "sqlite_database_exists": true,
        "started_serve": false,
        "ran_product_smoke": false
    });
    if real_fresh_machine_evidence {
        summary["kind"] = serde_json::json!("real_first_run_bootstrap_evidence");
        summary["evidence_kind"] = serde_json::json!("real_fresh_machine_first_run");
        summary["fresh_machine_simulation"] = serde_json::json!(false);
        summary["captured_at"] = serde_json::json!("2026-06-08T00:00:00Z");
        summary["source_checkout"] = serde_json::json!("clean-clone-or-unpacked-archive");
        summary["command_evidence"] = serde_json::json!([
            {
                "name": "bootstrap-local",
                "command": "./scripts/agent-llm-mm.sh bootstrap-local",
                "status": "passed",
                "exit_code": 0
            },
            {
                "name": "doctor",
                "command": "./scripts/agent-llm-mm.sh doctor",
                "status": "passed",
                "exit_code": 0
            }
        ]);
    } else {
        summary["kind"] = serde_json::json!("local_first_run_bootstrap_simulation");
        summary["fresh_machine_simulation"] = serde_json::json!(true);
    }
    fs::write(
        output_dir.join("summary.json"),
        serde_json::to_vec_pretty(&summary).expect("json"),
    )
    .expect("write first-run summary");
}

fn write_real_first_run_summary_at(output_dir: &std::path::Path) {
    fs::create_dir_all(output_dir).expect("create real first-run dir");
    fs::write(
        output_dir.join("summary.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "kind": "real_first_run_bootstrap_evidence",
            "evidence_kind": "real_fresh_machine_first_run",
            "captured_at": "2026-06-08T00:00:00Z",
            "source_checkout": "clean-clone-or-unpacked-archive",
            "command_evidence": [
                {
                    "name": "bootstrap-local",
                    "command": "./scripts/agent-llm-mm.sh bootstrap-local",
                    "status": "passed",
                    "exit_code": 0
                },
                {
                    "name": "doctor",
                    "command": "./scripts/agent-llm-mm.sh doctor",
                    "status": "passed",
                    "exit_code": 0
                }
            ],
            "local_only": true,
            "fresh_machine_simulation": false,
            "real_fresh_machine_evidence": true,
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
    .expect("write real first-run summary");
}

fn write_product_smoke_latest(root: &std::path::Path) {
    let output_dir = root
        .join("target")
        .join("reports")
        .join("self-revision-demo")
        .join("latest");
    fs::create_dir_all(&output_dir).expect("create latest dir");
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

fn write_windows_parity(root: &std::path::Path, status: &str) {
    let output_dir = root.join("windows-parity");
    write_windows_parity_at(&output_dir, status);
}

fn write_windows_parity_at(output_dir: &std::path::Path, status: &str) {
    fs::create_dir_all(output_dir).expect("create windows dir");
    fs::write(
        output_dir.join("summary.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "evidence_kind": "windows_runtime_parity",
            "captured_at": "2026-06-08T00:00:00Z",
            "status": status,
            "runner": "windows",
            "runtime_parity": true,
            "command_evidence": [
                {
                    "name": "bootstrap-local",
                    "command": "pwsh -File scripts/agent-llm-mm.ps1 bootstrap-local",
                    "status": "passed",
                    "exit_code": 0
                },
                {
                    "name": "doctor",
                    "command": "pwsh -File scripts/agent-llm-mm.ps1 doctor",
                    "status": "passed",
                    "exit_code": 0
                },
                {
                    "name": "product-smoke",
                    "command": "pwsh -File scripts/product-smoke-local.ps1",
                    "status": "passed",
                    "exit_code": 0
                }
            ]
        }))
        .expect("json"),
    )
    .expect("write windows parity");
}

fn write_support_bundle(root: &std::path::Path, files: &[&str]) {
    let output_dir = root.join("support-bundle");
    write_support_bundle_at(&output_dir, files);
}

fn write_support_bundle_at(output_dir: &std::path::Path, files: &[&str]) {
    fs::create_dir_all(output_dir).expect("create support bundle dir");
    for file in files {
        fs::write(output_dir.join(file), support_bundle_file_json(file))
            .expect("write support file");
    }
}

fn write_placeholder_support_bundle(root: &std::path::Path, files: &[&str]) {
    let output_dir = root.join("support-bundle");
    fs::create_dir_all(&output_dir).expect("create support bundle dir");
    for file in files {
        fs::write(output_dir.join(file), "{}").expect("write placeholder support file");
    }
}

fn allowed_support_bundle_files() -> Vec<&'static str> {
    vec![
        "manifest.json",
        "doctor.json",
        "config-shape.json",
        "operation-summaries.json",
        "release-metadata.json",
        "product-smoke-summary.json",
        "local-log-excerpts.json",
    ]
}

fn support_bundle_file_json(file: &str) -> Vec<u8> {
    let value = match file {
        "manifest.json" => serde_json::json!({
            "bundle_format": "agent-llm-mm-local-alpha-support-bundle-v1",
            "local_only": true,
            "upload_performed": false,
            "files": allowed_support_bundle_files()
        }),
        "doctor.json" => serde_json::json!({
            "status": "config-shape-ok",
            "self_revision_write_path": "run_reflection",
            "runtime_bootstrap_performed": false
        }),
        "product-smoke-summary.json" => serde_json::json!({
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
            "latest_artifacts_present": true,
            "self_revision_write_path_expected": "run_reflection"
        }),
        "operation-summaries.json" => serde_json::json!({
            "limit": 25,
            "available": false,
            "entries": []
        }),
        "local-log-excerpts.json" => serde_json::json!({
            "available": false,
            "read_only": true,
            "excerpts": []
        }),
        "config-shape.json" => serde_json::json!({
            "database_url_shape": "sqlite://<local-path>"
        }),
        "release-metadata.json" => serde_json::json!({
            "generated_at": "2026-05-16T00:00:00Z"
        }),
        other => panic!("unexpected support bundle file {other}"),
    };
    serde_json::to_vec_pretty(&value).expect("support bundle fixture json")
}
