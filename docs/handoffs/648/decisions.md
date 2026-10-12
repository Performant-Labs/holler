# Decisions: #648, the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ

Run handoff directory `docs/handoffs/648` · branch `issue-648-implementation` · uiSurface false (D and U are N/A).
Append-only; every phase adds its own entry.

## A (Phase 3, up-front plan review): 2026-10-11T19:49-06:00

- **Decided:** PASS on `docs/handoffs/648-brief.md` at `1c81e2d` (sha256 `1f12c6140e9e6508...`). There are no blocks, and twelve warns, eight of them binding. The detail is in `handoff-A.md`.
  - **The objects and the layers are right.**
    - `roster_cmd.rs` keeps the hub call and joins the hub's rows to the registry CLI-side, reusing `pane::list::PaneRow` and `output::emit`/`emit_error`.
    - The registry is read through `Wiring::connect`'s ports and `ProfileScope`.
    - The hub, the wire and `holler-pane` are untouched, and nothing persists.
  - **The binding warns:**
    - **W-1:** the seam. The hub call comes first. A deferred source of ports, which `main.rs` builds around `Wiring::connect`, is called only after a reply and never under `--server`. One pure function takes the registry as ports, error or "not read". Under `--profile`, a failed `connect` is R4's error, and `scope.resolve` is the only read.
    - **W-2:** the pair rule is one named predicate, pinned to `say_cmd::resolve_pane_target` by a test. Moving it into `pane::list` is the approved alternative.
    - **W-3:** ADR 0003 lines 91 and 99 and the help at `cli.rs:63` and `:71-75` get a roster carve-out. Exception approved.
    - **W-4:** three output modes. Every roster `usage` is a stderr line. Other errors go through `emit_error`. Hub errors are `unavailable`, except `RemotePolicyRefused`, which gets a roster-owned open refusal code (exit 3).
    - **W-5:** the six cells are taken by position in `pane::list::COLUMNS`, pinned by a test.
    - **W-6:** an in-process hub call runs only under the state-dir lock, with a scratch `StateDir`. Golden rows carry no wall-clock cell.
    - **W-7:** `render_table`'s fixed-width cadence is extended, and the legend is `--help` text.
    - **W-8:** three ADR-0021 sentences are added (parked panes; the wiring's users; the open code). Exception approved.
  - **Not binding:**
    - W-9: `note:` against `warning:`, and a JSON signal for a partial read (for O);
    - W-10: one join function, and a key-collision test;
    - W-11: the stale line 133;
    - W-12: spec observations for T and S.
- **Assumed:**
  - Clap derive uses the doc comment of `Command::Roster` as the long help of `holler roster --help` (not probed).
  - The deferred source in W-1 compiles in the shape described (a closure over an `Option` or `OnceCell` holding the `Wiring`). F may pick another shape with the same order and ownership.
  - `RefusalCode::from_static` accepts a new roster-owned code as it does `say_cmd.rs`'s three. Its name is F's.
- **Hedged:**
  - **Warns instead of a BLOCK.** W-1 to W-8 are binding warns, not a BLOCK (the #649 precedent). None changes an acceptance criterion, and a BLOCK on an unchanged brief would stop the run for an amendment the warns supply.
  - **Overriding R10.** W-3 and W-8 approve Blast radius exceptions that override R10's "nothing else in that file changes". That clause was decided about line 94 alone, and the project rule requires the ADR to be updated in the same change. The operator can overrule.
  - **The alternative for W-4.** R3's form for `RemotePolicyRefused` (no envelope) is left to O.
- **Evidence:**
  - **Read in full:** the brief's refresh, scope, acceptance, blast radius, decisions and E11; `roster_cmd.rs`, `main.rs`, `output.rs`, `pane/list.rs`, `say_cmd.rs`, `pane/wiring.rs`.
  - **Read in part:** `wiring/store.rs`, `transport.rs`, `cli.rs`, the hub's `roster.rs` and `control_server.rs`, `profile.rs`, `ports.rs`, `profile_scope.rs` (both), `vocab.rs`.
  - **ADRs:** ADR-0003 lines 88-110; ADR-0021 lines 245-275, 316-330, 685-720, 760-830, 862-895.
  - **Tests:** `roster_cli_test.rs`, `hold_cli_test.rs`, `grant_cli_test.rs`, `pane_verbs/{main,list}.rs`, `pane_verbs/process/{main,docs_rows}.rs`, `pane_integration/main.rs`, `verb_harness/mod.rs`, `docs_pane_control_test.rs`; and the `Cargo.toml` test targets.
  - **State checked:** `git fetch` (`origin/main` is still `bda1069`); `gh issue view 839` and `648` (both open).

## T (Phase 4, author / RED): 2026-10-11T20:15-06:00

- **Decided:** RED is valid. The detail is in `handoff-T-red.md`.
  - **What was authored.** 29 tests:
    - 25 in-process in the new target `roster_panes_test`, with 4 goldens;
    - 4 process tests in `roster_cli_test.rs`. One of them (clap's usage errors) is a pin that passes today.
  - **The API.** It is named here, as W-1 leaves it to T and F:
    - `roster_cmd::PANE_COLUMNS`;
    - `Registry::{Ports, Failed, NotRead}`;
    - `render(reply, registry, roster, choice, sink) -> i32`;
    - `run(roster, choice, source, sink) -> i32`.
  - **The text layout** (W-7): HOLD is padded to 20, then PANE 20, POS 6, PROFILE 12, HEALTH 10, SHOWN 20, and DRIVEN unpadded.
  - **Test changes the brief assigns:**
    - `section_4_marks_the_roster_extension_later` is replaced by `section_4_documents_the_roster_columns`;
    - the `roster --profile` guard is deleted;
    - `roster_profile_is_refused_with_the_roster_story` and `ROSTER_PROFILE_REFUSAL` are deleted.
- **Assumed:**
  - Stderr `note:` lines are printed in all three modes, `--format=json` included (R3 and W-9 say the roster reports a partial read only on stderr).
  - Legacy `--json` errors go to stderr as text (W-4).
  - `RemotePolicyRefused` under `--format=json` is pinned only as a refusal-class envelope with today's message. Its code name is F's.
- **Hedged:**
  - **W-2.** It is pinned by behaviour (the roster's `mismatches` against `resolve_pane_target`), not by naming a predicate, so either of W-2's placements passes.
  - **AC 13 (a)** is tested twice: in-process, to show R3's check is first in `run` (before the policy refusal), and as the process test the AC names.
  - **Fixtures.** The fixture rows are serialized from the hub's real `Row` type, so they are the reply's real shape and also serve as W-10's key-collision check.
- **Evidence:**
  - **Runs on the clean tree:**
    - `cargo test -p holler-cli --test roster_cli_test`: 6 passed, 3 failed, all three on their assertions;
    - `roster_panes_test` and `docs_pane_control_test` fail to build on exactly the four new API items;
    - `pane_cli_process`: 35 passed.
  - **A scratch stub of today's behaviour behind the new API** gave 0 of 25 passing, every failure an assertion.
  - **A scratch trial implementation** gave 25 of 25 and 9 of 9; with it the docs test fails only on R9's guide text.
  - **Restored.** Both scratch versions were reverted from scratchpad copies; `src/` is unchanged.
  - **Checks:** `lint.sh` exit 0, `changelog-check.sh` exit 0, `cargo clippy -p holler-cli --tests -D warnings` clean.

## F (Phase 6, implement): 2026-10-11T20:30-06:00

- **Decided:** the roster is implemented against T's API, and every authored test is GREEN. The detail is in
  `handoff-F.md`.
  - **The API, as T named it:** `roster_cmd::{PANE_COLUMNS, Registry, render, run}`. `RosterResult` is gone. Also
    public: `HELP_LEGEND` (the `--help` legend, W-7) and `REMOTE_POLICY_REFUSED` (W-4's open code).
  - **The open code is `remote-policy-refused`.** It is a refusal (exit 3), only in `--format=json`'s failure envelope;
    the text forms keep today's line. It is added to ADR-0021 section 9's open codes (W-8).
  - **Usage errors come before any dial.** `run` refuses `--profile` with `--server` first (R3), then a malformed
    `--profile` name. So `roster --profile '!!!'` with no hub is exit 2, not the no-hub exit 1. `render` checks the name
    again for a direct caller.
  - **W-2: one named predicate in `roster_cmd.rs`** (`shown_driven_mismatch`), whose doc names both twins.
    `say_cmd.rs` is untouched, so there is still no code overlap with #839.
  - **W-5: each cell is looked up by its column's name in `pane::list::COLUMNS`** at run time. T's
    `the_cells_are_pane_lists_cells_for_the_same_record` pins it.
  - **The table is one formatter.** `table_line` pads every cell but the last to a `WIDTHS` entry, for the header and
    every row; HOLD is padded as a cell.
  - **`roster --json` keeps today's bytes but for the six keys:** written as it was, with no newline added after the
    document. Text and the envelope go through `output::emit`.
  - **W-12: a `--profile P` that joins no row prints `(no sessions of profile P on the hub)`**, not the false
    `(no sessions on the hub)`. No test pins it.
  - **`main.rs`'s deferred source holds the `Wiring` in a `std::cell::OnceCell`**, not T's `Option` plus `move`; the
    order and ownership are W-1's.
  - **Docs:** the guide gets R9's paragraph verbatim (wrapped) and loses section 12's bullet. ADR 0003 gets W-3's
    two carve-outs and loses R10's sentence. ADR-0021 gets R9's two hunks and W-8's three sentences.
  - **CHANGELOG:** the brief's entry, corrected. "No ADR 0003 change" is now "additive, so MINOR under ADR 0003", since
    W-3 edits two ADR 0003 sentences. It also says `--profile` with `--server` is a usage error and a registry that
    cannot answer prints one `note:` line. It sits after the #652 entry, not at the top, because `origin/main`'s #851
    (`d6ae539`, merged after this branch's base) inserted at the top.
- **Assumed:**
  - A `PaneError::NotImplemented` from `Wiring::connect` (it never answers one today) is treated like a store's: no
    note.
  - A hub reply that is not an object, or has no `rows` array, prints as an empty roster (as `render_table` did).
- **Hedged:**
  - **Two code docs outside the blast radius stay slightly untrue** (A's W-8, not binding): `reconcile::shown_differs`
    names the roster among its readers, and `PaneRow.driven` says "printed, never compared". They are left for O's
    follow-up, as A directs.
  - **Merge with `origin/main`.** `d6ae539` (#851) touches `cli.rs`, `main.rs`, ADR 0003, the guide and CHANGELOG, in
    hunks this change does not touch; the branch was not merged, as F does not commit.
- **Evidence:**
  - `cargo test -p holler-cli --test roster_panes_test`: 25 passed. `--test roster_cli_test`: 9 passed.
  - `--test docs_pane_control_test --test docs_cli_test --test cli_surface_test`: 16, 3 and 3 passed.
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean. `bash scripts/lint.sh`: exit 0.
    `bash scripts/changelog-check.sh`: ok. `roster_cmd.rs` is 578 lines.
  - **The full workspace suite** (the pipeline's `test.unit.command`, with `--no-fail-fast`): exit 0, 2402 passed,
    0 failed, 32 ignored.
    - An earlier run without `--no-fail-fast` stopped at the known flake #847
      (`remote_admin_traffic_never_moves_the_bodys_last_seen`).
    - That test passed 3 of 3 runs alone and passed in the full run.
    - This change makes no call on its `--server` path. The hub moves `last_seen` only from a token's WebSocket circuit
      (`circuit.rs:834`), never from the control socket.
  - `docs/handoffs/648/evidence.md`: the unchanged-source facts the diff relies on.

## T (Phase 7, verify / GREEN): 2026-10-11T20:46-06:00

- **Decided:**
  - GREEN: all 29 authored tests pass, and the full workspace suite is exit 0 (2402 passed, 0 failed, 32 ignored).
  - Tier 2 is clean, so the verdict is PASS. No test was repaired: F flagged none, and none looked wrong.
  - I appended a T section to `evidence.md` for the unchanged-source facts the tests rely on: the hub `Row`,
    `check_envelope`'s class rule, `FakeProfileScope`'s order, the loopback-policy message and
    `resolve_pane_target`.
- **Assumed:**
  - rustfmt is not a gate, because neither `ci.yml` nor `lint.sh` runs it. The two rustfmt diffs in `roster_cmd.rs`
    are advisory.
- **Hedged:**
  - W-12's empty `--profile` line is still unpinned (advisory).
  - The merge of `d6ae539` (#851) should re-run the docs tests and the changelog check.
- **Evidence:**
  - Seven mutations of `roster_cmd.rs`, all reverted, and each failed at least one test: the pair rule, the join, the
    NotRead note, the profile scope, the profile filter, the policy code and the de-duplication.
  - Tier 1 commands: see `handoff-T-green.md`.
