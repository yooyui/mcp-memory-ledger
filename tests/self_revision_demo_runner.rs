use std::{fs, process::Command};

use serde_json::Value;

#[test]
fn demo_runner_writes_expected_artifacts_and_proves_decision_shift() {
    let output_dir = tempfile::tempdir().expect("tempdir");

    let status = Command::new(env!("CARGO_BIN_EXE_run_self_revision_demo"))
        .arg("--output-dir")
        .arg(output_dir.path())
        .arg("--server-bin")
        .arg(env!("CARGO_BIN_EXE_agent_llm_mm"))
        .status()
        .expect("run demo runner");

    assert!(status.success());

    for name in [
        "doctor.json",
        "snapshot-before.json",
        "snapshot-after.json",
        "decision-before.json",
        "decision-after.json",
        "timeline.json",
        "sqlite-summary.json",
        "report.md",
    ] {
        assert!(
            output_dir.path().join(name).exists(),
            "missing demo artifact: {name}"
        );
    }

    let doctor: Value = read_json(
        output_dir
            .path()
            .join("doctor.json")
            .to_string_lossy()
            .as_ref(),
    );
    assert_eq!(doctor["self_revision_write_path"], "run_reflection");

    let snapshot_before: Value = read_json(
        output_dir
            .path()
            .join("snapshot-before.json")
            .to_string_lossy()
            .as_ref(),
    );
    let snapshot_after: Value = read_json(
        output_dir
            .path()
            .join("snapshot-after.json")
            .to_string_lossy()
            .as_ref(),
    );

    assert!(
        snapshot_before["commitments"]
            .as_array()
            .expect("before commitments")
            .iter()
            .any(|value| value == "forbid:write_identity_core_directly")
    );
    assert!(
        snapshot_after["commitments"]
            .as_array()
            .expect("after commitments")
            .iter()
            .any(|value| value == "prefer:confirm_conflicting_commitment_updates_before_overwrite")
    );

    let decision_before: Value = read_json(
        output_dir
            .path()
            .join("decision-before.json")
            .to_string_lossy()
            .as_ref(),
    );
    let decision_after: Value = read_json(
        output_dir
            .path()
            .join("decision-after.json")
            .to_string_lossy()
            .as_ref(),
    );

    assert_eq!(decision_before["blocked"], false);
    assert_eq!(decision_after["blocked"], false);
    assert_eq!(
        decision_before["decision"]["action"],
        "apply_commitment_update_now"
    );
    assert_eq!(
        decision_after["decision"]["action"],
        "confirm_conflicting_commitment_updates_before_overwrite"
    );

    let sqlite_summary: Value = read_json(
        output_dir
            .path()
            .join("sqlite-summary.json")
            .to_string_lossy()
            .as_ref(),
    );
    assert_eq!(
        sqlite_summary["reflection_trigger_ledger"][0]["status"],
        "handled"
    );
    assert_eq!(
        sqlite_summary["reflection_trigger_ledger"][0]["trigger_type"],
        "conflict"
    );

    let timeline: Value = read_json(
        output_dir
            .path()
            .join("timeline.json")
            .to_string_lossy()
            .as_ref(),
    );
    assert!(
        timeline["baseline"]["event_reference"]
            .as_str()
            .is_some_and(|value| value.starts_with("event:") && value.len() > "event:".len()),
        "external demo artifact references use canonical event:<id> form"
    );
    assert_eq!(timeline["baseline"]["event_id"], Value::Null);
    assert_eq!(timeline["gate_before"]["blocked"], true);
    assert_eq!(timeline["negative_conflict"]["handled_conflict_rows"], 0);
    assert_eq!(timeline["positive_conflict"]["handled_conflict_rows"], 1);
}

#[test]
fn shell_wrapper_runs_demo_runner_and_writes_report() {
    let output_dir = tempfile::tempdir().expect("tempdir");

    let output = Command::new("bash")
        .arg("scripts/run-self-revision-demo.sh")
        .arg(output_dir.path())
        .output()
        .expect("run shell wrapper");

    assert!(output.status.success(), "{output:?}");
    assert!(output_dir.path().join("report.md").exists());
    assert!(String::from_utf8_lossy(&output.stdout).contains("self-revision demo artifacts"));
}

fn read_json(path: &str) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read json")).expect("parse json")
}
