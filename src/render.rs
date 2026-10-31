//! Pure renderers for the canonical traceability report.
//!
//! // @trace SPEC-CLI-OUTPUT, REQ-FMT-JSON, REQ-NFR-DETERM

use crate::graph::validator::ViolationSeverity;
use crate::report::Report;

/// Serializes a report as stable, pretty-printed JSON.
pub fn render_json(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

/// Renders the report summary using its configured layer names.
pub fn render_terminal_summary(report: &Report) -> String {
    if report.default_layer_names {
        return format!(
            "Total Nodes:          {}\n\
             Technical Specs:      {}\n\
             Requirements:         {}\n\
             Covered Reqs:         {}\n\
             Code Links:           {}\n\
             Test Links:           {}\n\
             Broken Links:         {}\n\
             Cycle Loops:          {}\n\
             Orphan Nodes:         {}\n\
             Coverage Rate:        {:.1}%\n\
             Status:               {}",
            report.statistics.total_nodes,
            report.statistics.specs_count,
            report.statistics.requirements_count,
            report.statistics.covered_nodes,
            report.statistics.code_links,
            report.statistics.test_links,
            report.statistics.broken_links,
            report.statistics.cycles_count,
            report.statistics.orphan_nodes,
            report.statistics.coverage_percent,
            if report.status == "pass" {
                "PASS"
            } else {
                "FAIL"
            },
        );
    }

    let spec_count = report
        .spec_layer_name
        .as_ref()
        .map(|name| format!("{name} Nodes:        {}\n", report.statistics.specs_count))
        .unwrap_or_default();
    format!(
        "Total Nodes:          {}\n\
         {} Nodes:        {}\n\
         Covered Nodes:        {}\n\
         {spec_count}\
         Code Links:           {}\n\
         Test Links:           {}\n\
         Broken Links:         {}\n\
         Cycle Loops:          {}\n\
         Orphan Nodes:         {}\n\
         Coverage Rate:        {:.1}%\n\
         Status:               {}",
        report.statistics.total_nodes,
        report.coverage_layer_name,
        report.statistics.requirements_count,
        report.statistics.covered_nodes,
        report.statistics.code_links,
        report.statistics.test_links,
        report.statistics.broken_links,
        report.statistics.cycles_count,
        report.statistics.orphan_nodes,
        report.statistics.coverage_percent,
        if report.status == "pass" {
            "PASS"
        } else {
            "FAIL"
        },
    )
}

/// Renders validation diagnostics for stderr.
pub fn render_diagnostics(report: &Report, strict: bool) -> String {
    let mut output = String::new();
    for violation in &report.violations {
        let label = match violation.severity(strict) {
            ViolationSeverity::Info => "INFO",
            ViolationSeverity::Warning => "WARN",
            ViolationSeverity::Error => "ERROR",
        };
        output.push_str(&format!("[{label}] {}\n", violation.message()));
    }
    output
}
