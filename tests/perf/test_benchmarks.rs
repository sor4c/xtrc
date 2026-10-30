//! Quantitative Non-Functional Requirement (NFR) benchmark and verification suite.
//!
//! Verifies sub-second execution latency (`REQ-NFR-PERF`), bounded memory and 512-byte
//! binary header probing (`REQ-NFR-MEM`), linear $O(V + E)$ scalability and literal
//! pre-filtering throughput (`REQ-NFR-SCALE`), and deterministic multi-core output
//! (`REQ-NFR-DETERM`).
//!
//! // @trace:test SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, SPEC-CI-PERF
//! // @trace:test REQ-NFR-PERF, REQ-NFR-MEM, REQ-NFR-SCALE, REQ-NFR-DETERM, REQ-NFR-ROBUST, REQ-CI-PERF, US-PERF-01

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use xtrc::render::{render_json, render_terminal_summary};
use xtrc::Report;
use xtrc::{
    contains_trace_marker_bytes, detect_cycles, load_workspace_graph, scan_comments_file,
    scan_comments_text, validate_graph, FileOrigin, Layer, LayerModel, LinkKind, NodeId, TraceLink,
    TraceNode, TraceabilityGraph, ValidationOptions,
};

/// Helper to construct the `@trace` marker string dynamically in test fixtures so
/// `xtrc`'s self-check does not treat synthetic fixture strings as repository links.
fn trace_tag() -> &'static str {
    concat!("@", "trace")
}

/// Helper to construct the `@trace:test` marker string dynamically in test fixtures.
fn trace_test_tag() -> &'static str {
    concat!("@", "trace:test")
}

