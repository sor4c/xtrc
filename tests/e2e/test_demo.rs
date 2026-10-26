//! Integration and verification tests for the end-to-end demo workspace and reports.
//!
//! // @trace:test REQ-DEMO-SETUP, REQ-DEMO-RUNNER, REQ-DEMO-REPORTS, REQ-VER-BUMP-PATCH, SPEC-DEMO-WORKSPACE, SPEC-DEMO-REPORTS, SPEC-VER-BUMP

use std::path::Path;
use std::process::Command;

#[test]
// @trace:test REQ-DEMO-SETUP, SPEC-DEMO-WORKSPACE
fn test_demo_workspace_strict_check() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtrc"))
        .args(["--root", "examples/demo", "check", "--strict"])
        .output()
        .expect("execute xtrc check --strict on examples/demo");

    assert!(
        output.status.success(),
        "xtrc check --strict failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Coverage Rate:        100.0%"));
    assert!(stdout.contains("Status:               PASS"));
    assert!(stdout.contains("Total Nodes:          8"));
    assert!(stdout.contains("Covered Reqs:         2"));
    assert!(stdout.contains("Broken Links:         0"));
}

#[test]
// @trace:test REQ-DEMO-RUNNER, REQ-DEMO-REPORTS, SPEC-DEMO-REPORTS
fn test_demo_e2e_runner_script_and_reports() {
    let runner_path = Path::new("examples/demo/run-demo.sh");
    assert!(runner_path.is_file(), "run-demo.sh must exist");

    let xtrc_bin = env!("CARGO_BIN_EXE_xtrc");
    let status = Command::new("bash")
        .arg(runner_path)
        .arg("--xtrc-bin")
        .arg(xtrc_bin)
        .status()
        .expect("execute run-demo.sh");

    assert!(status.success(), "run-demo.sh must exit with code 0");

    let reports_dir = Path::new("examples/demo/reports");

    // 1. HTML Report Verification
    let html_path = reports_dir.join("traceability-report.html");
    assert!(html_path.is_file(), "traceability-report.html must exist");
    let html = std::fs::read_to_string(&html_path).expect("read HTML report");
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("Traceability Compliance Audit Report"));
    assert!(html.contains("VIS-DEMO-PAY"));
    assert!(html.contains("REQ-DEMO-AUTH"));
    assert!(html.contains("REQ-DEMO-CAPTURE"));
    assert!(html.contains(">PASS</span>") || html.contains("PASS"));

    // 2. JSON Report Verification
    let json_path = reports_dir.join("traceability-report.json");
    assert!(json_path.is_file(), "traceability-report.json must exist");
    let json_raw = std::fs::read_to_string(&json_path).expect("read JSON report");
    let json: serde_json::Value =
        serde_json::from_str(&json_raw).expect("JSON report must be valid JSON");
    assert_eq!(
        json.get("schema_version").and_then(|v| v.as_str()),
        Some("1.0.0")
    );
    assert_eq!(json.get("command").and_then(|v| v.as_str()), Some("export"));
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("pass"));
    let nodes = json
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("nodes array");
    assert!(nodes
        .iter()
        .any(|n| n.get("id").and_then(|v| v.as_str()) == Some("VIS-DEMO-PAY")));
    assert!(nodes
        .iter()
        .any(|n| n.get("id").and_then(|v| v.as_str()) == Some("REQ-DEMO-AUTH")));

    // 3. Markdown Report Verification
    let md_path = reports_dir.join("traceability-report.md");
    assert!(md_path.is_file(), "traceability-report.md must exist");
    let md = std::fs::read_to_string(&md_path).expect("read Markdown report");
    assert!(md.contains("# Traceability Compliance Audit Report"));
    assert!(md.contains("REQ-DEMO-AUTH"));
    assert!(md.contains("REQ-DEMO-CAPTURE"));

    // 4. CSV Report Verification
    let csv_path = reports_dir.join("traceability-report.csv");
    assert!(csv_path.is_file(), "traceability-report.csv must exist");
    let csv = std::fs::read_to_string(&csv_path).expect("read CSV report");
    assert!(csv.contains("REQ-DEMO-AUTH"));
    assert!(csv.contains("REQ-DEMO-CAPTURE"));

    // 5. Matrix Exports Verification
    let matrix_html_path = reports_dir.join("matrix.html");
    assert!(matrix_html_path.is_file(), "matrix.html must exist");
    let m_html = std::fs::read_to_string(&matrix_html_path).expect("read matrix.html");
    assert!(m_html.contains("<table") && m_html.contains("REQ-DEMO-AUTH"));

    let matrix_md_path = reports_dir.join("matrix.md");
    assert!(matrix_md_path.is_file(), "matrix.md must exist");
    let m_md = std::fs::read_to_string(&matrix_md_path).expect("read matrix.md");
    assert!(m_md.contains("REQ-DEMO-AUTH"));
}

#[test]
// @trace:test REQ-VER-BUMP-PATCH, SPEC-VER-BUMP
fn test_version_patch_bump_matches_0_1_2() {
    let output = Command::new("bash")
        .arg("scripts/compute-version.sh")
        .output()
        .expect("compute version");
    assert!(output.status.success());
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert!(
        version.starts_with("0.1.2"),
        "Computed version should start with 0.1.2, got: {version}"
    );
}
