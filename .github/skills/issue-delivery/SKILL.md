---
name: issue-delivery
description: 'Implement a GitHub issue end to end in xtrc. Use when asked to implement, complete, or deliver an issue, including requirements review, tests, independent PR review, version and SBOM impact, merge, post-merge CI, issue closure, and branch cleanup.'
argument-hint: 'GitHub issue number or URL'
user-invocable: true
---

# Implement an xtrc Issue

Use this workflow when the user asks to deliver an issue, not when they ask only for a plan, explanation, or partial implementation. Follow the repository's [AGENTS.md](../../../AGENTS.md), [Copilot instructions](../../copilot-instructions.md), and [contribution and delivery policy](../../../CONTRIBUTING.md). Treat those files and the issue as authoritative; this skill orchestrates them rather than replacing their requirements.

## Safety and authorization

- Never push directly to `main`. Make all changes on an issue-specific branch and submit them through a PR.
- Do not overwrite or discard unrelated user changes. If the worktree is dirty, the current branch is not `main`, or repository state conflicts with the expected workflow, inspect and preserve the state; stop and ask before proceeding if safe isolation is unclear.
- Do not invent requirements, mark unchecked acceptance criteria complete without evidence, or silently change the issue's scope.
- Ask the user when requirements are ambiguous, a material design choice is unresolved, or a required action is not authorized. Stop rather than claim completion when GitHub permissions, CI status, the independent reviewer, or another required capability is unavailable.
- A finding from `github-advanced-security[bot]` is a hard merge blocker until it has been fixed and its review conversation resolved. Inspect the bot's review threads and the required `Trusted Advanced Security Review Findings` check; never dismiss, ignore, or bypass a finding to unblock a merge. Do not rely on a PR review-comment-triggered workflow for this check because it can execute PR-controlled workflow code.
- If a finding is fixed and its thread resolved without a new push, dispatch the trusted workflow from `main` for the current PR number and head SHA using `gh workflow run security-review-gate.yml --repo sor4c/xtrc --ref main -f pr_number=<number> -f head_sha=<head-sha>`.
- For the one-time bootstrap PR that first adds `.github/workflows/security-review-gate.yml`, GitHub cannot run the trusted `pull_request_target` workflow until it is on the base branch. Do not merge until required-conversation-resolution protection is active, all GHAS findings are fixed and their threads resolved, and the trusted checker has been independently run against the exact current PR head. Immediately after merge, require the new trusted workflow check on `main`; ordinary PRs must not merge if that required check is absent or not passing.
- Do not include auto-closing PR phrases (`Closes`, `Fixes`, or `Resolves`) for the issue. GitHub may close it at merge time before the merge-commit workflows finish. Use `Relates to #N`; close the issue explicitly only after post-merge verification succeeds.

## Workflow

### 1. Establish issue context

1. Resolve the supplied issue number/URL and verify it belongs to `sor4c/xtrc`. Read the full issue, labels, acceptance criteria, linked issues/specifications, and current state.
2. Confirm the issue is actionable and its requirements and acceptance criteria are sufficiently clear. If it is closed, already implemented, lacks requirements, or conflicts with repository policy, stop and ask what the user wants.
3. Inspect the current repository state, `main`, existing branches/PRs, relevant tests and implementation, `.github/workflows/`, and the applicable sections of `AGENTS.md`, `CONTRIBUTING.md`, and project docs. Check issue checklist items against the repository; do not assume they are already done.
4. Confirm the worktree is clean and start from an up-to-date `main`. Create a dedicated branch named `<type>/<issue-number>-<short-description>` (`feat/`, `fix/`, `docs/`, or `chore/` as appropriate). Never reuse an unrelated branch.

### 2. Implement the whole issue