/// Scaffolds a realistic 1,000-file polyglot workspace with complete 7-layer traceability.
fn create_thousand_file_workspace(root: &Path) {
    let config_toml = r#"version = "1"

[project]
name = "perf-bench"
root = "."

[specs]
include = ["docs/**/*.md"]
exclude = ["target/**"]

[code]
include = ["src/**/*.*"]
extensions = [".rs", ".py", ".go", ".ts"]

[tests]
include = ["tests/**/*.*"]

[validation]
strict = true
require_test_coverage = true
"#;
    fs::write(root.join(".xtrc.toml"), config_toml).expect("write .xtrc.toml");

    let docs_dir = root.join("docs");
    let src_dir = root.join("src");
    let tests_dir = root.join("tests");
    fs::create_dir_all(&docs_dir).expect("create docs");
    fs::create_dir_all(&src_dir).expect("create src");
    fs::create_dir_all(&tests_dir).expect("create tests");

    // 1. Author specification documents (10 files, 200 nodes across Layers 1-6)
    for doc_idx in 1..=10 {
        let mut doc = format!(
            "---\ndocument_type: specification\nid: SPEC-BENCH-{doc_idx:03}\ntitle: Benchmark Spec {doc_idx}\nversion: 1.0.0\nstatus: approved\n---\n\n"
        );
        doc.push_str(&format!(
            "### VIS-BENCH-{doc_idx:02}: Vision {doc_idx}\n- **ID**: VIS-BENCH-{doc_idx:02}\n- **Parent**: None\n- **Type**: Vision\n\n"
        ));
        doc.push_str(&format!(
            "### STRAT-BENCH-{doc_idx:02}: Strategy {doc_idx}\n- **ID**: STRAT-BENCH-{doc_idx:02}\n- **Parent**: VIS-BENCH-{doc_idx:02}\n- **Type**: Strategy\n\n"
        ));
        doc.push_str(&format!(
            "### EPIC-BENCH-{doc_idx:02}: Epic {doc_idx}\n- **ID**: EPIC-BENCH-{doc_idx:02}\n- **Parent**: STRAT-BENCH-{doc_idx:02}\n- **Type**: Epic\n\n"
        ));
        doc.push_str(&format!(
            "### US-BENCH-{doc_idx:02}: User Story {doc_idx}\n- **ID**: US-BENCH-{doc_idx:02}\n- **Parent**: EPIC-BENCH-{doc_idx:02}\n- **Type**: User Story\n\n"
        ));

        for item in 1..=8 {
            let req_id = format!("REQ-BENCH-{doc_idx:02}-{item:02}");
            let spec_id = format!("SPEC-BENCH-{doc_idx:02}-{item:02}");
            doc.push_str(&format!(
                "### {req_id}: Requirement {doc_idx}.{item}\n- **ID**: {req_id}\n- **Parent**: US-BENCH-{doc_idx:02}\n- **Type**: Requirement\n\n"
            ));
            doc.push_str(&format!(
                "### {spec_id}: Technical Spec {doc_idx}.{item}\n- **ID**: {spec_id}\n- **Parent**: {req_id}\n- **Type**: TechnicalSpec\n\n"
            ));
        }

        fs::write(docs_dir.join(format!("spec_{doc_idx:02}.md")), doc).expect("write spec doc");
    }

    let tag = trace_tag();
    let test_tag = trace_test_tag();

    // 2. Author 600 source files (80 with trace markers, 520 non-matching source files)
    let extensions = ["rs", "py", "go", "ts"];
    for idx in 0..600 {
        let ext = extensions[idx % extensions.len()];
        let sub = src_dir.join(format!("mod_{:02}", idx / 50));
        fs::create_dir_all(&sub).expect("create src subdir");
        let file_path = sub.join(format!("file_{idx:04}.{ext}"));

        let content = if idx < 80 {
            let doc_idx = (idx / 8) + 1;
            let item = (idx % 8) + 1;
            let comment_prefix = if ext == "py" { "#" } else { "//" };
            format!(
                "{comment_prefix} {tag} SPEC-BENCH-{doc_idx:02}-{item:02}, REQ-BENCH-{doc_idx:02}-{item:02}\n\
                 pub fn compute_{idx}(x: usize) -> usize {{\n    x.wrapping_mul(42).wrapping_add({idx})\n}}\n"
            )
        } else {
            // Non-matching source file with realistic multi-line boilerplate
            format!(
                "// Standard module implementation {idx}\n\
                 pub struct Component{idx} {{ pub value: u64 }}\n\
                 impl Component{idx} {{\n    pub fn process(&self) -> u64 {{ self.value + {idx} }}\n}}\n"
            )
        };
        fs::write(file_path, content).expect("write src file");
    }

    // 3. Author 390 test files (80 with test trace markers, 310 helper/non-matching test files)
    for idx in 0..390 {
        let sub = tests_dir.join(format!("suite_{:02}", idx / 50));
        fs::create_dir_all(&sub).expect("create test subdir");
        let file_path = sub.join(format!("test_{idx:04}.rs"));

        let content = if idx < 80 {
            let doc_idx = (idx / 8) + 1;
            let item = (idx % 8) + 1;
            format!(
                "// {test_tag} SPEC-BENCH-{doc_idx:02}-{item:02}, REQ-BENCH-{doc_idx:02}-{item:02}\n\
                 #[test]\nfn verify_{idx}() {{ assert_eq!(2 + 2, 4); }}\n"
            )
        } else {
            format!("#[test]\nfn helper_assertion_{idx}() {{ assert!(true); }}\n")
        };
        fs::write(file_path, content).expect("write test file");
    }
}

// @trace:test SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, REQ-NFR-PERF, US-PERF-01
#[test]
fn test_nfr_perf_1000_file_workspace_sub_500ms() {
    let dir = tempdir().expect("create tempdir");
    create_thousand_file_workspace(dir.path());

    let start = Instant::now();
    let (_root, _config, graph) =
        load_workspace_graph(Some(dir.path()), None).expect("load 1000-file workspace graph");
    detect_cycles(&graph).expect("cycle detection must succeed");
    let report = validate_graph(
        &graph,
        &ValidationOptions {
            strict: true,
            require_test_coverage: true,
            layer_filter: None,
            id_filter: None,
            layer_model: LayerModel::default(),
        },
    );
    let elapsed = start.elapsed();

    assert!(
        report.is_success(true),
        "1,000-file benchmark workspace must pass strict validation: {:?}",
        report.violations
    );
    assert_eq!(report.statistics.requirements_count, 80);
    assert_eq!(report.statistics.covered_nodes, 80);
    assert_eq!(report.statistics.coverage_percent, 100.0);
    assert!(
        elapsed < Duration::from_millis(500),
        "REQ-NFR-PERF violation: 1,000-file full pipeline took {:?}, exceeding 500ms budget",
        elapsed
    );
}

