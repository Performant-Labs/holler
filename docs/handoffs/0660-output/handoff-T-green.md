# Handoff-T-green: Phase 6 — GREEN + Tier 2 on the #660 conformance suite

**Date:** 2026-10-09, 11:28 PM MDT
**Branch / worktree:** `issue-0660-output` / `.claude/worktrees/0660-output` (base `519947a`, opener `2f122b9` on top — empty, ignored per instructions)
**Issue:** #660 · **Rigor:** in-session · **UI surface:** no (U N/A)
**Handoff-F reviewed:** `docs/handoffs/0660-output/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/0660-output/handoff-T-red.md`

VERDICT: GREEN

## GREEN confirmation

The plugin already recorded GREEN at the t-green crossing (workspace suite, exit 0). Re-verified
narrow, one cargo at a time, in the run worktree:

```
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 134 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.04s

$ cargo test -p holler-cli --test pane_cli_process
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

Counts match handoff-T-red and handoff-F exactly. After the Copy-pin repair below, the edited
target was re-run:

```
$ cargo test -p holler-cli --test pane_verbs        (post-repair)
test result: ok. 134 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.04s
```

F's no-op is **consistent with everything I observe**: `git diff 519947a -- crates/holler-cli/src`
is empty (verified first-hand), the full worktree diff is exactly the two test files
(+321/−9) plus this handoffs directory, and the suite is green against the unchanged `output.rs`.
A deliberate no-op was the only honest F contract under decision 4 (GREEN-on-contact) and the
operator's test-only continuation (~11:05 PM journal entry); nothing in this phase contradicts it.

## The Copy-pin repair (F's note 1 — accepted, fixed)

F was right: the `Format` `Copy` claim in `the_fixed_signatures_and_types_compile_unchanged`
(`crates/holler-cli/tests/pane_verbs/output_api.rs`) was not compile-pinned — `assert_eq!` only
*borrows* its operands, so `let copy = Format::Text; assert_eq!(copy, Format::Text);` would
compile even with `Copy` removed.

Before:

```rust
// `Format`: the two variants, and `Copy` (a resolved choice is passed by value).
let copy = Format::Text;
assert_eq!(copy, Format::Text);
assert_eq!(Format::Json, Format::Json);
```

After (a real move-and-use pin):

```rust
// `Format`: the two variants, and `Copy` — pinned by a real move-and-use.
// `resolved` is moved into `again` and then read again, which compiles only
// while `Format` is `Copy`; a bare `assert_eq!(copy, Format::Text)` merely
// borrows and so could never catch `Copy` being removed.
let resolved = Format::Text;
let again = resolved;
assert_eq!(resolved, again);
assert_eq!(Format::Json, Format::Json);
```

`let again = resolved;` moves `resolved` if `Format: !Copy`; the subsequent read in `assert_eq!`
is then E0382 (borrow of moved value) — a compile failure, which is the pin. Proven on scratch
files in temp (no tracked file touched, so no mutation testing): the same pattern with a
non-`Copy` struct fails `rustc` with `error[E0382]: borrow of moved value` (exit 1); with
`#[derive(Copy, Clone)]` it compiles (exit 0). The test name is unchanged; the suite stayed green
through the repair (re-run above) — no new RED was introduced at t-green.

## F's notes 2 and 3 — already hedged, no change needed (confirmed)

