#!/usr/bin/env bash
# @trace SPEC-VER-POLICY, SPEC-CD-DELIVERY, REQ-VER-POLICY
# Computes a deterministic, unique version string for xtrc builds and deliveries.
#
# Policy:
# - Base version (<MAJOR>.<MINOR>.<PATCH>) is read from Cargo.toml.
# - If HEAD is tagged with an exact release tag "v<MAJOR>.<MINOR>.<PATCH>",
#   emits "<MAJOR>.<MINOR>.<PATCH>".
# - Otherwise, emits "<MAJOR>.<MINOR>.<PATCH>-r<COMMIT_COUNT>.g<SHORT_SHA>"
#   (e.g., "0.1.0-r25.g87a20ea"), guaranteeing every commit on main produces a
#   monotonic, collision-free version across Cargo, Debian (.deb), Homebrew (.rb),
#   tarball archives (.tar.gz), and SPDX 2.3 JSON SBOMs.

set -euo pipefail

REPO_DIR="${1:-.}"
CARGO_TOML="${REPO_DIR}/Cargo.toml"

if [ ! -f "${CARGO_TOML}" ]; then
  echo "Error: Cargo.toml not found at '${CARGO_TOML}'" >&2
  exit 2
fi

BASE_VERSION="$(awk -F'"' '/^version *= *"/ { print $2; exit }' "${CARGO_TOML}")"
if [ -z "${BASE_VERSION}" ]; then
  echo "Error: failed to extract base version from '${CARGO_TOML}'" >&2
  exit 2
fi

if git -C "${REPO_DIR}" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  EXACT_TAG="$(git -C "${REPO_DIR}" describe --exact-match --tags HEAD 2>/dev/null || true)"
  if [ "${EXACT_TAG}" = "v${BASE_VERSION}" ]; then
    echo "${BASE_VERSION}"
    exit 0
  fi

  COMMIT_COUNT="$(git -C "${REPO_DIR}" rev-list --count HEAD 2>/dev/null || echo "0")"
  SHORT_SHA="$(git -C "${REPO_DIR}" rev-parse --short=7 HEAD 2>/dev/null || echo "unknown")"
  if [ "${COMMIT_COUNT}" != "0" ] && [ "${SHORT_SHA}" != "unknown" ]; then
    echo "${BASE_VERSION}-r${COMMIT_COUNT}.g${SHORT_SHA}"
    exit 0
  fi
fi

echo "${BASE_VERSION}"