// @trace:test SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, REQ-NFR-MEM, US-PERF-01
#[test]
fn test_nfr_mem_binary_header_probe_and_memory_bounds() {
    let dir = tempdir().expect("create tempdir");
    create_thousand_file_workspace(dir.path());

    // Write a 4 MiB binary asset containing a NUL byte in the first 512 bytes followed by
    // bogus trace strings that would cause broken links if the full binary were scanned.
    let bin_path = dir.path().join("src").join("large_asset.rs");
    let mut binary_payload = vec![b'A'; 4 * 1024 * 1024];
    binary_payload[128] = 0x00;
    let bogus = format!("\n// {} REQ-BOGUS-BINARY-SHOULD-NEVER-PARSE\n", trace_tag());
    binary_payload[1024..1024 + bogus.len()].copy_from_slice(bogus.as_bytes());
    fs::write(&bin_path, &binary_payload).expect("write 4MiB binary asset");

    let start = Instant::now();
    let links = scan_comments_file(&bin_path, Some(dir.path()), None)
        .expect("binary file scan should succeed cleanly");
    let probe_elapsed = start.elapsed();

    assert!(
        links.is_empty(),
        "REQ-NFR-MEM violation: binary file must be skipped via 512-byte header probe"
    );
    assert!(
        probe_elapsed < Duration::from_millis(25),
        "REQ-NFR-MEM violation: 512-byte binary header probe took {:?}, exceeding 25ms budget",
        probe_elapsed
    );

    // Verify full 1,000-file workspace graph metadata memory footprint stays well under 50 MiB
    let (_root, _config, graph) =
        load_workspace_graph(Some(dir.path()), None).expect("load workspace with binary asset");
    let estimated_graph_bytes: usize = graph
        .all_nodes()
        .map(|n| {
            std::mem::size_of::<TraceNode>()
                + n.id().as_str().len()
                + n.title().len()
                + n.description().len()
        })
        .sum::<usize>()
        + std::mem::size_of_val(graph.all_links());

    let max_budget_bytes = 50 * 1024 * 1024; // 50 MiB
    assert!(
        estimated_graph_bytes < max_budget_bytes,
        "REQ-NFR-MEM violation: graph metadata footprint {} bytes exceeds 50 MiB budget",
        estimated_graph_bytes
    );
}

// @trace:test SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, REQ-NFR-SCALE, US-PERF-01
#[test]
fn test_nfr_scale_linear_dag_validation_and_prefilter_throughput() {
    // 1. Test literal trace marker pre-filter throughput on a 1 MiB non-matching source file
    let one_mib_source =
        "pub fn compute_value(input: u64) -> u64 { input.wrapping_add(1) }\n".repeat(16_384);
    let prefilter_start = Instant::now();
    assert!(!contains_trace_marker_bytes(one_mib_source.as_bytes()));
    let extracted = scan_comments_text(&one_mib_source, Path::new("src/large.rs"), None, None);
    let prefilter_elapsed = prefilter_start.elapsed();

    assert!(extracted.is_empty());
    assert!(
        prefilter_elapsed < Duration::from_millis(50),
        "REQ-NFR-SCALE violation: 1 MiB literal pre-filter took {:?}, exceeding 50ms budget",
        prefilter_elapsed
    );

    // 2. Test linear O(V + E) graph validation and cycle detection on a 2,400-node DAG
    let mut nodes = Vec::with_capacity(2_404);
    let mut links = Vec::with_capacity(800);
    let origin = FileOrigin::new("docs/scale.md", 1);

    nodes.push(TraceNode::new(
        NodeId::new("VIS-SCALE-01").expect("id"),
        "Scale Vision",
        Layer::Vision,
        vec![],
        origin.clone(),
        "Root",
    ));
    nodes.push(TraceNode::new(
        NodeId::new("STRAT-SCALE-01").expect("id"),
        "Scale Strategy",
        Layer::Strategy,
        vec![NodeId::new("VIS-SCALE-01").expect("id")],
        origin.clone(),
        "Strategy",
    ));
    nodes.push(TraceNode::new(
        NodeId::new("EPIC-SCALE-01").expect("id"),
        "Scale Epic",
        Layer::Epic,
        vec![NodeId::new("STRAT-SCALE-01").expect("id")],
        origin.clone(),
        "Epic",
    ));
    nodes.push(TraceNode::new(
        NodeId::new("US-SCALE-01").expect("id"),
        "Scale Story",
        Layer::UserStory,
        vec![NodeId::new("EPIC-SCALE-01").expect("id")],
        origin.clone(),
        "Story",
    ));

    for i in 1..=1_200 {
        let req_id = NodeId::new(format!("REQ-SCALE-{i:04}")).expect("req id");
        let spec_id = NodeId::new(format!("SPEC-SCALE-{i:04}")).expect("spec id");
        nodes.push(TraceNode::new(
            req_id.clone(),
            format!("Requirement {i}"),
            Layer::Requirement,
            vec![NodeId::new("US-SCALE-01").expect("id")],
            origin.clone(),
            "Req body",
        ));
        nodes.push(TraceNode::new(
            spec_id.clone(),
            format!("Spec {i}"),
            Layer::TechnicalSpec,
            vec![req_id],
            origin.clone(),
            "Spec body",
        ));
        links.push(TraceLink::new(
            spec_id.clone(),
            "src/scale.rs",
            i,
            LinkKind::SourceCode,
        ));
        links.push(TraceLink::new(
            spec_id,
            "tests/scale_test.rs",
            i,
            LinkKind::AutomatedTest,
        ));
    }

    let dag_start = Instant::now();
    let graph = TraceabilityGraph::build(nodes, links).expect("build 2404-node graph");
    detect_cycles(&graph).expect("no cycles in 2404-node graph");
    let report = validate_graph(
        &graph,
        &ValidationOptions {
            strict: true,
            require_test_coverage: true,
            layer_filter: None,
            id_filter: None,
            layer_model: LayerModel::default(),
        },
    );
    let dag_elapsed = dag_start.elapsed();

    assert!(report.is_success(true));
    assert_eq!(report.statistics.total_nodes, 2_404);
    assert_eq!(report.statistics.coverage_percent, 100.0);
    assert!(
        dag_elapsed < Duration::from_millis(100),
        "REQ-NFR-SCALE violation: 2,404-node O(V+E) DAG build + validation took {:?}, exceeding 100ms budget",
        dag_elapsed
    );
}

