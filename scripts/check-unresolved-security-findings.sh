#!/usr/bin/env bash
# @trace SPEC-CI-VERIFY, REQ-CI-VERIFY
set -euo pipefail

if [[ -z "${GITHUB_REPOSITORY:-}" || -z "${PR_NUMBER:-}" || ! "${PR_NUMBER}" =~ ^[0-9]+$ ]]; then
  echo "GITHUB_REPOSITORY and a numeric PR_NUMBER are required." >&2
  exit 2
fi

if [[ -z "${GH_TOKEN:-}" ]]; then
  echo "GH_TOKEN is required to inspect pull request review threads." >&2
  exit 2
fi

owner="${GITHUB_REPOSITORY%%/*}"
repository="${GITHUB_REPOSITORY#*/}"
if [[ -z "${owner}" || -z "${repository}" || "${repository}" == */* ]]; then
  echo "GITHUB_REPOSITORY must be in owner/repository form." >&2
  exit 2
fi

query="$(cat <<'GRAPHQL'
query($owner: String!, $name: String!, $number: Int!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviewThreads(first: 100, after: $cursor) {
        nodes {
          isResolved
          comments(first: 100) {
            nodes {
              author {
                login
              }
              url
            }
            pageInfo {
              hasNextPage
            }
          }
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
    }
  }
}
GRAPHQL
)"

findings_file="$(mktemp)"
trap 'rm -f "${findings_file}"' EXIT
cursor=""
incomplete=false

while :; do
  args=(
    graphql
    -F "owner=${owner}"
    -F "name=${repository}"
    -F "number=${PR_NUMBER}"
    -f "query=${query}"
  )
  if [[ -n "${cursor}" ]]; then
    args+=(-F "cursor=${cursor}")
  fi

  response="$(gh api "${args[@]}")"
  if ! jq -e '(.errors // []) | length == 0' >/dev/null <<<"${response}"; then
    echo "GitHub GraphQL returned errors; refusing to pass the security review gate." >&2
    exit 1
  fi
  if ! jq -e '.data.repository.pullRequest.reviewThreads != null' >/dev/null <<<"${response}"; then
    echo "Could not read pull request review threads; refusing to pass the security review gate." >&2
    exit 1
  fi

  jq -r '
    .data.repository.pullRequest.reviewThreads.nodes[]
    | select(.isResolved == false)
    | .comments.nodes[]
    | select(
        .author.login == "github-advanced-security[bot]"
        or .author.login == "github-advanced-security"
      )
    | .url
  ' <<<"${response}" >>"${findings_file}"

  if jq -e '
    any(
      .data.repository.pullRequest.reviewThreads.nodes[];
      .comments.pageInfo.hasNextPage == true
    )
  ' >/dev/null <<<"${response}"; then
    incomplete=true
  fi

  has_next="$(jq -r '.data.repository.pullRequest.reviewThreads.pageInfo.hasNextPage' <<<"${response}")"
  if [[ "${has_next}" != "true" ]]; then
    break
  fi

  cursor="$(jq -r '.data.repository.pullRequest.reviewThreads.pageInfo.endCursor // empty' <<<"${response}")"
  if [[ -z "${cursor}" ]]; then
    echo "Review-thread pagination was incomplete; refusing to pass the security review gate." >&2
    exit 1
  fi
done

if [[ "${incomplete}" == "true" ]]; then
  echo "A review thread has more than 100 comments; refusing to pass without inspecting every comment." >&2
  exit 1
fi

if [[ -s "${findings_file}" ]]; then
  echo "Unresolved github-advanced-security[bot] finding(s) block this PR:" >&2
  while IFS= read -r finding_url; do
    printf ' - %s\n' "${finding_url}" >&2
  done <"${findings_file}"
  exit 1
fi

echo "No unresolved github-advanced-security[bot] findings."
