# Issue tracker: GitHub

Issues for this repo live on GitHub (`Performant-Labs/holler`). Use the `gh` CLI. Infer the repo from `git remote`.

- **Read**: `gh issue view <number> --comments`
- **Create**: `gh issue create --title "..." --body "..."`
- **Comment**: `gh issue comment <number> --body "..."`
- **Sub-issues**: GitHub native parent/child. Where that is unavailable, put `Part of #<epic>` at the top of the child body.

Spec sources, in order: the issue referenced by the branch or commits; then binding ADRs in `docs/adr/` (`ADR-0001.md`–`ADR-0006.md`); then `docs/protocol/v2.md` for wire-level contracts.
