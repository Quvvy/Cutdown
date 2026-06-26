---
name: issue-implement
description: Implement a GitHub issue for Cutdown — read issue, branch, minimal fix, validate, commit, push, open PR. Use when asked to implement, fix, or work on issue #N or an agent-ready issue.
disable-model-invocation: true
---

# Implement a GitHub issue

You are the **implement** agent. Make the change, validate it, and open a PR. One issue, one branch, one focused diff.

Read [cutdown.md](cutdown.md) for project-specific pitfalls before editing timeline, export, or persistence code.

## Workflow

1. **Read the issue**
   - `gh issue view N --json title,body,labels,url`
   - Restate acceptance criteria in your own words.
   - If criteria are missing or ambiguous, ask before coding.

2. **Branch**
   - Start from up-to-date `main`: `git fetch origin` then `git checkout main` and `git pull origin main`
   - Create `fix/issue-N-short-slug` (lowercase, hyphens, no spaces)

3. **Implement**
   - Smallest change that satisfies the issue.
   - Match existing naming, types, and patterns in touched files.
   - Do not refactor unrelated code or expand scope.

4. **Validate** (required before commit)
   ```powershell
   npm run validate:release
   ```
   Fix failures. Do not commit with a red validate run.

5. **Commit**
   - Stage only files relevant to the issue.
   - Never stage: `.cursor/debug*.log`, `.tauri/`, `release/*.exe`, secrets, `.env`
   - Message: one short subject line focused on *why* (repo style: complete sentences)

6. **Push and open PR**
   ```powershell
   git push -u origin HEAD
   gh pr create --title "..." --body "$(cat <<'EOF'
   ## Summary
   ...

   ## Test plan
   - [ ] npm run validate:release
   - [ ] ...

   Closes #N
   EOF
   )"
   ```
   - Link the issue (`Closes #N`).
   - If you changed timeline CSS/layout, note in the PR body that **installed NSIS smoke** is recommended (see cutdown.md).

## Do not

- Commit unless implementing an issue (user may have other rules — follow them)
- Force-push `main`
- Work on multiple unrelated issues in one PR
- Skip validation

## After PR is open

Tell the user the PR URL and suggest a **new chat** with the `issue-review` skill for a readonly review.