1. Work to the issue's acceptance criteria and behavioral intent. Add or update tests, documentation, traceability, and all directly or indirectly affected surfaces.
2. Explicitly assess whether the change requires:
   - a base-version change under the repository's SemVer policy;
   - an SBOM change or regeneration under the delivery policy and `scripts/generate-sbom.sh`;
   - README, design, requirements, NFR, examples, release, or other documentation updates;
   - updates to tests, fixtures, CI workflows, generated assets, or dependent metadata.
   Update only what the change warrants. Explain in the PR why a potentially relevant surface did not need a change; do not bump versions or regenerate artifacts mechanically.
3. Run every applicable repository quality gate from `AGENTS.md` and the issue. Read full output and exit status; fix failures rather than weakening checks or claiming success.
4. Review the complete diff against the issue and repository rules. Check whitespace, generated files, unrelated edits, traceability, and whether every acceptance criterion has evidence. Restore only generated side effects that this task created and that are not intended deliverables.

### 3. Independent clean-context review

1. Before merging, invoke the `issue-reviewer` custom agent in a fresh context. Give it the issue URL and acceptance criteria, PR URL or base/head SHAs, and ask it to independently inspect the complete change.
2. The reviewer is read-only. Do not have it edit files, commit, push, merge, or modify GitHub state.
3. If the agent is unavailable, the review did not examine the complete diff, or it returns actionable findings, do not merge. Fix findings on the issue branch, rerun affected checks, and request a new fresh-context review of the updated diff. Repeat until there are no unresolved findings. Record the review outcome in the PR.
4. Do not ask the implementation context to impersonate or summarize its own work as an independent clean-context review.

### 4. Pull request and pre-merge gate

1. Commit with an issue reference, push the issue branch, and open a PR targeting `main`. Include:
   - the issue reference `Relates to #N` (never an auto-closing keyword);
   - a summary mapped to the issue requirements;
   - verification commands and observed results;
   - clean-context review outcome;
   - version/SBOM/docs impact assessment;
   - acceptance criteria only when supported by evidence.
2. Wait for every required PR check to finish successfully and inspect review comments and unresolved threads, especially findings from `github-advanced-security[bot]`. Do not merge while any such finding is unresolved, or while any required check is pending, failed, or unknown. Resolve findings only after addressing them. If thread resolution does not trigger a new trusted check, dispatch the trusted workflow from `main` for the current PR number and head SHA, and verify success for that exact head. Apply only the one-time trusted-workflow bootstrap procedure above when the PR itself installs that workflow.
3. Merge only when the user asked for end-to-end delivery and all requirements, independent review, PR checks, and repository merge rules are satisfied. If merge authorization or a required gate is unclear, stop and ask; never bypass a required check.

### 5. Verify the merge and close the issue

1. After merge, obtain the exact merge commit SHA. Find and wait for every required workflow triggered on `main` for that SHA. Determine required checks from repository rules and workflow configuration; do not substitute branch/PR checks or a different SHA's results.
2. Verify each required run's final conclusion. If a required workflow fails, keep or reopen the issue immediately, diagnose and fix via a follow-up issue branch and PR, and repeat clean-context review, PR gates, merge, and exact-SHA post-merge verification. If a workflow is pending, missing, skipped unexpectedly, or unverifiable, leave the issue open and report verification as pending.
3. Only after all required post-merge checks pass, update issue acceptance checkboxes that are demonstrably satisfied and close the issue explicitly with a concise verification comment. Preserve the rest of the issue body.

### 6. Restore repository state

1. Delete the merged remote issue branch and the corresponding local branch.
2. Switch the local checkout to `main`, fast-forward from `origin/main`, and verify it is at the merged commit or later.
3. Confirm `git status` is clean and no task branch remains checked out. Preserve unrelated work; if cleanup cannot safely produce a clean `main`, stop and report the exact state instead of deleting or overwriting anything.
4. Report the PR, merge SHA, post-merge workflow results, issue state, branch cleanup, worktree state, and any skipped/not-applicable version/SBOM/doc updates. Do not report the task complete until the gates above are verified.
