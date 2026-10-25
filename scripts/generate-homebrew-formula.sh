#!/usr/bin/env bash
# @trace SPEC-CD-DELIVERY, REQ-PKG-DIST
# Generates a Homebrew Formula (xtrc.rb) for the packaged xtrc release archive,
# embedding the exact SHA-256 checksum and installing both the binary and SPDX SBOM.

set -euo pipefail

VERSION="${1:?Usage: generate-homebrew-formula.sh <version> <tarball_path> [output_path] [repo] [tag]}"
TARBALL_PATH="${2:?Usage: generate-homebrew-formula.sh <version> <tarball_path> [output_path] [repo] [tag]}"
OUTPUT_PATH="${3:-dist/xtrc.rb}"
REPO="${4:-sor4c/xtrc}"
TAG="${5:-main-head}"

if [ ! -f "${TARBALL_PATH}" ]; then
  echo "Error: tarball not found at '${TARBALL_PATH}'" >&2
  exit 2
fi

mkdir -p "$(dirname "${OUTPUT_PATH}")"

TARBALL_FILENAME="$(basename "${TARBALL_PATH}")"
SHA256="$(sha256sum "${TARBALL_PATH}" | awk '{print $1}')"

cat > "${OUTPUT_PATH}" <<EOF
# @trace SPEC-CD-DELIVERY, REQ-PKG-DIST
class Xtrc < Formula
  desc "Fast, plain-text traceability CLI and 7-layer DAG engine in Rust"
  homepage "https://github.com/${REPO}"
  url "https://github.com/${REPO}/releases/download/${TAG}/${TARBALL_FILENAME}"
  version "${VERSION}"
  sha256 "${SHA256}"
  license "MIT"

  def install
    bin.install "xtrc"
    doc.install "README.md", "xtrc.spdx.json"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/xtrc --version")
  end
end
EOF
