# Handoff-T-red: Phase 4 - #640 part 3 of 3: the opt-in scratch-Herdr test, the contract docs and the ADR-0021 rows

**Date:** 2026-10-09 (10:59 PM MDT)
**Branch:** issue-640-implementation (head `ff31dd6`)
**Brief / wireframe reviewed:** `docs/handoffs/640-brief.md`; wireframe N/A (no UI surface)

This file replaces part 2's `handoff-T-red.md`, as the brief's Handoffs line says. Part 2's is in git at `0ad2d8a`.

## A precondition

Confirmed: A returned **PASS** on the plan (`docs/handoffs/640/handoff-A.md`, 0 block, 6 warn). T applied finding 3:
`scratch_herdr/mod.rs` has one bounded runner (`run_bounded`, drains stdout on a thread, polls `try_wait`, kills and
reaps on expiry, the shape of the host adapter's `exec::run`) and one bounded poll helper (`poll`, checks first, then
sleeps the lesser of the interval and the time left, the shape of `wait_for`). Start, stop and AC 13's two read polls
all use `poll`. Finding 5 (exempt `adapter.rs:338` from AC 19) is F's and S's. The other warns are not T's.

## What T added (test code only; no `src/` change)

| File | Change |
|---|---|
| `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs` (413 lines, new) | The harness, with every pinned item of the brief: `GATE_VAR`, `NAME_PREFIX`, `ROOT_PREFIX`, `SOCKET_PATH_LIMIT`, `Gate`, `gate`, `check_name`, `new_name`, `check_socket`, `prove`, `scratch_env`, `opt_in`, `ScratchHerdr::{start, session, socket}` and its `Drop`. Private: `command` (the one `Command::new`), `run_bounded`, `scratch_base`, `make_root`. Public extra: `poll` (used by the test file's read polls). |
| `crates/holler-adapter-herdr/tests/scratch_herdr_test.rs` (337 lines, new) | AC 1-7 (default run) and the two ignored tests of AC 12-13. |
| `crates/holler-adapter-herdr/tests/adapter_messages_test.rs` (339 lines, new) | AC 16-18, through the `Tap`. |
| `crates/holler-adapter-herdr/tests/wire_herdr/mod.rs` (650 to 654 lines) | `Tapped::Fail(PaneError)` and its one arm in `Tap::exchange`; the `Tap` doc names it. |
| `crates/holler-adapter-herdr/Cargo.toml` | The `tempfile` comment names the scratch root (comment only). |

The crate has `autotests` on (no `[[test]]` entries), so both new test files build without a manifest change, as
the run shows.

## Tests authored

**A. The harness's guards (default run, pure; no process, socket, file or env read).** They test test code, so they
pass at RED by construction (Decision 15).

| Test | AC | Pins |
|---|---|---|
| `the_gate_runs_only_on_exactly_1` | 1 | `Run` only for `Some("1")`; `None`, `""`, `"0"`, `"true"`, `"1 "`, `"yes"` skip |
| `the_name_guard_refuses_the_default_session_by_name` | 2 | `default` and `Default` refused, the text names `default` |
| `the_name_guard_accepts_only_holler640_and_8_hex` | 3 | the one valid form, and the ten refused names of AC 3 |
| `generated_names_pass_the_guard_and_differ` | 4 | 64 names, all valid, all distinct |
| `the_socket_guard_keeps_the_socket_inside_the_root` | 5 | inside the root by components; sibling prefix, outside, relative, `..`, `.`, the root itself and a 100-byte path refused; a 99-byte path accepted |
| `the_server_is_proven_by_its_own_status` | 6 | `prove` accepts the right status (extra field ignored) and refuses `default` (text names it), another name, an outside socket, a missing `session`, a missing `socket`, a non-string `socket`, `not json` |
| `the_scratch_env_carries_nothing_of_the_live_herdr` | 7 | the literal inherited set of AC 7: no `HERDR_*`/`TMUX`/`TMUX_PANE`, no live value, each `HOME`/`XDG_*` once and under the root, `SHELL=/bin/sh`, `PATH` and `LANG` kept |

Tier: unit-level, in an integration test file, since the harness lives in `tests/` (the cheapest tier that reaches it).

**C. Against a real scratch Herdr (ignored; `HOLLER_HERDR_SCRATCH=1`).** Tier: e2e against a real server. The issue's
line requires it; nothing cheaper reaches real Herdr.

| Test | AC | Pins |
|---|---|---|
| `scratch_herdr_passes_the_conformance_suite` | 12 | `run_herdr_conformance` with one `ScratchHerdr` per case as the guard, workspace `holler640-grid` (2 by 1); all 11 cases hold |
| `scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane` | 13 | steps 1-6 of AC 13 in order; `opt_in()` is the first statement; polls are 100 ms / 10 s and their failure quotes only a line count |

**D. The adapter's two follow-ups (default run, in process).** Tier: integration through the wire fake's `Tap` (the
errors are produced inside the adapter's call path, which a unit test of `protocol.rs` cannot reach).

| Test | AC | Pins |
|---|---|---|
| `a_timeout_names_the_port_method` | 16 | the 14 rows of AC 16's table: each wire exchange is failed with the transport's `Timeout { op: "herdr.<wire method>" }`, and the call must return exactly `Timeout { op: <port op> }`. The expected ops come from `HerdrOp::as_str` (so they equal the test kit's), plus `herdr.connect`. Each row also asserts the trap fired, so a row cannot pass by never reaching its exchange. |
| `every_other_error_passes_through_unchanged` | 17 | the same 14 exchanges failed with `Unavailable { what: "injected-unavailable" }`; each call returns it exactly. A regression guard for F's mapping: it holds today and fails if the rename touches any error but `Timeout`. Part 2's AC 17 (split `pane-not-found` is `unavailable`) stays pinned by the existing `adapter_test.rs`. |
| `a_misplaced_new_panes_id_is_cut_to_64` | 18 | `confirm`'s "landed elsewhere" arm (split rewritten `down` to `right`, new id `LONG` in the split's reply and the tree) |
| `a_new_pane_with_no_cell_has_its_id_cut_to_64` | 18 | `confirm`'s "no cell" arm (only the split's reply names `LONG`) |
| `a_vanished_split_targets_id_is_cut_to_64` | 18 | `changed_under` (the tree names the target `LONG`; the fake answers its split `pane_not_found`; asserts the split was sent) |
| `a_tabless_workspaces_label_is_cut_to_64` | 18 | `grid_tab` via `ensure_pane` (label `LONG`, the snapshot's tabs emptied; asserts the rewrite happened) |

Each AC 18 test asserts `what` contains `format!("{:?}...", "x".repeat(64))`, not `TAIL-NOT-QUOTED`, and no `\n`.

## RED confirmation

Run: `CARGO_BUILD_JOBS=4 cargo test -p holler-adapter-herdr --no-fail-fast`. It compiles. Every part 1 and part 2
target stays green: `adapter_conformance_test` 4, `adapter_test` 18, `layout_test` 10, `plan_splits_test` 19,
`protocol_test` 33, `transport_test` 13. `scratch_herdr_test`: 7 passed, 2 ignored. `adapter_messages_test`: 1 passed,
5 failed. Each failure is on the assertion the feature changes:

```
---- a_timeout_names_the_port_method ----
panicked at crates/holler-adapter-herdr/tests/adapter_messages_test.rs:217:5:
Connect with ping #1 timed out: got Err(Timeout { op: "herdr.ping" }), want Err(Timeout { op: "herdr.connect" })
EnsureFirst with session.snapshot #1 timed out: got Err(Timeout { op: "herdr.session.snapshot" }), want Err(Timeout { op: "herdr.ensure_pane" })
EnsureFirst with workspace.create #1 timed out: got Err(Timeout { op: "herdr.workspace.create" }), want Err(Timeout { op: "herdr.ensure_pane" })
EnsureFirst with layout.export #1 timed out: got Err(Timeout { op: "herdr.layout.export" }), want Err(Timeout { op: "herdr.ensure_pane" })
EnsureSecond with layout.export #1 timed out: got Err(Timeout { op: "herdr.layout.export" }), want Err(Timeout { op: "herdr.ensure_pane" })
EnsureSecond with pane.split #1 timed out: got Err(Timeout { op: "herdr.pane.split" }), want Err(Timeout { op: "herdr.ensure_pane" })
EnsureSecond with layout.export #2 timed out: got Err(Timeout { op: "herdr.layout.export" }), want Err(Timeout { op: "herdr.ensure_pane" })
SendText with pane.send_text #1 timed out: got Err(Timeout { op: "herdr.pane.send_text" }), want Err(Timeout { op: "herdr.send_text" })
SendKeys with pane.send_keys #1 timed out: got Err(Timeout { op: "herdr.pane.send_keys" }), want Err(Timeout { op: "herdr.send_keys" })
Read with pane.read #1 timed out: got Err(Timeout { op: "herdr.pane.read" }), want Err(Timeout { op: "herdr.read" })
Close with pane.close #1 timed out: got Err(Timeout { op: "herdr.pane.close" }), want Err(Timeout { op: "herdr.close" })
Snapshot with session.snapshot #1 timed out: got Err(Timeout { op: "herdr.session.snapshot" }), want Err(Timeout { op: "herdr.snapshot" })
Snapshot with layout.export #1 timed out: got Err(Timeout { op: "herdr.layout.export" }), want Err(Timeout { op: "herdr.snapshot" })
Version with ping #1 timed out: got Err(Timeout { op: "herdr.ping" }), want Err(Timeout { op: "herdr.version" })

---- a_misplaced_new_panes_id_is_cut_to_64 ----  (adapter_messages_test.rs:249)
"Herdr put the new pane \"xxxx...xxxx-TAIL-NOT-QUOTED\" at r1c2, not at r2c1; it is left where it landed" does not quote "xxxx...xxxx"...
---- a_new_pane_with_no_cell_has_its_id_cut_to_64 ----  (:249)
"the new pane \"xxxx...xxxx-TAIL-NOT-QUOTED\" has no cell in workspace \"w\", so it is not at r2c1; it is left where Herdr put it" does not quote "xxxx...xxxx"...
---- a_vanished_split_targets_id_is_cut_to_64 ----  (:249)
"Herdr no longer has the pane \"xxxx...xxxx-TAIL-NOT-QUOTED\" that r2c1 is split from: workspace \"w\" changed while the pane was placed" does not quote "xxxx...xxxx"...
---- a_tabless_workspaces_label_is_cut_to_64 ----  (:249)
"Herdr workspace \"xxxx...xxxx-TAIL-NOT-QUOTED\" has no tab, so it has no grid" does not quote "xxxx...xxxx"...
```

(`xxxx...xxxx` abbreviates the 64 `x` here only; the output has all 64.) The AC 16 failure shows the wire `op` at every
row, and each AC 18 failure shows the whole `LONG` and names the intended arm, so each test reaches the site it pins.
`every_other_error_passes_through_unchanged` passes at RED by design (a guard; see the table).

**AC 9-11 (the gate, run by T; they check the gate, not F's work): all PASS.**

- AC 9: `cargo test -p holler-adapter-herdr --test scratch_herdr_test -- --list --ignored` lists exactly
  `scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane` and `scratch_herdr_passes_the_conformance_suite`.
  `grep -rn '#\[ignore' crates/holler-adapter-herdr/` prints exactly `scratch_herdr_test.rs:214` and `:268`, each
  `#[ignore = "opt-in: a scratch Herdr server; set HOLLER_HERDR_SCRATCH=1 (#640)"]`. The default run reports `2 ignored`.
- AC 10: `env -i target/debug/deps/scratch_herdr_test-<hash> --ignored --nocapture`: 2 passed, and two lines
  `skipped (HOLLER_HERDR_SCRATCH is not 1): no scratch Herdr server was started`.
- AC 11: `env -i HOLLER_HERDR_SCRATCH=1 <binary> --ignored`: both FAILED, each with
  `HOLLER_HERDR_SCRATCH=1 asks for a scratch Herdr, but no executable `herdr` is on PATH`.

**AC 12-14 (opt-in, on the pipeline's machine, against the merged adapter): all PASS, no real-Herdr gap.**

Command: `HOLLER_HERDR_SCRATCH=1 cargo test -p holler-adapter-herdr --test scratch_herdr_test -- --ignored --test-threads=1 --nocapture`,
then the built binary three more times with the same arguments.

```
test scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane ... Herdr reports version 0.9.1-preview.2026-09-21-0ff0f27e2226
the pane pass took 65.939103ms
ok
test scratch_herdr_passes_the_conformance_suite ... the conformance suite took 2.094111521s
ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 2.26s
```

The three repeats: pane pass 67.1 ms, 67.7 ms, 67.3 ms; conformance suite 2.34 s, 2.39 s, 2.45 s; all `ok`.

AC 14 record:

- `herdr --version` (run with an empty environment except `PATH` and a throwaway `HOME`/`XDG_*`):
  `herdr 0.9.1-preview.2026-09-21-0ff0f27e2226`, the build spike #636 tested (protocol 22).
- Duration of each test: above (pane pass about 66 ms including server start and proof; conformance suite 2.1-2.5 s
  for 11 servers).
- After every run, `pgrep -af 'h640\.'` printed nothing but the checking shell itself, and `pgrep -a herdr | grep -c holler640`
  printed `0`.
- `find /tmp -maxdepth 1 -name 'h640.*'` printed `0` entries. The scratch base on this machine is `/tmp`: `TMPDIR`
  is `<home>/.cache/tmp`, 31 bytes, over the harness's 29-byte limit (decisions.md).

So the four behaviours the wire fake held INFERRED (part 2 brief, E-9) held against real Herdr wherever the suite
reaches them: `pane-not-found` for `send_text`, `send_keys` and `read` of a closed pane (conformance case 9, and AC 13
step 6's `close` again and `read`), and the close and snapshot paths of the cases. AC 15 adds nothing: there is no gap
to reproduce. T did not mark any wire-fake behaviour VERIFIED in code, since no fake change was needed; the evidence is
this record.

**Doc ACs 21-27 show the old text (RED, F's edits):**

- AC 21: `grep -nE 'C-c|Enter' crates/holler-pane/src/ports.rs` prints `101:/// `Enter` or `C-c`).`
- AC 22: the `ensure_pane` doc names neither `grid-out-of-range` nor `grid-unreachable`.
- AC 23: `outside the Herdr workspace's extent` and `<port>.<method>` each occur 0 times in `error.rs`.
- AC 24: matches `pane.rs:112`, `reconcile.rs:106`, `ADR-0021.md:40`.
- AC 25: `grid-unreachable` occurs 0 times in the ADR; the old `:427` and `:434` sentences are present; `<port>.<method>` 0.
- AC 26: `HOLLER_HERDR_SCRATCH` occurs 0 times in `docs/testing.md`. AC 27: no entry yet.
- AC 19: `grep -n 'fn excerpt' crates/holler-adapter-herdr/src/*.rs` prints `protocol.rs:580:fn excerpt(...)`, not `pub(crate)`.

**Gates on T's files at RED:** `cargo clippy -p holler-adapter-herdr --all-targets -- -D warnings` clean;
`cargo fmt --check -p holler-adapter-herdr` clean; `bash scripts/lint.sh` exit 0 (warns only, `wire_herdr/mod.rs` 654
lines among them, under 900). AC 8: `grep -rn 'Command::new' crates/holler-adapter-herdr/` prints only
`tests/scratch_herdr/mod.rs:247`; `std::env`/`env::var` under `tests/` match only `tests/scratch_herdr/mod.rs`. AC 29's
pattern over the crate matches only `tests/scratch_herdr/mod.rs` and `tests/scratch_herdr_test.rs`.

## Ready for F

Confirmed: RED is valid. F may implement against these tests: the `op` rule (Decision 9) and `excerpt` at the four
Herdr-sent quotes (Decision 10) turn AC 16 and AC 18 green while AC 17 stays green, and the doc edits D1-D6, A1-A9,
Decision 13 and 14 turn the doc greps. No real-Herdr gap was found, so F has no AC 15 fidelity fix. F writes no tests.
