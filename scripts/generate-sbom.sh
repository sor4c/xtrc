#!/usr/bin/env bash
# @trace SPEC-CD-DELIVERY, SPEC-VER-POLICY, REQ-PKG-SBOM, REQ-VER-POLICY
# Generates a deterministic SPDX 2.3 JSON Software Bill of Materials (SBOM)
# from Cargo workspace metadata and Cargo.lock, stamped with the unique
# delivery version policy identifier.

set -euo pipefail

OUTPUT_PATH="${1:-dist/xtrc.spdx.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DELIVERY_VERSION="${2:-$(bash "${SCRIPT_DIR}/compute-version.sh")}"

mkdir -p "$(dirname "${OUTPUT_PATH}")"

CREATED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

(cargo metadata --format-version 1 --locked --offline 2>/dev/null || cargo metadata --format-version 1 --locked) | jq \
  --arg created "${CREATED_AT}" \
  --arg delivery_ver "${DELIVERY_VERSION}" \
  '
  def spdx_id($name; $ver):
    "SPDXRef-Crate-" + (($name + "-" + $ver) | gsub("[^A-Za-z0-9.-]"; "-"));

  def pkg_ver($pkg; $root_id; $dver):
    if $pkg.id == $root_id then $dver else $pkg.version end;

  (.resolve.root) as $root_id |
  (.packages | map(select(.id == $root_id)) | .[0]) as $root_pkg |
  ($delivery_ver) as $root_ver |
  (.resolve.nodes | map({key: .id, value: .}) | from_entries) as $nodes |
  (.packages | map({key: .id, value: .}) | from_entries) as $pkgs |

  {
    spdxVersion: "SPDX-2.3",
    dataLicense: "CC0-1.0",
    SPDXID: "SPDXRef-DOCUMENT",
    name: ($root_pkg.name + "-" + $root_ver + "-sbom"),
    documentNamespace: ("https://github.com/sor4c/xtrc/spdx/" + $root_pkg.name + "-" + $root_ver + "/" + $created),
    creationInfo: {
      created: $created,
      creators: [
        "Tool: xtrc-spdx-generator-1.0.0",
        "Organization: xtrc contributors"
      ],
      licenseListVersion: "3.22"
    },
    documentDescribes: [
      spdx_id($root_pkg.name; $root_ver)
    ],
    packages: [
      .packages[] |
      (pkg_ver(.; $root_id; $root_ver)) as $pver |
      {
        name: .name,
        SPDXID: spdx_id(.name; $pver),
        versionInfo: $pver,
        downloadLocation: (if .source == null then "NOASSERTION" else ("https://crates.io/api/v1/crates/" + .name + "/" + $pver + "/download") end),
        filesAnalyzed: false,
        licenseConcluded: (.license // "NOASSERTION"),
        licenseDeclared: (.license // "NOASSERTION"),
        copyrightText: "NOASSERTION",
        primaryPackagePurpose: (if .id == $root_id then "APPLICATION" else "LIBRARY" end),
        externalRefs: [
          {
            referenceCategory: "PACKAGE-MANAGER",
            referenceType: "purl",
            referenceLocator: ("pkg:cargo/" + .name + "@" + $pver)
          }
        ]
      }
    ],
    relationships: (
      [
        {
          spdxElementId: "SPDXRef-DOCUMENT",
          relatedSpdxElement: spdx_id($root_pkg.name; $root_ver),
          relationshipType: "DESCRIBES"
        }
      ] + [
        .resolve.nodes[] as $node |
        ($pkgs[$node.id]) as $parent_pkg |
        (pkg_ver($parent_pkg; $root_id; $root_ver)) as $parent_ver |
        $node.deps[] |
        ($pkgs[.pkg]) as $dep_pkg |
        (pkg_ver($dep_pkg; $root_id; $root_ver)) as $dep_ver |
        {
          spdxElementId: spdx_id($parent_pkg.name; $parent_ver),
          relatedSpdxElement: spdx_id($dep_pkg.name; $dep_ver),
          relationshipType: "DEPENDS_ON"
        }
      ]
    )
  }
  ' > "${OUTPUT_PATH}"
