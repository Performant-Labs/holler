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

---

## Contract amendment (amend-first; resolves the BLOCK in `docs/handoffs/835/handoff-A.md`)

Everything above this line is the issue, verbatim, and stays the source of truth. This section is the contract amendment the issue's Work item 3 puts first. It decides what the issue leaves open, in the order of handoff-A's findings. Where this section and the issue differ, it says so and why; the issue wins everywhere it is explicit. The amendment lands first: ADR-0021 and the port contract change before any test or code (the pipeline's T and F follow it).

### 1. The signal (handoff-A finding 1)

- **A new read-only `HostPort` method beside `ps`:** `foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>` (F matches the exact signature style of `ps` in `crates/holler-pane/src/ports.rs`). It answers the command **name** of the foreground process of every live pane in the named tmux session, one entry per pane, in the order tmux lists them. A session with no pane process answers an empty list. A missing session is `pane-not-found`, exactly as `ps` answers.
- **Names only.** It reads command names, never an argv, a command line, or Claude Code's configuration directory (spike decision 4; invariant I7). The tmux adapter reads `#{pane_current_command}` from the same `list-panes -s -t =NAME:` listing `ps` uses, with a strict parser beside `parse_ps`, and maps errors through the adapter's existing `Call` and `refused()` path. It does not use Herdr's `pane.process_info`, which returns full argv and command lines.
- **Default-armed, not required** (the #645 precedent). A required method would force edits to `Unwired` in `wiring.rs` (owned by #649, running now), the launch and apply rigs and `ports_test.rs`. The default **fails loudly**: it returns a refusal with an open code in the style of `send_prompt`'s `prompt-unsupported` (F matches that default's pattern and names the code, e.g. `command-names-unsupported`, exit 3). It never answers an empty list, because an implementation that does not override it would then import a Claude Code pane as OpenCode.
- **`ps` is not changed** (it would reach #647's frozen `reconcile/observe.rs`).
- **The records.** Add the method to ADR-0021 section 2's port table and its "amended since" sentence. The epic's contract block is a GitHub issue body: the operator edits it (not the pipeline); the brief only flags it.
- **The fake and its pins.** `FakeHost` answers, for a session, the basename of argv[0] of each process `run` started in it (as tmux reports `sleep` for `env -- sleep`), and an empty list when none; a new `HostOp` variant whose `as_str` equals the adapter's `OP_*` constant; one conformance case in the test kit's host suite and one `Mutant` break, with the `Mutant` wrapper delegating the new method so `the_unbroken_host_wrapper_passes` still holds. The issue's fixture then needs no new scenario method: `host.run(name, cwd, ["claude", ...])` in the left-alone pane's session.

### 2. The rule, for every answer (finding 2)

- A pane whose session has a live pane whose command name is exactly **`claude`** is **left alone** by default, and **adopted** with `--adopt-claude-code` once #831 has merged.
- **Any other answer is imported as OpenCode, as today**: `bash` (the old launcher's OpenCode panes run a bash script, `oc-holler`, so they report `bash`), `opencode`, or an empty session. This keeps a pane whose OpenCode server is merely down adopted and flagged `server-down` (issue Work item 3), keeps #650's fixtures (which start no process in any pane's session) green, and matches the old launcher's own test for a Claude Code process (`claude` on argv[0]).
- **Only `claude` is left alone.** No other command name is classified.
- The name lives in **one constant in `holler-pane`** (so #831 and #832 reuse it); nothing classifies command names anywhere else.
- **An unreadable signal fails closed.** If the new read returns an error other than `pane-not-found` (which the existing `ps` path already turns into `no-tmux-session`), including the default's loud refusal, the pane gets a new `problem` finding `command-unreadable` and no record, and a real run is refused as `import-blocked` like any other problem. It is never recorded as OpenCode on a guess. (This follows the import's fail-closed stance, e.g. `sources-disagree`.) The finding's message never quotes observed text except through `findings::quoted`.

### 3. The finding (finding 3)

- **Code: `left-alone-not-driven`** (final spelling in the ADR). **Level: `info`, as the issue says** ("an info finding"), not handoff-A's suggested `skipped`; the issue is the source of truth, `info` already exists, and the code differs from `harness-not-opencode` because the switch adopts these panes and not that one's. **Action: `skipped`** for the pane's row, built through `Outcome::left_alone` with a constructor beside `not_opencode`. No new `Action` and no new `Level`. One finding and no others for such a pane; the import makes no harness-health or session calls for it.
- `harness-not-opencode` is untouched. Handoff-A noted the two outcomes differ in level; this is a deliberate, recorded difference and S should treat it as such.
- It is a new code, stable once merged; add it to the finding table in ADR-0021 section 13 and the verb's help text. `--format=json` shows it like every finding (`level` `info`, `action` `skipped`).
- A pane left alone shows as `unregistered-herdr-pane` in a whole-fleet `doctor` pass: expected (issue Work item 1).

### 4. The switch (finding 4)

- **Keep `usage` (exit 2), as the issue says.** Reason: until #831 merges the value is a refused argument for this build, and the issue chose it. Handoff-A noted the CLI's one "story not landed" answer is `output::not_implemented(story)` (code `not-implemented`, exit 1); the brief records that the issue's `usage` was chosen over it. **Reuse the helper's wording**: build the "#831" text with `not_implemented_message(831)` so it has one spelling.
- **One site:** `Input::read`, step 1, before any file is read or any port is called, beside the existing `usage` checks in `crates/holler-cli/src/pane/import.rs`, so a dry run is refused too and nothing is written. #831 replaces that one place.
- **Documented in:** ADR-0021 section 9's `pane import` row, the `PaneImport` help text, ADR 0003's row for the verb, and `crates/holler-cli/tests/fixtures/cli-surface.txt` (a line with the flag).

### 5. After #831 (finding 5): out of scope here

No story owns the post-#831 adoption, and #835 cannot build it (`HarnessKind` has only `Opencode`). This story builds the **refusal**, not the adoption. Recorded for the owner: adopting into an existing `fleet` profile needs a change to the import's write step (`profile-exists` today refuses adding records to a profile that exists) or another route such as `pane launch --profile fleet` with #831's harness. The operator should add a line to #831 (or file a follow-up); the pipeline cannot, and does not.

### 6. Blast radius, by owner (finding 6)

- `crates/holler-pane/src/ports.rs` (frozen, #637's): the new default-armed method only.
- `holler-pane-testkit`: `FakeHost`, `HostOp`, the host conformance suite, the `Mutant`.
- `holler-adapter-host`: `src/lib.rs`, `src/tmux.rs`; **new adapter tests go in a new file**, not `tests/fake_tmux_test.rs` (818 lines, past the ~800 guard).
- `holler-pane` import: `import/assess.rs`, `import/observe.rs` (one more read on the same per-pane thread; update its "at most three port calls" note), `import/findings.rs` (two new kinds in `ALL`), `import.rs`.
- `holler-cli`: `pane/import.rs` (flag, `Input::read`, help), the import rig's read allow-list (`tests/pane_verbs/import/rig.rs`, which admits `HostOp::Ps` only), `cli-surface.txt`.
- Docs: ADR-0021 sections 2, 9 and 13 and the closing lists only; ADR 0003's row; `CHANGELOG.md` (an `## [Unreleased]` entry linking #835). Do **not** touch `wiring.rs` (#649), and keep ADR-0021 edits to those sections because #828 edits section 8 and `CHANGELOG.md`; #652's guide describes the import and should be told of the new default and flag.
- Files near the 900-line lint: check `wc -l` on every touched file before extending it.

### 7. #832 (finding 7)

#832 (same wave) must reuse the `HostPort` read and the one rule constant rather than add a second signal (such as Herdr's screen-detected agent). A note on #832 is for the operator.

### Fixture additions

On top of the issue's fixture: a pane whose command read errors (not `pane-not-found`) gives `command-unreadable`, no record, and a real run refused as `import-blocked`; a session with no process (the #650 default world) is imported as OpenCode; a pane whose command is `bash` is imported as OpenCode; every pane left alone writes nothing in a real run and in a dry run.
