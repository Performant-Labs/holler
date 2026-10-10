# Handoff-A-dup: Phase 7 - #640 part 3 of 3: the opt-in scratch-Herdr test, the contract docs and the ADR-0021 rows  (anti-duplication gate)

**Date:** 2026-10-09 (11:39 PM MDT)
**Branch:** issue-640-implementation
**Diff base:** `dc300ab` (the merge base; `origin/main` is now `d9eabbb`)   **Diff head:** `db95aa9` (F's code is `8993137`; only `decisions.md` and `handoff-T-green.md` changed after it)
**Reuse map:** `docs/handoffs/640-brief.md`, section "Reuse map (extend, do not duplicate)", plus the written justification in `docs/handoffs/640/handoff-A.md`, finding 3
**Verdict:** PASS

This file replaces part 2's `handoff-A-dup.md`, as the brief's Handoffs line says. Part 2's is in git at `0ad2d8a`.

## Summary

PASS, with no block findings and two warns. F extended every object the Reuse map named and built no parallel path.
`protocol::excerpt` became `pub(crate)` and now quotes all four Herdr-sent values, so the crate still has one copy. The
`op` rename happens once at the port boundary, in a new private `run_as`. That helper also replaces the eight places
that each took a call's deadline, so it consolidates code rather than adding a path beside it. The transport is
unchanged. T added `Tapped::Fail` to the one interceptor and ran `run_herdr_conformance` as written. T's scratch harness
has one bounded runner and one poll helper, as Phase 3 asked, and nothing reachable from this crate could replace either.
Both warns concern that runner. First, #663 merged a bounded runner into `holler-pane` after this branch's base, and
#696 will expose it, but nothing records that the harness's copy should go then. Second, the harness bounds the child's
exit but not the read of its output, unlike the runner it was modelled on.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs:262-285` (`run_bounded`), `:224-240` (`poll`) | **The harness's runner is not on the consolidation list.** After this branch's base, `origin/main` merged #663 (`d9eabbb`). That added a bounded runner to `holler_pane::probe` (`crates/holler-pane/src/probe.rs`), and `holler-pane` is a normal dependency of this crate. The module doc says its mechanics "are kept apart from the verdict, so #696 can lift them into the one bounded runner the adapters share". It is not usable here today. Its only public entry is `run_probe(argv, expect, timeout) -> ProbeResult`, which returns no stdout and uses the caller's environment and working directory. `prove` needs the stdout of `status server --json`, and `command` needs `env_clear()` and `root/work`. So the harness's own runner is still justified (`handoff-A.md`, finding 3). But #696 (open) scopes only "the host and OpenCode adapters ... removing the private copies", and does not name this harness. Phase 3 asked that the Reuse map say "#696's runner replaces the harness's when it lands". The brief was not reopened, so that sentence exists only in `handoff-A.md`, not in the brief, the harness's module doc or #696. Once #696 makes the runner public in `holler-pane`, `run_bounded` will be a near-copy of a reachable object with no record that it should go. | Add the harness to #696's scope. Editing an issue is outward-facing, so this is the operator's call. Suggested wording: "and `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs::run_bounded` (test code), which needs stdout, a cleared environment and a working directory". Optionally, add a one-line pointer to #696 in the harness's module doc. No change is needed in this PR. |
| 2 | warn | `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs:276-284` | **The runner bounds the exit, but not the read of the output.** Phase 3 (finding 3) asked for the runner to be shaped like the host adapter's `exec::run`. It drains stdout on a thread, polls `try_wait`, and kills and reaps on expiry, as asked. But after the child exits it calls `reader.join()` (`:283`) with no bound. `exec::run` (`origin/main:crates/holler-adapter-host/src/exec.rs`) collects its output against the same deadline (`collect(&rx, deadline, &program)`). Its module doc states the rule: "A child that exits while something it started still holds its pipes is `Expired` too once the deadline passes, so no call outlives its bound." `probe::run_bounded` does the same with `recv_timeout`. Here, if a `herdr` call (`status server --json` or `server stop`) leaves a descendant holding stdout, `run_bounded` blocks past `COMMAND_LIMIT`. `ScratchHerdr::start` would then run past `START_LIMIT`, and `Drop` past the brief's bound ("`Drop` cannot hang longer than 15 s", Risks). T's eight opt-in runs showed no such hang, and the tests never run in CI, so the cost is low. | This is T's file. Have the reader thread send its text over an `mpsc` channel, and call `recv_timeout` with the time left, treating expiry like the kill path. Alternatively, take #696's runner when it lands (finding 1). Neither blocks this PR. |

No duplication; extension is clean. F introduced no drift during the cycle.

**Carried from Phase 3, unchanged and not re-flagged:**

- **Warn 4: `herdr.connect` and the wording of D4 and A4.** D4 and A4 state the `op` rule for "the port method" and
  have no constructor clause. The exception is documented in the adapter: the module doc (`adapter.rs:10-14`), the
  `connect_with` doc (`:112-116`) and the constants' comment (`:53-54`). F's Design decision 5 gives the reason. This
  is S's to read against the brief.
- **Warn 6: three stale docs.**
  - (a) The `snapshot` port doc (`crates/holler-pane/src/ports.rs:146`: "Every pane Herdr has, with its position") does
    not match A9.
  - (b) The "Deferred to named stories" list has no line for A7's PROPOSED owner.
  - (c) Three `ASSUMPTION (#640)` comments in the test kit describe the port docs as they were before D1-D3. They ask
    #640 for exactly the edits this diff makes: `crates/holler-pane-testkit/src/herdr.rs:272-274`
    and `src/conformance/herdr.rs:189-196` and `:337-339`. They become false when this merges. The brief puts them out of
    scope, and the #638 follow-up is unfiled ("For the operator", item 4).

**Checked and not a finding** (the Reuse map, row by row, then F's new objects):

- **Quoting Herdr-sent text: `protocol::excerpt`.** The only change to `protocol.rs` is the word `pub(crate)` (`:580`).
  `grep -n 'fn excerpt' crates/holler-adapter-herdr/src/*.rs` prints that one line. The four Herdr-sent quotes go
  through it (`adapter.rs:312`, `:318`, `:422`, `:436`). The caller's values keep `{:?}`. The label `snapshot` returns
  (`:383`) stays whole, as Phase 3's warn 5 asked. For an id of 64 characters or fewer, `excerpt` returns exactly the
  old `{:?}` text, so existing messages are unchanged (`adapter_test.rs:222` still passes). The diff adds no other
  quoting or cutting helper. The workspace's other copies (`holler_pane::error::excerpt` and
  `holler-adapter-opencode/src/lib.rs:455`) existed before this diff and are unchanged.
- **One interceptor: `Tap` and `Tapped`.** The diff adds one variant, `Fail(PaneError)`
  (`tests/wire_herdr/mod.rs:219-221`), and its one arm (`:252`). The `Trap` in `adapter_messages_test.rs` is a hook
  passed to `Tap::new` (`:78-92`), not a transport. The crate's `impl Transport for` lines are still the four that
  existed before: `WireHerdr` (`wire_herdr/mod.rs:206`), `Tap` (`:247`), `Arc<T>` (`src/transport.rs:62`) and
  `UnixSocketTransport` (`:88`).
- **`WireHerdr`** is unchanged apart from `Tapped::Fail`. No real-Herdr gap was found at RED or at GREEN, so AC 15
  added nothing.
- **The conformance suite.** `run_herdr_conformance` runs as written (`scratch_herdr_test.rs:221-229`), in the shape
  of `adapter_conformance_test.rs:52-63`, with the `ScratchHerdr` as each case's guard. No case is copied. AC 13's
  pass (`scratch_herdr_test.rs:267-337`) is the issue's own acceptance line. Its run and send steps (output read back)
  are in no case. Its steps 5 and 6 overlap cases 4, 8 and 9 only because the issue asks for that scenario.
- **The spike's isolation (`scripts/spikes/herdr-lib.sh`)** is ported, not called, and the script is untouched. Its
  config (`CONFIG_TOML`, `scratch_herdr/mod.rs:58-60`) and its environment rule (`scratch_env`, `:170-195`) are
  restated in Rust because the Reuse map says the tests do not call the script.
- **The scratch root** uses `tempfile::Builder::new().prefix(ROOT_PREFIX).tempdir_in(..)` (`make_root`, `:297-312`).
  It is not a copy of the `tempfile::tempdir()` and `h.sock` helpers in `wire_herdr/serve.rs` and `transport_test.rs`
  (`:53-59`), which have no base-length rule, no Herdr home layout and no canonicalization.
- **`prove`** parses with `serde_json`, as the map says (`:150-168`).
- **The `op` names.** `OP_*` (`adapter.rs:55-62`) equal `HerdrOp::as_str`, plus `herdr.connect`. AC 16 takes its
  expected strings from `HerdrOp`, so the two cannot drift apart. The host adapter on `main` uses the same constant
  shape (`OP_ENSURE_SESSION` and the rest). The test kit stays a dev-dependency (ADR-0021 §5).
- **One runner and one poll (Phase 3, warn 3).** `run_bounded` and `poll`. Every wait goes through `poll`: the short
  command's exit (`:276`), the start proof (`:354`), the stop (`:404`), and AC 13's two read polls
  (`scratch_herdr_test.rs:246-257`, through the `pub` `poll`). Nothing reachable could replace either helper, on this
  branch or on `origin/main`:
  - `holler-pane` and `holler-pane-testkit` export no poll or wait helper.
  - `wait_for` and `StateDir` belong to `holler-cli`'s test support.
  - The host and OpenCode adapters' `exec::run` are `pub(crate)` in other adapter crates.
  - `probe::run_bounded` is private, and `run_probe` returns no stdout (finding 1).
  - The tmux sibling on `main` (`real_tmux_test.rs`: `SUN_PATH = 100`, `wait_until`, a `Server` guard) is in another
    crate's tests.
- **F's `run_as`** (`adapter.rs:134-146`). The eight per-method `self.deadline()?` calls are gone, and `deadline()` now
  has one caller (`:142`). The rename happens once, at the port boundary, as Decision 9 allows ("the trait impl calls a
  private method and maps its error once"). The crate had no such helper to extend. `changed_under` maps one variant
  of one exchange. The siblings' `Call` structs are in other adapter crates, which ADR-0021 §5 bars this crate from
  depending on.
- **F's `ensure`** (`:211-253`) is `ensure_pane`'s old body, moved unchanged except for the deadline line.
- **Per-file test helpers.** `adapter_messages_test.rs` repeats the 7-line `spec` from `adapter_test.rs` and adds a
  one-label `config`. `scratch_herdr_test.rs` has its own two-argument `spec` and builds `HerdrFixture` inline. This is
  the local pattern: every `tests/*.rs` here is its own crate and keeps its own small helpers. For example,
  `adapter_conformance_test.rs` has its own `config`, and `adapter_test.rs` has its own `cell` beside `common::cell`.
  The brief also keeps `tests/common/mod.rs` and `adapter_test.rs` untouched.
- **Drift dimensions.**
  - Layering: the transport is unchanged. Its diff against `dc300ab` and against `origin/main` is empty.
  - Dependencies: no manifest line changed except the `tempfile` comment.
  - ADR: ADR-0021 is updated in the same change (A1-A9, 18 lines added and 11 removed).
  - Protocol: no protocol file, golden file or `holler-proto` file is touched.
  - Size: every touched code file is under 800 lines. The largest are `holler-pane/src/error.rs` (713),
    `tests/wire_herdr/mod.rs` (654) and `src/adapter.rs` (493).
  - Public repository: the added lines name no personal infrastructure. The brief quotes the maintainer's own ruling
    from part 2's journal, which is already on `main`. Test literals such as `/home/someone` are generic, and the
    handoffs write `<home>` and `<scratch-root>`.
- **After the merge.** `git merge-tree --write-tree HEAD origin/main` (`d9eabbb`) is clean. Main's ADR-0021 edits
  (#646, #662 and #663) overlap none of A1-A9 in meaning. The changelog has one entry per part.

## Notes for F

None. The verdict is PASS. Findings 1 and 2 concern T's harness and a follow-up issue, not F's code, and neither needs
a change in this PR.
