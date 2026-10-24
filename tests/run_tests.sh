#!/usr/bin/env bash
#
# tests/run_tests.sh - Comprehensive E2E test runner for xtrc repository bootstrap
#
# Exits with 0 if and only if 100% of tests pass.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

echo "======================================================================"
echo " Running xtrc E2E Repository Bootstrap Acceptance Test Suite"
echo " Repo root: ${REPO_ROOT}"
echo "======================================================================"

# Determine Python binary
if command -v python3 >/dev/null 2>&1; then
    PYTHON_BIN="python3"
elif command -v python >/dev/null 2>&1; then
    PYTHON_BIN="python"
else
    echo "ERROR: python3 or python is required to run acceptance tests." >&2
    exit 1
fi

echo "Using Python: $(${PYTHON_BIN} --version)"
echo ""

# Run test suite via unittest discovery
echo "Executing test_bootstrap.py..."
set +e
${PYTHON_BIN} -m unittest discover -s tests -p "test_*.py" -v
EXIT_CODE=$?
set -e

echo ""
echo "======================================================================"
if [ ${EXIT_CODE} -eq 0 ]; then
    echo " [PASS] 100% of xtrc bootstrap acceptance tests passed successfully!"
    echo "======================================================================"
    exit 0
else
    echo " [FAIL] One or more acceptance tests failed. Exit code: ${EXIT_CODE}"
    echo "======================================================================"
    exit ${EXIT_CODE}
fi
