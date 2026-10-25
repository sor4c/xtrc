#!/usr/bin/env bash
# ==============================================================================
# xtrc End-to-End Tracing Demo Runner
#
# Executes the full plain-text traceability lifecycle on an example multi-layer
# workspace, demonstrating verification, hierarchy visualization, matrix
# generation, and multi-format compliance reporting (HTML, JSON, Markdown, CSV).
# ==============================================================================

set -euo pipefail

# Determine directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
REPORTS_DIR="${SCRIPT_DIR}/reports"

# Parse CLI arguments or environment variables
XTRC_BIN="${XTRC_BIN:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --xtrc-bin)
      XTRC_BIN="$2"
      shift 2
      ;;
    -h|--help)
      echo "Usage: ./run-demo.sh [--xtrc-bin <PATH_TO_XTRC>]"
      echo ""
      echo "Options:"
      echo "  --xtrc-bin <PATH>   Path to xtrc binary (defaults to target/release, target/debug, or PATH)"
      echo "  -h, --help          Show this help message"
      exit 0
      ;;
    *)
      echo "Error: Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

# Locate xtrc executable
if [ -z "${XTRC_BIN}" ]; then
  if [ -x "${REPO_ROOT}/target/release/xtrc" ]; then
    XTRC_BIN="${REPO_ROOT}/target/release/xtrc"
  elif [ -x "${REPO_ROOT}/target/debug/xtrc" ]; then
    XTRC_BIN="${REPO_ROOT}/target/debug/xtrc"
  elif command -v xtrc >/dev/null 2>&1; then
    XTRC_BIN="$(command -v xtrc)"
  else
    echo "Notice: Compiled xtrc binary not found. Building debug binary..."
    cargo build --manifest-path "${REPO_ROOT}/Cargo.toml" --quiet
    XTRC_BIN="${REPO_ROOT}/target/debug/xtrc"
  fi
fi

echo "================================================================================"
echo "                   xtrc End-to-End Tracing Demo Runner                          "
echo "================================================================================"
echo "Workspace Directory : ${SCRIPT_DIR}"
echo "Reports Output Dir  : ${REPORTS_DIR}"
echo "Using xtrc Binary   : ${XTRC_BIN}"
echo "xtrc Version        : $("${XTRC_BIN}" --version)"
echo "--------------------------------------------------------------------------------"

mkdir -p "${REPORTS_DIR}"

# Step 1: Strict Traceability Verification
echo "==> [1/5] Verifying 7-Layer Traceability Graph (xtrc check --strict)..."
"${XTRC_BIN}" --root "${SCRIPT_DIR}" check --strict

# Step 2: Hierarchical Tree Traversal
echo ""
echo "==> [2/5] Visualizing Dependency Hierarchy from Vision Root (xtrc tree)..."
"${XTRC_BIN}" --root "${SCRIPT_DIR}" tree VIS-DEMO-PAY

# Step 3: Multidimensional Traceability Matrix
echo ""
echo "==> [3/5] Generating Multidimensional Matrices (Table, Markdown, HTML)..."
"${XTRC_BIN}" --root "${SCRIPT_DIR}" matrix
"${XTRC_BIN}" --root "${SCRIPT_DIR}" matrix --format markdown > "${REPORTS_DIR}/matrix.md"
"${XTRC_BIN}" --root "${SCRIPT_DIR}" matrix --format html > "${REPORTS_DIR}/matrix.html"

# Step 4: Multi-Format Compliance Export Reports
echo ""
echo "==> [4/5] Exporting Multi-Format Audit Reports (HTML, JSON, Markdown, CSV)..."
"${XTRC_BIN}" --root "${SCRIPT_DIR}" export --format html -o "${REPORTS_DIR}/traceability-report.html"
"${XTRC_BIN}" --root "${SCRIPT_DIR}" export --format json -o "${REPORTS_DIR}/traceability-report.json"
"${XTRC_BIN}" --root "${SCRIPT_DIR}" export --format markdown -o "${REPORTS_DIR}/traceability-report.md"
"${XTRC_BIN}" --root "${SCRIPT_DIR}" export --format csv -o "${REPORTS_DIR}/traceability-report.csv"

# Step 5: Report File Integrity Verification
echo ""
echo "==> [5/5] Verifying Generated Report Artifacts..."
REPORT_FILES=(
  "traceability-report.html"
  "traceability-report.json"
  "traceability-report.md"
  "traceability-report.csv"
  "matrix.html"
  "matrix.md"
)

for file in "${REPORT_FILES[@]}"; do
  full_path="${REPORTS_DIR}/${file}"
  if [ ! -s "${full_path}" ]; then
    echo "Error: Expected report artifact '${file}' is missing or empty." >&2
    exit 1
  fi
  size="$(wc -c < "${full_path}" | tr -d ' ')"
  echo "  [OK] ${file} (${size} bytes)"
done

echo ""
echo "================================================================================"
echo " Demo completed successfully! All reports generated in:                         "
echo "   ${REPORTS_DIR}                                                               "
echo " View HTML Report in Browser:                                                   "
echo "   file://${REPORTS_DIR}/traceability-report.html                               "
echo "================================================================================"
