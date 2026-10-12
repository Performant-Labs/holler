# Handoff-F: Phase 6 - #648 the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ

**Date:** 2026-10-11
**Branch:** issue-648-implementation (worktree `.claude/worktrees/0648b-roster-shown-driven`, head `4bc2088`, plus this
phase's uncommitted change)
**Issue:** #648

## What was done

- `crates/holler-cli/src/roster_cmd.rs` (138 to 578 lines): the roster, rebuilt on T's API.
  - **Public:** `PANE_COLUMNS`, `Registry::{Ports, Failed, NotRead}`, `run(roster, choice, source, sink) -> i32` and
    `render(reply, registry, roster, choice, sink) -> i32`. Also `HELP_LEGEND` (the `--help` legend) and
    `REMOTE_POLICY_REFUSED` (the roster's open code). `RosterResult` is gone.
  - **Private:** the output mode, error routing, the registry read, the join, the pair rule, and the text and JSON
    renderers.
  - **Unchanged:** `last_turn_display` and `hold_display`.
- `crates/holler-cli/src/main.rs`:
  - **Removed:** `ROSTER_STORY` and the `roster --profile` refusal.
  - **`dispatch`** hands `roster` the whole `FormatChoice`.
  - **`run_roster`** builds a `Sink` over stdout and stderr, and a deferred `Wiring::connect` source that holds the
    `Wiring` in a `OnceCell`.
  - **Docs:** `print_leaf_result` no longer names `Roster`, and `dispatch`'s doc names `json_explicit`.
- `crates/holler-cli/src/cli.rs`:
  - `#[command(after_help = crate::roster_cmd::HELP_LEGEND)]` on `Command::Roster`.
  - The `--profile` field comment, which described the refusal.
  - The help of `--json` and `--format` gets a roster carve-out (W-3).
- `docs/pane-control.md`: section 4's "later" sentence is replaced by R9's roster paragraph, verbatim, wrapped at 120.
  Section 12's roster bullet is deleted.
- `docs/adr/ADR-0003.md`: W-3's carve-outs at line 91 (`--format=json` is `--json` on every verb but `roster`) and line
  99 (the roster's envelope and `--json` shape). R10's "Until #648" sentence at line 94 is deleted.
- `docs/adr/ADR-0021.md`:
  - R9: section 9's "Deferred to #648" becomes "Decided (#648)", and the roster bullet of "Deferred to named stories"
    is deleted.
  - W-8: the parked-pane sentence, the roster among the wiring's users, and `remote-policy-refused` among the open
    codes.
- `CHANGELOG.md`: one `[Unreleased]` / Enhancements entry.
- `docs/handoffs/648/evidence.md` (new), `docs/handoffs/648/decisions.md` (appended) and this file.

## Design decisions

1. **The three modes and the error boundary (W-4).**
   - **Modes.** A private `Mode` is read from `FormatChoice`: text; legacy (`Json`, not `json_explicit`); envelope
     (`json_explicit`).
   - **Errors.** Every error goes through one function, `fail`. A `usage`-class error is `error: <message>` on stderr
     in every mode. Any other goes through `output::emit_error`: `Format::Json` in the envelope mode, `Format::Text`
     otherwise. So a bare `--json` keeps today's `error:` line, byte for byte.
2. **Hub errors keep today's messages (W-4).**
   - **`NoLiveHub`** keeps "no live holler hub reachable at <dir>". Every other `ControlError` keeps its `to_string()`.
   - **The code** is `unavailable` (exit 1), except for `RemotePolicyRefused`, which gets the roster-owned open code
     `remote-policy-refused` (exit 3).
   - **Why that name.** It mirrors the `ControlError` variant, so it is searchable. It also says what was refused:
     this machine's policy refused to dial the `--server` hub, not that a server refused anything. T's trial name was
     `roster-server-refused`; no test pins the name.
3. **Usage errors come before any dial.** `run` refuses `--profile` with `--server` first (R3). Then it parses the
   `--profile` name (`pane::list::profile_name`), so a malformed name is exit 2 even with no hub. W-1's order is
   otherwise exact: the hub call, then the source only after a reply and never with `--server`, then `render`.
   `render` parses the name again, because it is public and the tests call it directly.
4. **One registry read.**
   - **Without `--profile`:** `pane_store.list()`. Its error only empties the columns. `not-implemented` prints no
     note; any other error prints `note: pane registry unavailable (<code>)`.
   - **With `--profile`:** `scope.resolve(P, None)` alone (W-1), and its error is the run's (R4).
   - **`Registry::Failed`** is the store-error path without `--profile` and the run's error with it.
   - **`Registry::NotRead`** prints R3's note. Combined with `--profile`, it is the usage error `run` already
     refuses, for a direct caller.
5. **The join** is `pane_label`: `name.split_once('/')`'s left part, or the whole name. Its doc cites AC 10 and #839,
   so #839 has one place to change (W-10).
6. **The pair rule (W-2) is one named private predicate**, `shown_driven_mismatch`. Its doc names
   `say_cmd::pane_state_refusal` and the hub's `dispatch.rs` twin. I took W-2's first option, not the shared
   `pane::list` function: that one would edit `say_cmd.rs`, which #839 also edits. T's
   `the_mismatch_rule_is_say_pane_targets_rule` pins it by behaviour.
7. **The cells (W-5).** Each of the six text cells is `PaneRow::cells()` at its column name's position in
   `pane::list::COLUMNS`, found at run time, so there is no hard-coded index. T's test compares the cells column by
   column name.
8. **The JSON values (W-5).**
   - **Where they come from.** A private `#[derive(Serialize)] PaneKeys` is built from the joined `PaneRow`'s fields,
     so `pos` and `health` are in `pane list`'s serde forms. A `None` is `null`; nothing is skipped.
   - **How they are added.** Each JSON row is the hub's row object, cloned, with the six keys `extend`ed in.
   - **The two JSON modes.** `--json` is the reply with `rows` replaced. `--format=json` is
     `emit(.., {rows, mismatches})`.
9. **`--json`'s bytes.** Today `roster --json` printed the document with no trailing newline (`print!`), and it still
   does: the document is written as it is and flushed, and a failed write exits 1. Text and the envelope go through
   `output::emit`.
10. **The table (W-7).**
    - **One formatter.** `table_line` serves the header and every row: each cell but the last is padded to its
        `WIDTHS` entry, then one space. The widths are today's eight, then HOLD 20, PANE 20, POS 6, PROFILE 12,
        HEALTH 10 and SHOWN 20, with DRIVEN unpadded. This replaces the two 15-placeholder `format!` strings that
        extending `render_table` literally would have needed.
    - **HOLD is padded as a cell**, the discriminator T found: `reconnecting` overflows CONN on the `io/alpha` row.
11. **No row to print.** An empty reply prints today's `(no sessions on the hub)`. Under `--profile`, an empty result
    prints `(no sessions of profile P on the hub)` instead (A's W-12). Today's line would claim the hub has no session
    when it may have many. No test pins this, and it is in the decisions journal.
12. **The legend (W-7)** is `HELP_LEGEND`, kept in `roster_cmd.rs` next to `PANE_COLUMNS`, so `cli.rs` (now 784 lines)
    grows by one attribute line for it. It is clap's `after_help`, so `-h` and `--help` both show it. It names the six columns, says that POS is read
    from Herdr and never from a name (`hj-c1r2` shows `r2c1`), and gives the summary line, `--profile`, `--server` and
    the two JSON shapes.
13. **`run_roster`'s source** holds the `Wiring` in `std::cell::OnceCell::get_or_init` rather than T's `Option` plus
    `move` plus `let slot = slot`. It is the same order and ownership, and the closure is an ordinary `Fn`.

## Reuse / extend-vs-new

- **Extended:** `roster_cmd.rs`, the object the brief names. At 578 lines it is under lint's 600-line warning, so no
  `roster_panes.rs` was made.
- **Reused, unchanged:**
  - from `crate::pane::list` (the analogue the brief and `list.rs`'s own module doc offer to "the roster, #648"):
    `PaneRow`, `PaneRow::cells`, `COLUMNS`, `NO_VALUE`, `profile_name` and `text_value`;
  - from `crate::output`: `emit`, `emit_error`, `ErrorBody`, `ErrorCode`, `Sink` and `FormatChoice`;
  - `Wiring::connect` and `Wiring::ports`, `ProfileScope::resolve` and `PaneStore::list`;
  - `transport::call` and `ControlCall::roster`, as before.
- **New objects:**
  - `Registry` (W-1's three cases);
  - the private `Mode`, `Registered`, `Joined`, `PaneKeys` and `RosterData`;
  - the predicate (W-2's first option: a third copy, named, documented and pinned).

  No parallel path exists for anything the brief said to extend.

## Architecture notes for A

- **Layers.** The CLI only. `holler-hub`, `holler-pane`, the wire and the test kit are untouched. No dependency is
  added and no protocol field changes.
- **Public interface of `holler_cli::roster_cmd`:**
  - `RosterResult` is removed, and `run`'s signature changed;
  - `render`, `Registry`, `PANE_COLUMNS`, `HELP_LEGEND` and `REMOTE_POLICY_REFUSED` are new.

  `main.rs` was the only user of the old API.
- **The open code.** `remote-policy-refused` is roster-owned and declared with `RefusalCode::from_static`, as
  `say_cmd.rs` declares its own.
- **The wiring.** `main.rs::run_roster` is the roster's only caller of `Wiring::connect`, so `wiring.rs`'s list of
  callers stays true (ADR-0021 ruling 1). ADR-0021's "Wiring as built" now names the roster.
- **Local patterns.** Exit codes are returned, never `exit`ed. Output goes through an injected `Sink`, as the pane and
  profile verbs' does. No `unwrap`, `expect` or `panic`, and no `#[allow]` was added.

## Deviations from spec / wireframe

- **ADR 0003 lines 91 and 99 are edited** (W-3, binding, exception approved). Scope item 6's "no ADR 0003 amendment"
  is about the `--json` shape, which is additive, and R10's "nothing else in that file changes" was decided about line
  94. No "Amended by" header line was added.
- **The CHANGELOG entry** is the brief's text with three changes:
  - "(additive, no ADR 0003 change)" became "(additive, so MINOR under ADR 0003)", since W-3 now edits ADR 0003;
  - one sentence was added on `--profile` with `--server` (exit 2);
  - one sentence was added on the `note:` line.

  It is placed after the #652 entry; the decisions journal says why.
- **The usage check of the profile name comes before the hub call** (decision 3). This adds to W-1's order and does
  not change it.
- **The empty `--profile` line** (decision 11) is W-12, which the brief leaves open.
- No wireframe (no UI surface).

## Tier 1 self-check (incl. tests now GREEN)

```text
$ cargo test -p holler-cli --test roster_panes_test
running 25 tests
... (all 25 named in handoff-T-red.md) ... ok
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p holler-cli --test roster_cli_test
test roster_usage_errors_are_clap_text_in_every_format ... ok
test roster_profile_with_server_is_a_usage_error ... ok
test roster_empty_prints_friendly_message ... ok
test roster_cli_shows_two_sessions_from_one_body ... ok
test roster_json_shape ... ok
test roster_all_shows_gone_row_in_human_table ... ok
test roster_reads_the_real_registry_of_the_local_hub ... ok
test roster_server_does_not_read_the_local_registry ... ok
test roster_cli_prefix_filters_by_label ... ok
test result: ok. 9 passed; 0 failed

$ cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test --test cli_surface_test
test result: ok. 3 passed (cli_surface_test)
test result: ok. 3 passed (docs_cli_test; every_documented_holler_command_parses included)
test result: ok. 16 passed (docs_pane_control_test; section_4_documents_the_roster_columns included)

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)      (no warning, no error)

$ bash scripts/lint.sh        -> exit 0 (roster_cmd.rs 578 lines, not listed; cli.rs 784 was already a 600-line warn)
$ bash scripts/changelog-check.sh   -> changelog-check: ok
```

`holler roster --help`, run with a scratch `HOLLER_STATE_DIR` (`--help` exits before any dial), prints the legend after
the options.

**The full workspace suite** (the pipeline's `test.unit.command`, run with `--no-fail-fast`): `cargo test --workspace
--no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` gave exit 0. Over 153 result lines, 2402
passed, 0 failed and 32 were ignored.

**One flake, not this change.** The first run, without `--no-fail-fast`, stopped at one failure:
`remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen`, where the body's `last_seen` moved from 275
to 276.
- **It is known.** It is open issue #847, "Flake: ... fails under the loaded workspace run", filed 2026-10-11 17:25
  MDT, before this change.
- **It passed again:** 3 of 3 runs of the target alone, and in the full run above.
- **This change cannot move `last_seen`.** It adds no call on the `--server` path. Its local reads (`control/status`,
  `pane/list`) go over the control socket, while `Roster::touch`, the per-frame writer of `last_seen`, is called only
  from a token's WebSocket circuit (`crates/holler-hub/src/circuit.rs:834`).

## Evidence appendix

`docs/handoffs/648/evidence.md`. It covers:
- `PaneRow::cells` and `COLUMNS`, and `PaneRow`'s JSON forms;
- `profile_name`, and `StoreScope::resolve` and `members`;
- the say_cmd pair rule;
- `resolve_format`'s `json_explicit`;
- `emit_error`'s text and JSON forms, `write_line` and `settle`;
- `class_of` on an open code, and `RefusalCode::from_static`'s const check;
- the transport's policy refusal and `ControlError`'s `Display`;
- `Wiring::connect` and `reach_hub`, and `HubPaneStore::list`;
- the hub's `{"rows": rows}` reply.

## Tests that look wrong (for T)

None.

## Known issues

- **Two code docs outside the blast radius stay slightly untrue** (A's W-8, not binding):
  - `holler-pane/src/reconcile.rs:181-183` names the roster among `shown_differs`' readers;
  - `crates/holler-cli/src/pane/list.rs:239-241` says `PaneRow.driven` is "printed, never compared".

  A routes both to an O follow-up, which should also ask whether the roster should flag a parked pane.
- **In production the summary line cannot fire until #839** records DRIVEN (R6). The PANE column fills only where a
  body's token label equals a pane's name (A's W-12). Both are as specified; S should confirm the second against the
  issue's intent.
- **The branch is behind `origin/main` by `d6ae539` (#851),** which touches `cli.rs`, `main.rs`, ADR 0003, the guide
  and CHANGELOG. Each of its hunks is away from this change's hunks: the CHANGELOG entry was placed after the #652
  entry for that reason. F does not merge.

## Files changed

Production and docs (no test files):

- `crates/holler-cli/src/roster_cmd.rs`
- `crates/holler-cli/src/main.rs`
- `crates/holler-cli/src/cli.rs`
- `docs/pane-control.md`
- `docs/adr/ADR-0003.md`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Pipeline scratch: `docs/handoffs/648/handoff-F.md` (new), `docs/handoffs/648/evidence.md` (new),
`docs/handoffs/648/decisions.md` (appended).
