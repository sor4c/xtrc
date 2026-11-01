---
name: Implement GitHub Issue
description: "Deliver an xtrc GitHub issue end to end using the repository workflow, independent review, and post-merge verification."
argument-hint: "Issue number or URL"
agent: agent
---

Implement and deliver this GitHub issue: ${input:issue:Issue number or URL}

Follow the repository `issue-delivery` skill in `.github/skills/issue-delivery/SKILL.md` and all repository instructions. Complete the issue through implementation, independent clean-context review, PR checks, merge, verification of required workflows on the exact merge commit, issue checklist/closure, and branch/worktree cleanup. Stop and ask rather than bypassing an unclear requirement, failed or unavailable gate, missing permission, or unsafe cleanup. Do not put an auto-closing keyword in the PR description.
