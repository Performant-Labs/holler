# Brief: #645 part 2 — `--first` on reset and the activity refusals

Repo: `Performant-Labs/holler`. Issue: #645 (epic #633, wave 3; the issue is the spec). Rigor:
**in-session**. UI surface: no (D and U are N/A). Branch: `issue-0645-switch-reset` from
`origin/main` at `3d95aec`. Run worktree: `<run-worktree>`; session root: `<primary>`.

## Objective

Close #645's remaining acceptance on top of part 1 (`bff4dbe`, the session-switch transaction):
queue reset's first message through the harness API and refuse a pane whose conversation is not
idle or holds a question — through the operator's surgical carve-out (decisions.md; the #700
precedent): two **default-armed** `HarnessPort` seam methods inside this story's radius.

## Acceptance criteria

- [ ] `holler pane reset PANE [--first TEXT]`: the text is queued to the NEW session through
      `HarnessPort::send_prompt` after the record write, in the same transaction; never any
      keystroke (I4); `--first` absent → behavior identical to part 1 (no port call added).
- [ ] The "order ran in the wrong session" incident, as a test: with another pane (`Q`) and the
      old session present, `--first` lands in the new session ONLY — no prompt to the old
      session, none to any other pane, zero Herdr/host calls.
- [ ] `send_prompt` failure after a successful record is reported honestly: the run names that
      the session was created, shown and recorded but the first message did not land (a stable
      failure/refusal code; exit per ADR-0021 §9 class), and no reconcile step is appended (the
      record is correct).
- [ ] Plan-phase refusals (before any write, via `HarnessPort::session_activity` on the pane's
      CURRENT session of record): a busy conversation and a held question each refuse with a
      stable code and change nothing; a pane with NO session of record is not refused (doctor's
      remedy case); a switch (no `--first`) is unaffected.
- [ ] The port defaults are permissive seams: `send_prompt` default = a stable
      "prompt-unsupported" refusal; `session_activity` default = idle. No existing impl changes
      behavior; the testkit fake and the real adapter keep their own stories (#684, #642 part 3).
- [ ] Every new case rides the shared runner: envelope-valid JSON via `check_envelope`, one text
      line, exit codes identical across formats (ADR-0021 §9: 0 ok / 1 failure / 2 usage /
      3 refusal — the epic's amendment; the issue's older wording is superseded).
- [ ] This verb's rows updated where part 1 set precedent: `ADR-0003.md`, `cli-surface.txt`.
- [ ] New `.rs` content rustfmt-clean (ruling 4: existing files are not reformatted — the
      crate-wide drift is out of scope), clippy clean on touched targets, no new `unsafe`,
      narrow suite green.

## RED policy (test-first, honestly)

The feature does not exist: T's new cases (the incident test, the activity refusals, `--first`
plumbing) MUST fault against current main — that is the RED and F's contract. If any new case
is green on contact, T records it with evidence and O surfaces it before F runs (no fake RED,
no mutation). Engine unit tests in `holler-pane` ride the same rule.

## Boundaries (operator carve-out + issue blast radius; the parallel panes own everything else)

- **F:** `holler-pane/src/ports.rs` (**only** the two default-armed methods, the closed `Activity`
  enum and the default's `RefusalCode` const — the carve-out's whole extent),
  `holler-pane/src/tx_switch.rs`, `holler-cli/src/pane/reset.rs`,
  `holler-cli/src/pane/switch.rs`, this verb's rows in `docs/adr/ADR-0003.md`,
  `docs/adr/ADR-0021.md` (A W-1: the `PROPOSED (645b)` codes row :453, the deferred item
  :669-671, §8's "at most seven port calls / they send no prompt" :376-378 — this story's
  rows, part 1's precedent) and `crates/holler-cli/tests/fixtures/cli-surface.txt`.
- **T:** `crates/holler-cli/tests/pane_verbs/reset.rs`, `switch.rs` (their shared runner),
  and engine unit tests beside `tx_switch.rs` where the fakes allow. Wrapper doubles live in
  these files only (the `WriterInSelect` pattern).
- **Never:** the testkit (`holler-pane-testkit/**`, #684's), `adapter-opencode/**` (#642's),
  `pane/wiring.rs` (#649's), `doctor/**` incl. `rig.rs` (#647's), `launch/**` (#644's),
  `cli.rs`/`main.rs` (#670's), any `Cargo.toml`, `error.rs`'s enum (closed; `Refused` codes
  only), the `Pane`/`Profile` contract records.
- No mutation testing; one cargo at a time; never a live fleet, a running pane or a real
  session; never print or commit a credential. Handoffs in this public repo: placeholders only.

## Handoffs

`docs/handoffs/0645-switch-reset/` (in `<run-worktree>`): `survey.md`, `decisions.md`,
`handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
`handoff-S.md`.

## Verification

Narrow: `cargo test -p holler-cli --test pane_verbs` and `cargo test -p holler-pane --lib`.
Full (pre-merge, plugin-owned): `cargo test --workspace -- --skip
roster_stays_accurate_under_concurrent_body_load`. Known repo-wide instability (local sibling
cargo load, CI macos/ubuntu process tests) is documented in run 0660-output's journal; a flake
is diagnosed and journaled, never routed to F as rework.
