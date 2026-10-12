# [feat(cli): holler --version prints the commit the binary was built from](https://github.com/Performant-Labs/holler/issues/851)

Part of epic #633 (supports the cutover, #654). Raised by the operator, 2026-10-11.

## Problem
`holler --version` prints `holler 0.4.0` for every build made since release v0.4.0, because the workspace version stays at 0.4.0 until the next release. On 2026-10-11 three binaries answered `--version` as follows, and only one of them had `holler pane`: a Homebrew build of the release (0.4.0, no `pane` verbs), a build from `main` (0.4.0, all `pane` verbs), and a stale installed build (0.2.0). A person cannot tell from `--version` which code they are running. The live cutover (#654) must run the new build, and `--version` cannot confirm that it does.

## Work
1. `holler --version` (and `-V`) prints the version and the commit it was built from, for example `holler 0.4.0 (bda1069)`. The first token and the version stay exactly as today, so anything that parses the first line keeps working. A build from a dirty tree appends `-dirty`.
2. The hash comes from the build (a `build.rs` in `holler-cli`, `git rev-parse --short=7 HEAD`). When there is no `.git` (a source archive or a packaged build), it reads an optional build-time variable `HOLLER_GIT_SHA`, and otherwise prints `unknown`. The packaged release build (Homebrew) sets it, so a release prints its tag's commit.
3. The same string is used wherever the version is shown (`--version`, and any `version` field a verb already prints; do not add a new field to a frozen JSON envelope).
4. Update the CLI surface fixture and ADR 0003's row if the version line is pinned there, and add a CHANGELOG entry.

## Acceptance
- A build from a git checkout prints the 7-character hash of `HEAD`; a build with `HOLLER_GIT_SHA=abc1234` and no `.git` prints `abc1234`; a build with neither prints `unknown`. Each is a test (the second and third can build a tiny fixture crate or run the logic as a pure function).
- `holler --version` on a dirty tree ends `-dirty`.
- No other output changes; golden files are unchanged.

## Pipeline
`rigor: in-session`. A small story; no contract amendment is needed (the version line is not part of the pane contract). Not a dependency of #654, but the cutover runbook should say to confirm the binary by `--version` once this has merged.

## Rules
See the epic (#633): public repository (no personal host, tailnet or account names), `in-session` rigor, no story touches a live fleet, a real Herdr session, a running pane or a real Claude or OpenCode session.

Filed by an AI agent (Claude Code) at the operator's direction, 2026-10-11.
