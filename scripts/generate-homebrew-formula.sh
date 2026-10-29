#!/usr/bin/env bash
# @trace SPEC-CD-DELIVERY, REQ-PKG-DIST
# Generates a Homebrew Formula with native Intel and Apple Silicon macOS archives.

set -euo pipefail

VERSION="${1:?Usage: generate-homebrew-formula.sh <version> <intel_tarball> <arm_tarball> [output_path] [repo] [tag]}"
INTEL_TARBALL="${2:?Usage: generate-homebrew-formula.sh <version> <intel_tarball> <arm_tarball> [output_path] [repo] [tag]}"
ARM_TARBALL="${3:?Usage: generate-homebrew-formula.sh <version> <intel_tarball> <arm_tarball> [output_path] [repo] [tag]}"
OUTPUT_PATH="${4:-dist/xtrc.rb}"
REPO="${5:-sor4c/xtrc}"
TAG="${6:-main-head}"

if [ ! -f "${INTEL_TARBALL}" ]; then
  echo "Error: Intel macOS tarball not found at '${INTEL_TARBALL}'" >&2
  exit 2
fi

if [ ! -f "${ARM_TARBALL}" ]; then
  echo "Error: Apple Silicon macOS tarball not found at '${ARM_TARBALL}'" >&2
  exit 2
fi

mkdir -p "$(dirname "${OUTPUT_PATH}")"

INTEL_FILENAME="$(basename "${INTEL_TARBALL}")"
INTEL_SHA256="$(sha256sum "${INTEL_TARBALL}" | awk '{print $1}')"
ARM_FILENAME="$(basename "${ARM_TARBALL}")"
ARM_SHA256="$(sha256sum "${ARM_TARBALL}" | awk '{print $1}')"

cat > "${OUTPUT_PATH}" <<EOF
# @trace SPEC-CD-DELIVERY, REQ-PKG-DIST
class Xtrc < Formula
  desc "Fast, plain-text traceability CLI and 7-layer DAG engine in Rust"
  homepage "https://github.com/${REPO}"
  version "${VERSION}"
  license "MIT"

  on_intel do
    url "https://github.com/${REPO}/releases/download/${TAG}/${INTEL_FILENAME}"
    sha256 "${INTEL_SHA256}"
  end

  on_arm do
    url "https://github.com/${REPO}/releases/download/${TAG}/${ARM_FILENAME}"
    sha256 "${ARM_SHA256}"
  end

  def install
    bin.install "xtrc"
    doc.install "README.md", "xtrc.spdx.json"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/xtrc --version")
  end
end
EOF
