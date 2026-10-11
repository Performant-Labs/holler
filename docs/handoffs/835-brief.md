# [feat(pane): holler pane import leaves panes Holler does not drive alone, and --adopt-claude-code adopts them](https://github.com/Performant-Labs/holler/issues/835)

Part of epic #633 (amended 2026-10-11). Follows the operator decision recorded on #654 (the Claude Code panes the import would adopt), option (b), changed to a default plus a switch.

## Problem
`holler pane import` registers every pane directory under the state root as an OpenCode pane and has no way to leave a pane out. Some fleet panes run a Claude Code CLI session instead of an OpenCode server, and the old per-pane files do not say so: they look identical to OpenCode panes. A real import would record those panes with the wrong harness. #654 lists this as an open operator decision before the real import.

## Work
1. **Default: leave a pane Holler does not drive alone.** `pane import` does not register a pane whose harness is not OpenCode. It reports an info finding for it (a new finding code, named in the contract amendment) and writes nothing for that pane. This is not a block: the import proceeds for the other panes, and a dry run shows the same finding. A pane left alone shows as `unregistered-herdr-pane` in a whole-fleet `doctor` pass, which is expected.
2. **Switch: `--adopt-claude-code`.** With it, the import also adopts those panes. Until the Claude Code harness (#831) has merged, the switch is refused as `usage` (exit 2), naming #831, and nothing is written. After #831 it registers them with that harness.
3. **How the import tells the harness apart is NOT decided here.** The old files cannot tell, and `HostPort::ps` returns process ids only (no command line). The first step of the story is the contract amendment (the epic's amend-first rule) that adds the read-only signal, for example the pane's foreground command name, and the rule that uses it. A pane whose OpenCode server is merely down must still be adopted and flagged `server-down`, not skipped; so health on the recorded port is not the signal.
4. **Surface.** The flag is added to the verb's `cli-surface.txt` line and to ADR 0003's row for this verb. `--format=json` shows the new finding like every other finding.

## Fixture
A scratch state root with one OpenCode pane directory and one pane directory whose pane runs a different foreground command, on fakes from the test kit: default run records the first and reports the second as left alone; with `--adopt-claude-code` before #831, exit 2 naming #831; a dry run records nothing in both cases.

## Pipeline
`rigor: in-session`. Wave 1 of this amendment. Builds on #650 (merged). Needs no real ports, so it does not wait for #649. Not a dependency of #654 while option (a) of #654 (an operator-made state root of OpenCode panes only) stays available as the fallback; if the operator chooses to wait for this story instead, it becomes one.

## Rules
See the epic (#633): public repository (no personal host, tailnet or account names), the amend-first rule for the contract, `in-session` rigor, no story touches a live fleet, a real Herdr session, a running pane or a real Claude or OpenCode session: verification is the test kit and scratch instances. A subscription login is never read, copied or logged; env var NAMES only (invariant I7).

Filed by an AI agent (Claude Code) at the operator's direction, 2026-10-11, from the real-fleet dry run of `pane import` that showed Claude Code panes cannot be told apart from OpenCode panes in the old files.
