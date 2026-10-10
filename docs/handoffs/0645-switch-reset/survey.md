# Survey: #645 part 2 — `--first` on reset, and the activity refusals

Run: issue #645 (epic #633, wave 3), rigor **in-session**, no UI surface. Branch
`issue-0645-switch-reset` (plugin-named) from `origin/main` at `3d95aec`. Run worktree:
`<run-worktree>`; session root: `<primary>`.

## What part 1 already carries (a sibling pane, `bff4dbe`)

`tx_switch.rs` (334 lines): plan → act → observe → record, one engine for both verbs
(`Target::Existing` / `Target::Fresh`); refusals `orchestrator-pane`, `server-unhealthy`,
`session-not-found`, `session-of-other-pane`, `pane-not-in-profile`, `pane-not-found`;
`SwitchFailure` with `acted`/`created` reconcile messaging. `switch.rs` (160): both verbs'
shared `execute`/`emit_outcome`/`render`. `reset.rs` (45): `PaneReset` args (PANE,
`--as-operator`, `--profile`), delegates to `execute` with `Target::Fresh`. Test suites:
`tests/pane_verbs/switch.rs` (649 lines, the shared `Case`/`both`/`one_case` runner both verbs
use) and `reset.rs` (288). Verified against the issue's acceptance test-kit bullets: every
in-radius bullet has a green, behavior-asserting test (see decisions.md).

## The delta (this run's deliverable)

1. `--first TEXT` on reset: after the record write, queue the text to the new session through
   the harness API — `HarnessPort::send_prompt(port, session, text)` — never by typing into the
   TUI (I4). The "order ran in the wrong session" incident as a test: the prompt lands in the
   NEW session and nowhere else (no other session, no other pane, zero keystrokes).
2. The activity refusals (issue scope, per pfleet #266 story 271): plan-phase refusals for a
   pane whose current conversation is not idle or holds a question —
   `HarnessPort::session_activity(port, session)` with a small closed `Activity` answer.
   A pane with no session of record must NOT be refused (reset is doctor's remedy for exactly
   that pane).

## Why the ports need the one carve-out (operator decision, #700 precedent)

The epic's fixed Ports list has no prompt or activity method; `ports.rs` is #637-frozen; the
harness fakes are #684's; the real adapter (#642 part 3, open) is unverified for prompts (the
#635 spike could not exercise the API without a model call). Default-armed trait methods break
none of the in-tree impls (test doubles in verb test files, the testkit fake, the CLI's
`Unwired`): the default for `send_prompt` returns a stable refusal the real adapter later
replaces; the default for `session_activity` answers idle (permissive seam — refusals fire only
where a port actually reports activity), so current behavior is unchanged everywhere the seam
is not implemented.

## Reuse & Analogous-Feature map

- **Extend, never copy:** the verbs' shared path (`switch.rs::execute`) gains the first-message
  pass-through; `SwitchRequest` (in `tx_switch.rs`, ours) gains `first: Option<String>`; reset's
  clap `Args` gains `--first` (ruling 2: verb flags live in the verb's own file).
- **Test doubles in our own files:** `WriterInSelect` (`switch.rs:478`) is the wrapper-double
  pattern — a `HarnessPort` decorator in the verb's test file. The incident test and the
  activity refusals fake through wrappers of exactly that shape over `FakeHarness`; `doctor/rig.rs`
  (#647's) is never touched.
- **Runner:** every new case rides `both`/`one_case` (envelope + exit-parity + I4 no-keystroke
  come free); refusal tables extend `reset_refusals_create_nothing`'s shape.
- **Codes:** constants in the engine/verb's own files via `PaneError::Refused` (ruling 3);
  kebab-case, stable. The `PaneError` enum itself is closed — no new variants, only `Refused`
  codes (the port default's "unsupported" answer must also be a `Refused` code, stable, so
  #642 part 3 can replace it without an enum change).

## Boundaries

See decisions.md (operator carve-out + issue blast radius). Verification: narrow
`cargo test -p holler-cli --test pane_verbs` (and `--lib -p holler-pane` for the engine's unit
tests); the plugin owns the workspace command. No `Cargo.toml`, no testkit, no adapter, no
other verb's files. One cargo at a time.
