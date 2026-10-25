#!/usr/bin/env bash
# @trace SPEC-CI-PERF, SPEC-PERF-BOUNDS, SPEC-PERF-HARNESS, REQ-CI-PERF, REQ-NFR-PERF, REQ-NFR-MEM, REQ-NFR-SCALE, REQ-NFR-DETERM, US-PERF-01, US-CICD-01
#
# Automated CI/CD Performance Measurement & NFR SLA Gate Harness for xtrc.
# Measures wall-clock latency across self-repository CLI operations and a
# synthetic 1,000-file polyglot workspace, emits a structured JSON report,
# and exits non-zero if any Non-Functional Requirement budget is exceeded.

set -euo pipefail

OUTPUT_JSON="${1:-dist/perf-report.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BINARY_PATH="${2:-${REPO_ROOT}/target/release/xtrc}"

mkdir -p "$(dirname "${OUTPUT_JSON}")"

if [ ! -x "${BINARY_PATH}" ]; then
    echo "Building optimized release binary for performance measurement..." >&2
    (cd "${REPO_ROOT}" && cargo build --release --locked)
fi

BINARY_ABS="$(cd "$(dirname "${BINARY_PATH}")" && pwd)/$(basename "${BINARY_PATH}")"

python3 - "${REPO_ROOT}" "${BINARY_ABS}" "${OUTPUT_JSON}" <<'PYEOF'
import json
import os
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

repo_root = Path(sys.argv[1]).resolve()
binary_path = Path(sys.argv[2]).resolve()
output_json = Path(sys.argv[3]).resolve()

trace_marker = "@" + "trace"
trace_test_marker = "@" + "trace:test"


