#!/usr/bin/env bash
# @trace SPEC-CD-DELIVERY, REQ-PKG-SBOM
# Generates a deterministic SPDX 2.3 JSON Software Bill of Materials (SBOM)
# from Cargo workspace metadata and Cargo.lock.

set -euo pipefail

OUTPUT_PATH="${1:-dist/xtrc.spdx.json}"
mkdir -p "$(dirname "${OUTPUT_PATH}")"

CREATED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

(cargo metadata --format-version 1 --locked --offline 2>/dev/null || cargo metadata --format-version 1 --locked) | jq \
  --arg created "${CREATED_AT}" \
  '
  def spdx_id($name; $ver):
    "SPDXRef-Crate-" + (($name + "-" + $ver) | gsub("[^A-Za-z0-9.-]"; "-"));

  (.resolve.root) as $root_id |
  (.packages | map(select(.id == $root_id)) | .[0]) as $root_pkg |
  (.resolve.nodes | map({key: .id, value: .}) | from_entries) as $nodes |
  (.packages | map({key: .id, value: .}) | from_entries) as $pkgs |

  {
    spdxVersion: "SPDX-2.3",
    dataLicense: "CC0-1.0",
    SPDXID: "SPDXRef-DOCUMENT",
    name: ($root_pkg.name + "-" + $root_pkg.version + "-sbom"),
    documentNamespace: ("https://github.com/sor4c/xtrc/spdx/" + $root_pkg.name + "-" + $root_pkg.version + "/" + $created),
    creationInfo: {
      created: $created,
      creators: [
        "Tool: xtrc-spdx-generator-1.0.0",
        "Organization: xtrc contributors"
      ],
      licenseListVersion: "3.22"
    },
    documentDescribes: [
      spdx_id($root_pkg.name; $root_pkg.version)
    ],
    packages: [
      .packages[] | {
        name: .name,
        SPDXID: spdx_id(.name; .version),
        versionInfo: .version,
        downloadLocation: (if .source == null then "NOASSERTION" else ("https://crates.io/api/v1/crates/" + .name + "/" + .version + "/download") end),
        filesAnalyzed: false,
        licenseConcluded: (.license // "NOASSERTION"),
        licenseDeclared: (.license // "NOASSERTION"),
        copyrightText: "NOASSERTION",
        primaryPackagePurpose: (if .id == $root_id then "APPLICATION" else "LIBRARY" end),
        externalRefs: [
          {
            referenceCategory: "PACKAGE-MANAGER",
            referenceType: "purl",
            referenceLocator: ("pkg:cargo/" + .name + "@" + .version)
          }
        ]
      }
    ],
    relationships: (
      [
        {
          spdxElementId: "SPDXRef-DOCUMENT",
          relatedSpdxElement: spdx_id($root_pkg.name; $root_pkg.version),
          relationshipType: "DESCRIBES"
        }
      ] + [
        .resolve.nodes[] as $node |
        ($pkgs[$node.id]) as $parent_pkg |
        $node.deps[] |
        ($pkgs[.pkg]) as $dep_pkg |
        {
          spdxElementId: spdx_id($parent_pkg.name; $parent_pkg.version),
          relatedSpdxElement: spdx_id($dep_pkg.name; $dep_pkg.version),
          relationshipType: "DEPENDS_ON"
        }
      ]
    )
  }
  ' > "${OUTPUT_PATH}"