- **Note 2 (the STUBS-riding diagnostic test expires):** already documented in the decisions
  journal (~10:50 PM, "Hedged — the diagnostic test rides a STUB verb picked from `STUBS` … will
  need rework when the last stub goes live") **and in the test itself** — `process/stub.rs:229`
  `.expect("a pane stub exists while this test lives (#660)")`, under the comment "one pane stub
  stands in". Confirmed present; nothing to add.
- **Note 3 (`--debug noisy` as the reading of "diagnostic"):** already recorded as an assumption
  (~10:50 PM, "Assumed — the banner is the diagnostic the bullet means"), restated in the test's
  doc comment (`stub.rs:217-221`). It is an interpretation the operator may still confirm; it
  stands as hedged.

## Tier 1 results

| Check | Command | Expected | Actual | Status |
|---|---|---|---|---|
| Edited test target green | `cargo test -p holler-cli --test pane_verbs` | 134/0 | 134 passed, 0 failed | PASS |
| Second touched target green | `cargo test -p holler-cli --test pane_cli_process` | 35/0 | 35 passed, 0 failed | PASS |
| F's reported commands re-run | same two targets (F reported 134/0 and 35/0) | match | match, no discrepancy | PASS |
| No-op verification | `git diff 519947a -- crates/holler-cli/src` | empty | empty (0 lines) | PASS |
| Full workspace suite | not run here — plugin-owned | plugin GREEN at the t-green crossing | not re-litigated | N/A |

Tier 1's "build/compile, lint, server starts, API smoke" collapse into cargo build+test+clippy for
this Rust crate; there is no server or API surface in this story.

## Tier 2 results

| Check | Method | Result | Status |
|---|---|---|---|
| Coverage per acceptance criterion | the 8 new + 1 extended tests of T-red, all passing | one or more tests per bullet (table below) | PASS |
| Test quality / proportionality | each test names a behavior, sits at unit tier except the process-tier banner test (only the real binary produces a banner), no duplication; the suite is 8+1 tests against a conformance-layer deliverable | proportionate; one weak pin found (Copy) and **repaired this phase** | PASS |
| Type safety | no `unsafe`, no casts added: `git diff 519947a -- crates/holler-cli/tests \| grep '^+.*unsafe'` | 0 added `unsafe` lines | PASS |
| Error handling / data integrity | refusal, usage-error, stream-failure and diagnostic paths all pinned by tests (goldens, NDJSON failure leg, forced diagnostic, ALL_CODES) | all green | PASS |
| API contract | `check_envelope`/`check_ndjson` (#638) plus exact raw goldens for key order/compactness | green | PASS |
| Migration safety | N/A — no schema change, no migrations (ADR 0003) | — | N/A |
| Clippy on touched targets | `cargo clippy -p holler-cli --test pane_verbs --test pane_cli_process` | `Finished`, **0 warning lines** | PASS |
| fmt on touched files | `rustfmt --check --edition 2021` on `output_api.rs` and `process/stub.rs` | both exit 0 (clean) | PASS |
| Pre-existing fmt drift | `cargo fmt --check` crate-wide | ~2,916 hunks on the committed tree under this machine's rustfmt 1.9.0 — predates the run, not from these files, out of scope (T-red advisory + ~10:45 PM journal entry) | NOTED, out of scope |

Playwright: N/A — this repo's e2e surface is not touched by #660 (no UI surface).

## Acceptance criteria status

| Criterion (brief) | Backing test(s) | Status |
|---|---|---|
| Goldens: success/refusal/usage, both formats, through `check_envelope` | `golden_success_in_both_formats_through_the_checker`, `golden_refusal_in_both_formats_through_the_checker`, `golden_usage_error_in_both_formats_through_the_checker` | PASS |
| NDJSON: each line parses alone, through `check_ndjson` | `emit_stream_in_json_mode_is_valid_ndjson_through_the_checker`, `emit_stream_ending_in_a_refusal_is_valid_ndjson_at_exit_3` | PASS |
| Forced diagnostic in JSON mode; stdout still one envelope | `process/stub.rs::a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope` | PASS |
| Exit-code parity table + checker assertions | `every_closed_code_exits_by_its_class_with_the_same_code_in_both_formats` (extended) | PASS |
| `GridPos` golden, `{"row":R,"col":C,"pos":"rRcC"}` row first | `an_envelope_carrying_a_grid_pos_serializes_row_col_pos_row_first` | PASS |
| #637-fixed signatures compile-pinned | `the_fixed_signatures_and_types_compile_unchanged` (+ type aliases) — **Copy now genuinely move-and-use pinned** | PASS |
| fmt clean, clippy clean, no new `unsafe`, CI green | touched files fmt-clean + clippy 0 warnings + 0 added `unsafe` (above); workspace suite plugin-GREEN at the crossing | PASS (touched scope) |

## Blocking issues

None.

## Residual risks / advisory notes

- **Workspace-suite flake under load is the known residual** — documented and diagnosed in
  decisions.md (~11:05 PM `remote_admin_test` RED, ~11:22 PM `body_run_test` RED; both targets
  pass in isolation; machine load 31+ with 6 concurrent cargo processes from sibling sessions;
  zero production change since `519947a`). Referenced here, not re-litigated. The plugin's
  t-green crossing measured GREEN; if a sibling-load flake recurs at a later plugin-owned
  workspace run (a-dup / merge gate), it is the pre-existing stability issue the journal names,
  not a regression of this suite. My narrow runs were stable across every invocation this phase.
- **The STUBS-riding diagnostic test and the `--debug noisy` interpretation** remain hedged as
  documented (above) — they age with the stub table and the operator's confirmation respectively.
- **T-red's two production advisories stand unimplemented by design** (blank-message `one_line()`
  edge; empty-stream `emit_stream` under `check_ndjson`) — outside #660's acceptance bullets; the
  operator's option (b) (extend the brief) was not taken.
- File size: `output_api.rs` is now 753 lines — still under the 900 fail gate, over the 600 warn;
  the #660 conformance tail remains the natural submodule split if the file grows again.

## Boundary compliance

Edited exactly one file this phase: `crates/holler-cli/tests/pane_verbs/output_api.rs` (the
Copy-pin block, 7 lines replacing 4). No `src/**`, no `Cargo.toml`, no `holler-pane/**`, no
`holler-pane-testkit/**`, no verb files. No mutation testing (the Copy-pin proof used throwaway
scratch files in the temp dir, since deleted; no tracked file was mutated). One cargo invocation
at a time throughout. Nothing committed — all edits left uncommitted in the worktree
(`git status`: `M output_api.rs`, `M process/stub.rs`, `?? docs/handoffs/0660-output/`).
