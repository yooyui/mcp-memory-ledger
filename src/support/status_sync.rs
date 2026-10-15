pub const PLAN_STATUS_DOCUMENT: &str = "docs/plans/2026-07-10-product-replan.md";
pub const REALITY_GATES_DOCUMENT: &str = "docs/product/follow-up-reality-gates.md";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealityGateContradiction {
    pub workstream: String,
    pub reality_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRealityGate {
    pub workstream: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealityGateReport {
    pub completed_plan_items: Vec<String>,
    pub reality_gates: Vec<RealityGateDocumentRow>,
    pub contradictions: Vec<RealityGateContradiction>,
    pub missing_gates: Vec<MissingRealityGate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealityGateDocumentRow {
    pub workstream: String,
    pub status: String,
}

impl RealityGateReport {
    pub fn from_contents(plan_contents: &str, reality_gate_contents: &str) -> Self {
        let completed_plan_items = parse_completed_plan_items(plan_contents);
        let reality_gates = parse_reality_gate_rows(reality_gate_contents);
        let mut contradictions = Vec::new();
        let mut missing_gates = Vec::new();

        for item in &completed_plan_items {
            match matching_reality_gate(item, &reality_gates) {
                Some(row) if is_incomplete_reality_status(&row.status) => {
                    contradictions.push(RealityGateContradiction {
                        workstream: row.workstream.clone(),
                        reality_status: row.status.clone(),
                    });
                }
                Some(_) => {}
                None => missing_gates.push(MissingRealityGate {
                    workstream: item.clone(),
                }),
            }
        }

        Self {
            completed_plan_items,
            reality_gates,
            contradictions,
            missing_gates,
        }
    }

    pub fn is_in_sync(&self) -> bool {
        !self.completed_plan_items.is_empty()
            && self.contradictions.is_empty()
            && self.missing_gates.is_empty()
    }

    pub fn format_contradictions(&self) -> String {
        if self.is_in_sync() {
            return "plan/status reality gates are in sync".to_string();
        }

        let mut lines = Vec::new();
        if self.completed_plan_items.is_empty() {
            lines.push(
                "- active plan has no completed checkbox items; status sync would be vacuous"
                    .to_string(),
            );
        }
        lines.extend(self.contradictions.iter().map(|contradiction| {
            format!(
                "- {}: plan marks implemented but reality gate is {}",
                contradiction.workstream, contradiction.reality_status
            )
        }));
        lines.extend(self.missing_gates.iter().map(|missing| {
            format!(
                "- {}: plan marks implemented but no matching reality gate row exists",
                missing.workstream
            )
        }));
        lines.join("\n")
    }
}

fn parse_completed_plan_items(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with("- [x]") {
                return None;
            }
            let label = if let Some(start) = trimmed.find("**") {
                let rest = &trimmed[(start + 2)..];
                rest.find("**").map(|end| &rest[..end]).unwrap_or(rest)
            } else {
                trimmed.trim_start_matches("- [x]").trim()
            };
            Some(strip_priority_prefix(label).to_string())
        })
        .collect()
}

fn parse_reality_gate_rows(contents: &str) -> Vec<RealityGateDocumentRow> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('|') || trimmed.contains("| ---") {
                return None;
            }
            let cells = trimmed
                .trim_matches('|')
                .split('|')
                .map(|cell| clean_markdown_cell(cell.trim()))
                .collect::<Vec<_>>();
            if cells.len() < 3 {
                return None;
            }
            let status = cells[2].clone();
            if !is_known_reality_status(&status) {
                return None;
            }
            Some(RealityGateDocumentRow {
                workstream: cells[1].clone(),
                status,
            })
        })
        .collect()
}

fn matching_reality_gate<'a>(
    item: &str,
    rows: &'a [RealityGateDocumentRow],
) -> Option<&'a RealityGateDocumentRow> {
    rows.iter().find(|row| names_match(item, &row.workstream))
}

fn names_match(plan_item: &str, workstream: &str) -> bool {
    let plan = normalized_name(plan_item);
    let gate = normalized_name(workstream);
    plan == gate || plan.contains(&gate) || gate.contains(&plan)
}

fn normalized_name(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace("v2", "")
        .replace("gate checker", "")
        .replace("gate", "")
        .replace("artifact", "")
        .replace("template", "")
        .replace("first runtime slice", "")
        .replace("first slice", "")
        .replace("read-only inventory", "")
        .replace("contracts", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_priority_prefix(value: &str) -> &str {
    value
        .split_once(' ')
        .filter(|(prefix, _)| {
            prefix.len() == 4
                && prefix.starts_with('P')
                && prefix.as_bytes()[1].is_ascii_digit()
                && prefix.as_bytes()[2] == b'.'
                && prefix.as_bytes()[3].is_ascii_digit()
        })
        .map(|(_, rest)| rest.trim())
        .unwrap_or(value.trim())
}

fn clean_markdown_cell(cell: &str) -> String {
    cell.trim_matches('`').trim().to_string()
}

fn is_known_reality_status(status: &str) -> bool {
    matches!(
        status,
        "implemented"
            | "implemented-unmerged"
            | "partial"
            | "simulation-only"
            | "planning-gate"
            | "blocked claim"
            | "not-implemented"
    )
}

fn is_incomplete_reality_status(status: &str) -> bool {
    status != "implemented"
}