def create_synthetic_1000_file_workspace(root: Path) -> None:
    (root / ".xtrc.toml").write_text(
        'version = "1"\n'
        '[project]\nname = "ci-perf-bench"\nroot = "."\n'
        '[specs]\ninclude = ["docs/**/*.md"]\nexclude = ["target/**"]\n'
        '[code]\ninclude = ["src/**/*.*"]\nextensions = [".rs", ".py", ".go", ".ts"]\n'
        '[tests]\ninclude = ["tests/**/*.*"]\n'
        '[validation]\nstrict = true\nrequire_test_coverage = true\n',
        encoding="utf-8",
    )
    docs_dir = root / "docs"
    src_dir = root / "src"
    tests_dir = root / "tests"
    docs_dir.mkdir(parents=True, exist_ok=True)
    src_dir.mkdir(parents=True, exist_ok=True)
    tests_dir.mkdir(parents=True, exist_ok=True)

    for doc_idx in range(1, 11):
        lines = [
            "---",
            "document_type: specification",
            f"id: SPEC-BENCH-{doc_idx:03d}",
            f"title: Benchmark Specification {doc_idx}",
            "version: 1.0.0",
            "status: approved",
            "---",
            "",
            f"### VIS-BENCH-{doc_idx:02d}: Vision {doc_idx}",
            f"- **ID**: VIS-BENCH-{doc_idx:02d}",
            "- **Parent**: None",
            "- **Type**: Vision",
            "",
            f"### STRAT-BENCH-{doc_idx:02d}: Strategy {doc_idx}",
            f"- **ID**: STRAT-BENCH-{doc_idx:02d}",
            f"- **Parent**: VIS-BENCH-{doc_idx:02d}",
            "- **Type**: Strategy",
            "",
            f"### EPIC-BENCH-{doc_idx:02d}: Epic {doc_idx}",
            f"- **ID**: EPIC-BENCH-{doc_idx:02d}",
            f"- **Parent**: STRAT-BENCH-{doc_idx:02d}",
            "- **Type**: Epic",
            "",
            f"### US-BENCH-{doc_idx:02d}: User Story {doc_idx}",
            f"- **ID**: US-BENCH-{doc_idx:02d}",
            f"- **Parent**: EPIC-BENCH-{doc_idx:02d}",
            "- **Type**: User Story",
            "",
        ]
        for item in range(1, 9):
            req_id = f"REQ-BENCH-{doc_idx:02d}-{item:02d}"
            spec_id = f"SPEC-BENCH-{doc_idx:02d}-{item:02d}"
            lines.extend(
                [
                    f"### {req_id}: Requirement {doc_idx}.{item}",
                    f"- **ID**: {req_id}",
                    f"- **Parent**: US-BENCH-{doc_idx:02d}",
                    "- **Type**: Requirement",
                    "",
                    f"### {spec_id}: Technical Spec {doc_idx}.{item}",
                    f"- **ID**: {spec_id}",
                    f"- **Parent**: {req_id}",
                    "- **Type**: TechnicalSpec",
                    "",
                ]
            )
        (docs_dir / f"spec_{doc_idx:02d}.md").write_text("\n".join(lines), encoding="utf-8")

    extensions = ["rs", "py", "go", "ts"]
    for idx in range(600):
        ext = extensions[idx % len(extensions)]
        sub = src_dir / f"mod_{idx // 50:02d}"
        sub.mkdir(parents=True, exist_ok=True)
        if idx < 80:
            doc_idx = (idx // 8) + 1
            item = (idx % 8) + 1
            prefix = "#" if ext == "py" else "//"
            body = (
                f"{prefix} {trace_marker} SPEC-BENCH-{doc_idx:02d}-{item:02d}, "
                f"REQ-BENCH-{doc_idx:02d}-{item:02d}\n"
                f"pub fn work_{idx}(v: usize) -> usize {{ v + {idx} }}\n"
            )
        else:
            body = f"// Non-matching module {idx}\npub fn noop_{idx}() -> usize {{ {idx} }}\n"
        (sub / f"file_{idx:04d}.{ext}").write_text(body, encoding="utf-8")

    for idx in range(390):
        sub = tests_dir / f"suite_{idx // 50:02d}"
        sub.mkdir(parents=True, exist_ok=True)
        if idx < 80:
            doc_idx = (idx // 8) + 1
            item = (idx % 8) + 1
            body = (
                f"// {trace_test_marker} SPEC-BENCH-{doc_idx:02d}-{item:02d}, "
                f"REQ-BENCH-{doc_idx:02d}-{item:02d}\n"
                f"#[test]\nfn test_{idx}() {{ assert_eq!(1, 1); }}\n"
            )
        else:
            body = f"#[test]\nfn helper_{idx}() {{ assert!(true); }}\n"
        (sub / f"test_{idx:04d}.rs").write_text(body, encoding="utf-8")

    # Include a 2 MiB binary asset with a NUL byte in the first 512 bytes (REQ-NFR-MEM)
    payload = bytearray(b"B" * (2 * 1024 * 1024))
    payload[64] = 0
    (src_dir / "binary_blob.rs").write_bytes(bytes(payload))


def measure_command(name: str, args: list[str], cwd: Path, budget_ms: float, runs: int = 3) -> dict:
    samples_ms = []
    last_rc = 0
    for _ in range(runs):
        t0 = time.perf_counter()
        proc = subprocess.run(
            [str(binary_path)] + args,
            cwd=str(cwd),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            check=False,
        )
        elapsed_ms = (time.perf_counter() - t0) * 1000.0
        last_rc = proc.returncode
        if last_rc != 0:
            stderr_msg = proc.stderr.decode("utf-8", errors="replace")
            raise RuntimeError(
                f"Benchmark workload '{name}' exited with code {last_rc}:\n{stderr_msg}"
            )
        samples_ms.append(elapsed_ms)

    samples_ms.sort()
    median_ms = round(samples_ms[len(samples_ms) // 2], 2)
    min_ms = round(samples_ms[0], 2)
    max_ms = round(samples_ms[-1], 2)
    passed = (last_rc == 0) and (median_ms <= budget_ms)
    return {
        "name": name,
        "command": "xtrc " + " ".join(args),
        "measured_ms": median_ms,
        "min_ms": min_ms,
        "max_ms": max_ms,
        "budget_ms": budget_ms,
        "passed": passed,
    }


benchmarks = []
benchmarks.append(
    measure_command(
        "self_check_strict",
        ["check", "--strict", "--json"],
        repo_root,
        budget_ms=200.0,
    )
)
benchmarks.append(
    measure_command(
        "self_matrix_json",
        ["matrix", "--format", "json"],
        repo_root,
        budget_ms=200.0,
    )
)
benchmarks.append(
    measure_command(
        "self_tree_json",
        ["tree", "--json"],
        repo_root,
        budget_ms=200.0,
    )
)
benchmarks.append(
    measure_command(
        "self_export_markdown",
        ["export", "--format", "markdown"],
        repo_root,
        budget_ms=200.0,
    )
)

with tempfile.TemporaryDirectory() as tmp_dir:
    synth_root = Path(tmp_dir)
    create_synthetic_1000_file_workspace(synth_root)
    benchmarks.append(
        measure_command(
            "synthetic_1000_files_check_strict",
            ["check", "--strict", "--json"],
            synth_root,
            budget_ms=500.0,
        )
    )

all_passed = all(b["passed"] for b in benchmarks)
report = {
    "schema_version": "1.0.0",
    "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "status": "pass" if all_passed else "fail",
    "benchmarks": benchmarks,
}

output_json.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

md_lines = [
    "### xtrc CI/CD Performance & NFR SLA Verification Report",
    "",
    "| Workload | Command | Median Latency (ms) | SLA Budget (ms) | Status |",
    "|---|---|---:|---:|---|",
]
for b in benchmarks:
    status_str = "PASS" if b["passed"] else "FAIL"
    md_lines.append(
        f"| `{b['name']}` | `{b['command']}` | {b['measured_ms']:.2f} | {b['budget_ms']:.1f} | **{status_str}** |"
    )
md_lines.append("")

md_summary = "\n".join(md_lines)
sys.stderr.write(md_summary + "\n")

step_summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
if step_summary_path:
    with open(step_summary_path, "a", encoding="utf-8") as sf:
        sf.write(md_summary + "\n")

if not all_passed:
    sys.stderr.write("ERROR: One or more NFR performance SLA budgets were exceeded.\n")
    sys.exit(1)
PYEOF
