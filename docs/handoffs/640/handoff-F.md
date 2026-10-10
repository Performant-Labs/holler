# Handoff-F: #640 part 3 of 3 - the opt-in scratch-Herdr test, the contract docs and the ADR-0021 rows

**Date:** 2026-10-09 (11:16 PM MDT)
**Branch:** issue-640-implementation (head `5cd4687`; the Workflow script commits this phase)
**Issue:** #640 (epic #633), part 3 of 3; the PR closes #640

This file replaces part 2's `handoff-F.md`, as the brief's Handoffs line says. Part 2's is in git at `0ad2d8a`.

| Field | Value |
|---|---|
| GitHub issue | #640 (read 2026-10-09; Scope and Acceptance line 3 carry the operator's 2026-10-09 amendments, "For the operator" item 1) |
| Working branch | `issue-640-implementation`, worktree `.claude/worktrees/0640-herdr-adapter` |
| Build plan phase | part 3 of 3 (the brief's Scope table, row 3) |
| Input documents read | `docs/handoffs/640-brief.md` (all 1401 lines), `handoff-A.md`, `handoff-T-red.md`, `decisions.md`, T's four test files, the files edited below, and the sibling adapters' `op` handling (`holler-adapter-opencode/src/lib.rs`, `origin/main:crates/holler-adapter-host/src/lib.rs`) |
| Acceptance criteria count | 32 in the brief (F's are 16, 18-27; the rest are T's, or gates) |
| Handoff document path | `docs/handoffs/640/handoff-F.md` |

No confirmation was awaited: this ran inside the Workflow script, with no human at the gate. The run used
`CARGO_BUILD_JOBS=4` for every build, and F never ran `herdr` or an ignored test (Decision 7: only T runs the scratch
server).

## What was done

- `crates/holler-adapter-herdr/src/adapter.rs`:
  - **The `op` rule (Decision 9).** Eight constants `OP_CONNECT` (`herdr.connect`) and `OP_ENSURE_PANE` to `OP_VERSION`
    (the test kit's `HerdrOp` strings). Every `HerdrPort` method and `connect_with` runs its body through one new
    private helper, `run_as(op, |deadline| ...)`. The helper takes the call's one deadline, as each method did on entry,
    and replaces the `op` of any `Timeout` from the body with the call's own. Every other error passes unchanged.
  - `ensure_pane`'s body moved, unchanged, into a private `ensure(spec, deadline)` next to `place` and `confirm`. Left
    in place inside the new closure, rustfmt re-wrapped its `match` badly. The trait method is now one line.
  - **`excerpt` at the four Herdr-sent quotes (Decision 10):** `made` in both arms of `confirm`, `target` in
    `changed_under`, and the label in `grid_tab`. Each `{:?}` placeholder there became `{}` with `excerpt(..)`.
  - The module doc gains two bullets (the `op` rule, and quoting cut to 64 characters), and `connect_with`'s doc names
    `herdr.connect`.
- `crates/holler-adapter-herdr/src/protocol.rs`: `fn excerpt` became `pub(crate) fn excerpt`. Nothing else changed
  (AC 19).
- `crates/holler-pane/src/ports.rs`: D1 (`Key`: `enter` / `ctrl+c`, sent as written) and D2 (`ensure_pane`: an occupied
  cell, `grid-out-of-range`, `grid-unreachable`). These are doc comments only.
- `crates/holler-pane/src/error.rs`: D3 (`GridOutOfRange`: a cell outside the Herdr workspace's extent) and D4
  (`Timeout`: `op` names the port method, as `<port>.<method>`). These are doc comments only.
- `crates/holler-pane/src/pane.rs` (D5) and `crates/holler-pane/src/reconcile.rs` (D6): a verb records
  `herdr_api_version`, and the adapter writes no record. These are doc comments only.
- `docs/adr/ADR-0021.md`: A1-A9 of Decision 12, each in place (+18 / -11 lines, only those rows and sentences). The
  `**Date:**` line is unchanged.
- `docs/testing.md`: the new section "Opt-in: a scratch Herdr server (#640)" (Decision 13), placed after "Beyond
  loopback: `interop.yml`" and before "Windows is deferred". It covers what the two tests check, the section C command,
  the gate (only `1`; a missing `herdr` fails), the isolation, CI never running the tests, protocol 22, and the sibling
  tmux convention (A finding 2).
- `CHANGELOG.md`: one `[Unreleased]` / `### Enhancements` entry right after the part-2 entry, linking #633, #649 and
  #640 (Decision 14's suggested text).
- `docs/handoffs/640/evidence.md`: replaced part 2's file with twelve part-3 facts (see the Evidence appendix).

## Design decisions

1. **Where the `op` is renamed: at the call boundary, around the whole body.** The alternative mirrors the sibling
   adapters' `Call { op, deadline }` struct, threaded through every private helper, with the renaming done at the one
   exchange site (`call`). I rejected it. The host and OpenCode adapters *create* their own timeouts, so a `Call` that
   carries the op is natural there. In this adapter the transport creates them, in its own wire vocabulary, under a
   `Transport` trait (AC 20 keeps that layer as it is). So the adapter renames at the port boundary, as the brief's
   example ("the trait impl calls a private method and maps its error once") and A's "maps a Timeout once, at the port
   boundary" describe. Wrapping the whole body guarantees "no `Timeout` escapes unrenamed" by construction, even for a
   future source that is not the transport. It also changes none of the helpers' signatures (`grid`, `workspace_grid`,
   `place`, `confirm`, `supported_server` still take a `deadline: Instant`). `run_as` also takes the deadline, so "one
   deadline per call, taken on entry" now lives in one place instead of eight `let deadline = self.deadline()?;` lines.
2. **`connect_with` names `herdr.connect`** through the same helper. Production's `connect` delegates to it
   (evidence.md), so the socket path and the test seam answer alike (AC 16's first row).
3. **`version()` is wrapped whole**, including its empty-or-control-character check (that check is `unavailable` and
   passes through). So all seven trait methods have the same shape, `self.run_as(OP_X, |deadline| { <old body> })`.
4. **The `op` strings are spelled in `src/`**, not imported. `holler-pane-testkit` is a dev-dependency only (ADR-0021
   §5). `adapter_messages_test.rs` pins them equal to `HerdrOp::as_str`, and the constants' comment says so, as the host
   adapter's comment does.
5. **D4 and A4 are kept verbatim, without a constructor exception (A finding 4).** D4 and A4 state the rule for a *port
   method* that runs out, and `connect` is not a port method. So `herdr.connect` falls outside the rule's subject rather
   than contradicting it. The brief bars F from changing D-text meaning (Decision 11) and from touching other ADR lines
   (AC 25). Also, naming an adapter's constructor in the contract crate's `PaneError` doc would tie `holler-pane` to an
   adapter. So the exception is stated where it is defined: the adapter's module doc, the `connect_with` doc and the
   constants' comment. A4's example `harness.health` is kept as well. The test kit's `FakeHarness` does answer a wedged
   `health` with `timeout` and `op` `harness.health` (`faults.enter(HarnessOp::Health)`), so the example is a true
   instance of the form, even though the real OpenCode adapter's `health` never errors. If the operator wants A's
   wording, it is a one-clause change to D4 and A4.
6. **ADR line wrapping.** A6, A7, A8 and A9 rewrap their own paragraph or bullet at the ADR's ~124 columns. Table rows
   (A1-A5) stay single lines, as the ADR's tables are. A8's sentence is the brief's text verbatim, a list of three
   clauses.
7. **`docs/testing.md` follows that file's style**: one paragraph per line, a `bash` fence for the command. It has no
   `holler ...` command, so `docs_cli_test` has nothing new to parse (it passes; see below).

## Reuse / extend-vs-new

- `protocol::excerpt` is **used**, made `pub(crate)` (the Reuse map's first row). There is no second copy:
  `grep -n 'fn excerpt' crates/holler-adapter-herdr/src/*.rs` prints one line.
- The `op` names equal `HerdrOp`'s (the Reuse map's last row), plus the documented `herdr.connect`.
- `run_as` is new, and there is no existing object to extend. The crate had no error-mapping helper of this kind.
  `changed_under` maps one variant for one exchange, and the siblings' `Call` structs are in other adapter crates,
  which ADR-0021 §5 bars this one from depending on. `run_as` replaces the eight `let deadline = self.deadline()?;`
  lines instead of adding a parallel path beside them.
- `ensure` is `ensure_pane`'s own body, moved as it was (no new logic).
- F added no test seam. `Tapped::Fail` and the scratch harness are T's.

## Architecture notes for A

- Layers: `adapter.rs` only (the port boundary). The transport's vocabulary and code are unchanged (AC 20:
  `git diff origin/main -- crates/holler-adapter-herdr/src/transport.rs` is empty).
- Public interface: unchanged. `HerdrAdapter`, `HerdrConfig` and every signature stay as they were. `run_as`, `ensure`
  and the `OP_*` constants are private. `excerpt` is `pub(crate)`, which is not public API.
- Behaviour visible to a caller: a `timeout` from any `HerdrPort` method now has `op` `herdr.<port method>`, and from
  `connect`/`connect_with` `herdr.connect`, where it used to be `herdr.<wire method>`. A message quoting a Herdr-sent id
  or label longer than 64 characters now cuts it and appends `...`. Nothing branches on an `op`'s text (evidence.md).
- Dependencies: none added or changed (`cargo tree -p holler-adapter-herdr -e normal --depth 1`: `holler-pane`,
  `serde_json`; `cargo machete` clean).
- Contract crate (`holler-pane`): doc comments only, no signature, derive, attribute or code line. ADR-0021 and
  `docs/testing.md` are docs.
- `adapter.rs` is 493 lines, under the 600-line warning and the 900-line fail.

## Deviations from spec / wireframe

None in meaning. Three choices the brief left open, or that A raised, are recorded here for S:

- **A finding 2 applied in Decision 13's section.** One sentence names the host adapter's tmux tests
  (`crates/holler-adapter-host/tests/real_tmux_test.rs`, opt-in through `--ignored` alone, skipping without `tmux`) and
  says why Herdr's also need `HOLLER_HERDR_SCRATCH=1`. That file is on `main` (`e327569`) but not on this branch's base
  `dc300ab`. It is cited in code font, not linked, and it exists in the tree the PR merges into.
- **A finding 5 applied.** `adapter.rs:383` (`workspace: workspace.label.clone()`, in `snapshot`; it was `:338`) is a
  returned value and stays whole. AC 19's second grep prints it next to the four `excerpt(..)` sites.
- **D4 and A4 verbatim** (Design decision 5): A finding 4's exception for the constructor is documented in the adapter,
  not in the contract crate or the ADR.

## Tier 1 self-check (incl. tests now GREEN)

Every command ran in the worktree with `CARGO_BUILD_JOBS=4` and without `HOLLER_HERDR_SCRATCH`.

```
$ cargo test -p holler-adapter-herdr --no-fail-fast
adapter_conformance_test   4 passed
adapter_messages_test      6 passed   (at RED: 1 passed, 5 failed)
  test a_tabless_workspaces_label_is_cut_to_64 ... ok
  test a_vanished_split_targets_id_is_cut_to_64 ... ok
  test a_misplaced_new_panes_id_is_cut_to_64 ... ok
  test a_new_pane_with_no_cell_has_its_id_cut_to_64 ... ok
  test a_timeout_names_the_port_method ... ok
  test every_other_error_passes_through_unchanged ... ok
adapter_test              18 passed
layout_test               10 passed
plan_splits_test          19 passed
protocol_test             33 passed
scratch_herdr_test         7 passed, 2 ignored
transport_test            13 passed   (still asserts `herdr.ping` at the transport level)

$ cargo test -p holler-pane                                  14 targets: 86 passed, 0 failed, 0 ignored
$ cargo test -p holler-cli --test docs_cli_test              3 passed (scans the new docs/testing.md section)
$ cargo clippy -p holler-adapter-herdr --all-targets -- -D warnings      clean
$ cargo clippy --workspace --all-targets -- -D warnings                  clean (Finished, no warning)
$ cargo fmt --check -p holler-adapter-herdr -p holler-pane               clean
$ RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-herdr -p holler-pane --no-deps   clean
$ bash scripts/lint.sh            exit 0; warns only (error.rs 713, adapter_test.rs 645, wire_herdr/mod.rs 654, protocol_test.rs 679)
$ bash scripts/changelog-check.sh changelog-check: ok
$ cargo machete                   didn't find any unused dependencies
```

The brief's greps for F's criteria, run from the repo root:

- AC 19: `grep -n 'fn excerpt' crates/holler-adapter-herdr/src/*.rs` prints only
  `protocol.rs:580:pub(crate) fn excerpt(text: &str) -> String {`. The second grep prints `adapter.rs:312` and `:318`
  (`excerpt(made.as_str())`), `:422` (`excerpt(&workspace.label)`), `:436` (`excerpt(target.as_str())`) and the exempt
  `:383`.
- AC 20: the transport's diff against `origin/main` and against `HEAD` is empty.
- AC 21: `grep -nE 'C-c|Enter' crates/holler-pane/src/ports.rs` prints nothing. AC 22: the `ensure_pane` doc names both
  codes (`ports.rs:127-131`).
- AC 23: `error.rs:416` contains `outside the Herdr workspace's extent`, and `:457` contains `<port>.<method>`.
- AC 24: `grep -rnE '#640 records it|recorded by the Herdr adapter' crates/ docs/adr/` prints nothing.
- AC 25: `grep -c 'grid-unreachable'` prints `4`. The two old sentences print nothing. `<port>.<method>` prints only
  `ADR-0021.md:396`, the §9 `timeout` row. `git diff --stat` reports `29 ++++++++++++++++++-----------`, A1-A9's lines
  only.
- AC 26: `docs/testing.md` names `HOLLER_HERDR_SCRATCH`, the section C command and "CI never runs them".
- AC 27: the entry sits right after the part-2 entry, and `changelog-check: ok`.
- AC 29 (production code): the narrowed part-2 AC 38 pattern over `src/` and `Cargo.toml` prints nothing. Over `tests/`
  it matches only `scratch_herdr/mod.rs` and `scratch_herdr_test.rs`. AC 8: one `Command::new`. AC 9: the two
  `#[ignore` lines are unchanged.
- AC 30: no manifest line changed in this phase.

**Merge check against `origin/main` (now `d9eabbb`, A finding 1).** F cannot merge `origin/main`, since that would be
a commit. A three-way `git merge-file` of each file F edited (base `dc300ab`, theirs `origin/main`, done on scratch
copies) is clean for all nine. That includes `ADR-0021.md`, where `main` changed the `profile create` and
`profile delete` rows two lines above A3's `profile apply` row, and `CHANGELOG.md`.

Not run by F (T's at T-green, by the brief's Test plan): AC 9-14 (the gate runs and the opt-in scratch-Herdr runs),
`cargo test --workspace`, and the two mutants.

## Evidence appendix

`docs/handoffs/640/evidence.md`. It holds twelve entries: the transport is the only `Timeout` producer; `connect`
delegates to `connect_with`; nothing reads an `op`'s text; the `HerdrOp` strings; `excerpt`'s exact behaviour, so a
short id is quoted as before; where each of the four quoted values comes from in Herdr's replies; why `snapshot`'s
returned label stays whole; D2 and D3 against `plan_splits`; D1 against the `send_keys` encoding; D5 and A1 against
the adapter's two fields; CI's one `--ignored` run; and the tmux sibling's convention (on `origin/main`).

## Tests that look wrong (for T)

None. One note, not a defect: `adapter_messages_test.rs` builds the expected `op` of `connect` from its own constant
`CONNECT_OP = "herdr.connect"`, while the other seven come from `HerdrOp`. That matches the brief (AC 16: "the one
string ... with no test-kit counterpart").

## Known issues

None against this part's acceptance criteria. These items are left for the operator or later stories, as A and the
brief record them:

- **The `snapshot` port doc** (`ports.rs:146`: "Every pane Herdr has, with its position") still says more than the
  adapter returns. A pane with no cell is left out, which A9 now records as waiting for a contract amendment. A finding
  6(a) proposed a D7. F did not add one: it is outside D1-D6 and would settle, in the contract crate, the question that
  A9 defers to an amendment.
- **No "Deferred to named stories" line for A7's PROPOSED owner** of `host.herdr_api_version` (A finding 6(b)). AC 25
  allows no ADR line beyond A1-A9. The item is marked **PROPOSED** in place, which the ADR's Status line covers, and it
  is "For the operator", item 2.
- **The test kit's `ASSUMPTION (#640)` comments** (`holler-pane-testkit/src/herdr.rs:272-274`,
  `conformance/herdr.rs:189-196`, `:337-339`) become stale once this merges. That is #638's crate, the brief puts it
  out of scope, and the follow-up issue is unfiled ("For the operator", item 4; A finding 6(c)). Filing it is
  outward-facing and not F's.

## Files changed

Production and docs (no test files):

- `crates/holler-adapter-herdr/src/adapter.rs`
- `crates/holler-adapter-herdr/src/protocol.rs`
- `crates/holler-pane/src/ports.rs`
- `crates/holler-pane/src/error.rs`
- `crates/holler-pane/src/pane.rs`
- `crates/holler-pane/src/reconcile.rs`
- `docs/adr/ADR-0021.md`
- `docs/testing.md`
- `CHANGELOG.md`

Handoffs: `docs/handoffs/640/handoff-F.md` (this file), `docs/handoffs/640/evidence.md` (replaced), and
`docs/handoffs/640/decisions.md` (F's entry appended).