// @trace:test SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, REQ-NFR-DETERM, US-PERF-01
#[test]
fn test_nfr_determ_reproducible_parallel_execution() {
    let dir = tempdir().expect("create tempdir");
    create_thousand_file_workspace(dir.path());

    let mut baseline_json: Option<String> = None;
    let mut baseline_summary: Option<String> = None;
    let mut baseline_links_len: Option<usize> = None;

    for iteration in 0..5 {
        let (_root, config, graph) =
            load_workspace_graph(Some(dir.path()), None).expect("parallel load");
        let validation = validate_graph(
            &graph,
            &ValidationOptions {
                strict: true,
                require_test_coverage: true,
                layer_filter: None,
                id_filter: None,
                layer_model: LayerModel::default(),
            },
        );
        let report = Report::from_graph(&graph, validation, "check", &config.layers);
        let json = render_json(&report).expect("serialize report json");
        let summary = render_terminal_summary(&report);
        let links_len = graph.all_links().len();

        if let Some(ref expected_json) = baseline_json {
            assert_eq!(
                &json, expected_json,
                "REQ-NFR-DETERM violation: JSON output differed on parallel iteration {iteration}"
            );
        } else {
            baseline_json = Some(json);
        }

        if let Some(ref expected_summary) = baseline_summary {
            assert_eq!(
                &summary, expected_summary,
                "REQ-NFR-DETERM violation: terminal summary differed on parallel iteration {iteration}"
            );
        } else {
            baseline_summary = Some(summary);
        }

        if let Some(expected_len) = baseline_links_len {
            assert_eq!(links_len, expected_len);
        } else {
            baseline_links_len = Some(links_len);
        }
    }
}

// @trace:test SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, SPEC-CI-PERF, REQ-NFR-PERF, REQ-CI-PERF, US-PERF-01
#[test]
fn test_nfr_self_repository_verification_sub_200ms() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let start = Instant::now();
    let (_root, _config, graph) =
        load_workspace_graph(Some(repo_root), None).expect("load xtrc self repository");
    detect_cycles(&graph).expect("xtrc repository must be cycle-free");
    let report = validate_graph(
        &graph,
        &ValidationOptions {
            strict: true,
            require_test_coverage: true,
            layer_filter: None,
            id_filter: None,
            layer_model: LayerModel::default(),
        },
    );
    let elapsed = start.elapsed();

    assert!(
        report.is_success(true),
        "Self-repository strict traceability check must pass 100%: {:?}",
        report.violations
    );
    assert!(
        elapsed < Duration::from_millis(200),
        "REQ-NFR-PERF violation: self-repository verification took {:?}, exceeding 200ms budget",
        elapsed
    );
}
