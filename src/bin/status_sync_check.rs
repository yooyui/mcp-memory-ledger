use std::{fs, process};

#[allow(dead_code)]
#[path = "../support/status_sync.rs"]
mod status_sync;

use status_sync::{PLAN_STATUS_DOCUMENT, REALITY_GATES_DOCUMENT, RealityGateReport};

fn main() {
    let plan_contents = read_document(PLAN_STATUS_DOCUMENT);
    let reality_gate_contents = read_document(REALITY_GATES_DOCUMENT);
    let report = RealityGateReport::from_contents(&plan_contents, &reality_gate_contents);

    if report.is_in_sync() {
        println!(
            "status sync ok: {} completed plan items match implemented reality gates",
            report.completed_plan_items.len()
        );
        return;
    }

    eprintln!(
        "plan/status reality gate drift detected:\n{}",
        report.format_contradictions()
    );
    process::exit(1);
}

fn read_document(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("failed to read {path}: {error}");
        process::exit(1);
    })
}
