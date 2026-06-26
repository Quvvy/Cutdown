# Agents

Two-role workflow for GitHub issues: one agent **implements**, another **reviews** (readonly). Manual triggers only — you start each chat.

Full walkthrough: [docs/AGENT-WORKFLOW.md](docs/AGENT-WORKFLOW.md)

## Skills (project)

| Skill | Role | Path |
|-------|------|------|
| `issue-implement` | Branch, fix, validate, PR | [.cursor/skills/issue-implement/](.cursor/skills/issue-implement/) |
| `issue-review` | Readonly PR review | [.cursor/skills/issue-review/](.cursor/skills/issue-review/) |

Invoke by name in your prompt (skills use `disable-model-invocation` so they load when you ask).

## Labels (GitHub)

Create once on the repo if you want a gate:

- **`agent-ready`** — issue is scoped and safe for an implement agent

## Starter prompts

### Implement (new chat)

Replace `N` with the issue number.

```text
Use the issue-implement skill.

Implement GitHub issue #N. Branch, validate, commit, push, and open a PR with gh.
```

### Review (new chat — not the same session as implement)

Replace `X` with the PR number.

```text
Use the issue-review skill. Review-only — do not edit files.

Review PR #X against the linked issue. Approve or request changes.
```

### Fix review feedback (implement chat or new implement chat)

```text
Use the issue-implement skill.

Address the review feedback on PR #X. Push updates to the same branch.
```

## Rules of thumb

- One issue → one branch → one PR
- Review in a **separate** chat from implement
- You merge after CI is green and review approves
- Validation before commit: `npm run validate:release`
