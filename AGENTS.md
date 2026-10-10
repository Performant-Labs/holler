# Holler — notes for OpenCode agents

## The coding pipeline

Holler stories run through the **Aftersight OpenCode coding pipeline**, pinned to
`opencode-pipeline-v0.1.0`. Its configuration lives in:

- `.aftersight/pipeline-pin.json` — the pinned release (shared with the Claude pipeline story, #674)
- `.aftersight/pipeline.config.json` — the test command, path globs, toolchain checks and the
  stage-model table (shared with #674 for `test.unit.command` only)
- `opencode.json` — the plugin entry and one agent block per role; `.aftersight/prompts/` holds the
  generated role prompts (regenerate both with Aftersight's `scripts/pipeline-shim.mjs`, never by hand)
- `.aftersight/releases/` — the linked release, per machine, never committed

How the stages work, which executors exist and what the refusals mean:
[Aftersight's stage runbook](https://github.com/Performant-Labs/aftersight/blob/main/docs/runbooks/pipeline-stages.md).
Installing or upgrading the pipeline in a consumer repo:
[the install guide](https://github.com/Performant-Labs/aftersight/blob/main/docs/runbooks/pipeline-install.md).

**Dispatch rule:** the orchestrator dispatches each role agent with the `task` tool and then records
the stage with `stage run`; only F (implement), which runs on the `claude-cli` executor, is *started*
by `stage run` (Aftersight#604). Use the shell CLI when a delegated pipeline tool answers empty.

## Holler's rules

The full rules live in `CLAUDE.md` and apply here too. The short list: Conventional Commits, `cargo
fmt`, clippy clean, no new `unsafe`, never touch a live fleet, a running pane or a real Herdr
session, never print a credential, and never print or commit a secret.
