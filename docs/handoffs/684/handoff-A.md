# Handoff-A: Phase 3 - #684 test kit slice e: `FakeHost`, `FakeHarness` and their conformance suites  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-684-implementation (at bb2b6b2; base e410e9d = origin/main)
**Brief reviewed:** docs/handoffs/684-brief.md   **Reuse map:** docs/handoffs/684-brief.md, the "Extend vs new" section and the "What slice a left to reuse" evidence block (this run has no survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS. The plan extends what slice a built instead of building beside it:

- Both fakes hold a `FaultSwitch<Op>` and call `enter` first in every port method.
- `HostOp` and `HarnessOp` implement the merged `PortOp::as_str`.
- Both suites run on `run_cases`, `succeeds`, `expect_code` and `expect_eq`, with one `CASES` table each.
- Only closed `PaneError` variants are returned.
- The four stubs, the module paths and the dependency on `holler-pane` alone stay as slice a fixed them.

What I checked:

- Every cited line I checked matches the code.
- The suites' layering holds: neither suite names a fake.
- The brief's central reuse claim holds. I compiled a standalone copy of `run_cases` with the brief's fold: `S = (H, HarnessRig)`, the closure binds `&H`/`&HarnessRig`, and `&H` coerces to `&dyn HarnessPort`. It builds and runs.
- Each closed code the suites hold #641 and #642 to is inside its stated meaning in ADR-0021 section 9 and `error.rs`. #683's suite was blocked for stretching `grid-out-of-range` past its ADR definition; nothing here does that, so no ADR edit is needed.

All six findings are `warn`. Two are worth settling before T writes tests:

- W-1: the sibling slice #683 gives its suite a different fixture shape.
- W-2: the two fakes share no state, and the brief does not say so.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | `HarnessRig` and `run_harness_conformance` with `F: FnMut() -> (S, HarnessRig, K)` (Decision 8) | pattern consistency, naming | Slice d (#683) is in flight in parallel and solves the same need: the implementation under test supplies each case's data. It bundles that data with the port: `HerdrFixture<H> { pub port: H, pub session, pub workspace }` with `F: FnMut() -> (HerdrFixture<H>, K)` (`.claude/worktrees/0683-herdr-fake/docs/handoffs/683-brief.md:300, 307-308`; unchanged by its amendment 9d79d0e). This brief instead uses a separate `HarnessRig` and a 3-tuple. Both reuse `run_cases` unchanged, and neither shape is on main, so there is no dominant pattern to block on. Once both merge, though, the crate has two shapes and two words ("Fixture", "Rig") for one idea, and #640 and #642 learn two calling conventions. The ASSUMPTION prefixes also differ: `// ASSUMPTION (#640):` in #683, `// ASSUMPTION (#642 to confirm):` here. | Pick one shape for both slices and record it in decisions.md. My lean is #683's bundle, because its `fresh` goes to `run_cases` with no re-tupling closure, which is closest to slice a's `run_cases(&CASES, fresh, \|case, store\| case(store))`. For this slice that would be `pub struct HarnessFixture<H> { pub harness: H, pub ports: [u16; 2], pub panes: [PaneId; 2] }`, `F: FnMut() -> (HarnessFixture<H>, K)`, and a `sample(harness)` constructor. If the brief is not amended before T, record the divergence as deliberate and align the two in the W-5 follow-up. |
| 2 | warn | Decision 10 ("Not modelled"); the `FakeHost` and `FakeHarness` docs | abstraction level (fidelity of fakes used together) | In production the two ports meet in the OS. The spike's adapter design makes `stop_owned` kill the harness server's process group and its descendants (opencode-pane-spike.md:175-177, 244). The server's pid is what a pane record keeps (`HarnessInfo.pid`, pane.rs:144-150). In the plan the two fakes are independent: `FakeHost::stop_owned` leaves a `FakeHarness` server `Running`, so `stop_owned` then `serve(name, port)` (what a relaunch does) gets the **old** pid back from the idempotent `serve`. A harness pid (from 20_000) never appears in `FakeHost::ps` (from 10_000), and `run(["opencode","serve",...])` starts no harness server. Independence is right for this slice: each fake implements one port, as each adapter does (epic: "each adapter implements one"), and coupling them would invent architecture. But nothing tells #644 (relaunch) or #647 (doctor: "a registered pane whose process died") that they must make the two fakes agree themselves. | No code change. Add an item to Decision 10 and one sentence to each fake's module doc: "The host and harness fakes share no state. `stop_owned` does not stop a harness server, and a harness pid is not in `ps`. A test that needs them to agree drives both, e.g. with a `HostPort` wrapper whose `stop_owned` also calls `FakeHarness::kill(port)`." |
| 3 | warn | Decisions 5 and 6; host cases 1, 2, 5, 7; harness cases 6, 8, 11, 13, 14 | cross-cutting (error taxonomy; contract across stories) | The suites are the contract that #641 and #642 must pass, so every code they pin binds those stories. I checked each pinned code against ADR-0021 section 9 (lines 375, 388-389, 393) and `error.rs`. Each is inside its stated meaning, so the stack rule "ADR updated in the same change" does not fire here, as it did for #683. `pane-not-found`, `session-not-found` and `unavailable` are tagged "(#638-#642.)" in `error.rs:454-467`, which names these adapters as raisers. `usage` covers "a missing ... argument" (`error.rs:407-409`). The closest call is `unavailable` for "no TUI in the pane", which I read as "a harness cannot be reached". One pin goes against an issue's wording: host case 7 `stop-owned-of-missing-session-is-ok` against #641's acceptance "a missing session is a typed error". Decision 6 reads that as covering only `run` and `ps`. That reading is sound (`relaunch` and `close` after a crash must not fail), but #641's author will meet it only when the suite runs. | Keep Decisions 5 and 6, with their reasons, in the suites' module docs, as Risks already plans, and state the reading of #641 at case 7. In the PR body, list what binds each adapter. For #641: `stop_owned` of a missing session is `Ok`; `run` and `ps` of one are `pane-not-found`; an empty argv is `usage`. For #642: no TUI is `unavailable`, checked before the session id; reachability is checked before existence; `shown_session` is `Ok(None)` with no TUI. #683's review asked the same for #640 (its W-2). |
| 4 | warn | `FakeHarness::server()`, `ServerView`, `ServerState` | abstraction level (public API with no consumer) | Every other scenario and inspection method of the two fakes has a named consumer, and an AC 5 or AC 6 test reads its result. `server(port)` has neither. Its `name` and `pid` and the `Frozen` and `Killed` states are public API that no test pins, and public items escape `dead_code`, so nothing would flag it. (`ServerState` is the fake's ground truth. It does not duplicate `holler_pane::Health`, which is an observed, stored value.) | Name the consumer in the method's doc (e.g. a #644 launch test checks which pane's server runs on a port), and pin it in AC 6: `serve_on_a_running_port` and the frozen and killed tests assert that `server(p0)` equals `Some(ServerView { name, pid, state })`. If there is no consumer, drop the method and keep `ServerState` private to `World`. |
| 5 | warn | AC 2 and AC 4 (`assert_suite_fails_on`); "Extend vs new" (the lock idiom); parsing pane names in the suites | anti-duplication (test scaffolding and small helpers) | (a) Each of the two new mutation-test files copies slice a's `assert_suite_fails_on` (`tests/pane_store_conformance_test.rs:251-263`). #683's review (its W-6) accepted the second copy and said that if slice c or e copied it again, it should move to `tests/support/mod.rs`. This slice adds the fourth and fifth copies. (b) The brief says a shared lock helper "would need a `conformance/mod.rs` or `lib.rs` edit". In fact a generic private `fn lock<T>(&Mutex<T>)` already exists at `feed.rs:216-220`, so sharing it would mean editing slice a's `feed.rs`, which is the wrong home for it. The brief's conclusion still holds: one private `lock()` per fake, as in `fault.rs:105-107`. (c) The suites need `PaneName::parse` mapped to `String`. Slice a's version is a private three-line `pane_name` (`conformance/pane_store.rs:415-417`). | (a) Accept the copies in this slice. Slices c, d and e run in parallel, and a `tests/support/mod.rs` added now would collide with any sibling that adds its own. O opens one follow-up for after #638's last slice merges: move `assert_fails_on(result: Conformance, case: &str)` into `crates/holler-pane-testkit/tests/support/mod.rs` with `#![allow(dead_code)] // #NNN`, the layout the repo already uses (`holler-pane/tests/common/mod.rs:1`, `holler-hub/tests/pane_support/mod.rs:1`, `holler-proto/tests/common/mod.rs:1`), and settle W-1 in the same change. Phase 7 will not reject these copies. (b) Correct the stated reason in the brief. (c) Use the shared helper, `succeeds("PaneName::parse", PaneName::parse(C1))?`, instead of a third copy of `pane_name`. Phase 7 checks this. |
| 6 | warn | Brief header; "Decisions made in this brief"; AC 7 under the `world.rs` fallback | ADRs and file structure (bookkeeping) | (a) The "Decision record" line cites ADR-0021 sections 5 and 7. Section 7 is about the hub's persistence and governs nothing in this slice. The sections the decisions actually rest on are: section 2 (`HarnessPort` is provisional, under the amend-first rule; Decision 3 and Out of scope); section 4 (the I2, I3, I5 and I6 test ideas these fakes serve); section 9 (the code classes behind Decisions 5 and 6); and "Deferred to named stories" (`HarnessPort`'s final form belongs to #642). (b) The issue's Blast radius lists only `src/host.rs`, `src/harness.rs` and `CHANGELOG.md`. The brief adds `src/conformance/{host,harness}.rs` (slice a's stubs assign them to #684), four test files and the optional `src/harness/world.rs`. All of these are needed and fall inside #638's `crates/holler-pane-testkit/**`, but no decision records the widening, and AC 9 checks against the brief's list. (c) AC 7 counts the ASSUMPTION comments in `src/harness.rs` only. | (a) Cite sections 2, 4 and 9 and "Deferred to named stories". (b) Add a decision that records the widening and its basis, so S does not flag it. (c) If F uses the fallback, keep the three ASSUMPTION comments on the `impl HarnessPort for FakeHarness` methods in `harness.rs`, and move only `World` and its helpers to `harness/world.rs`. |

Apart from these findings, the plan matches existing patterns. Checked against the code:

- **Evidence.** Every cited line I checked matches `bb2b6b2`:
  - the ports, `Argv`, `PaneName`/`PaneId`, and the error variants and their classes;
  - `fault.rs`, `conformance/mod.rs`, `pane_store.rs` and the slice a tests;
  - the spike's lines.
  `PaneName` has a hand-written `Debug`, so `ServerView` can derive `Debug`.
- **Layering.** The suites reach implementations only through the port, so they can run against the real adapters. The scenarios the port cannot cause (freeze, kill, delete, navigate, close) are confined to the fake-only tests.
- **Fault switch held directly.** The fakes hold their `FaultSwitch` directly, not in an `Arc` as `FakePaneStore` does. That is justified: neither port has a `watch` stream that must outlive a borrow.
- **No parallel path.** The HTTP-level fake OpenCode servers (`holler-body/tests/http_attach_driver_test/fake_server.rs`, `holler-cli/tests/attach_cli_test/fake_server.rs`) sit at attach mode's wire seam (ADR 0014), not at `HarnessPort`, and the test kit cannot depend on them. The frozen `timeout` uses the fault switch's `Timeout { op: op.as_str() }` shape; there is no helper to call, and adding one would mean editing slice a's file.
- **Naming.** The names follow slice a and the sibling slices:
  - `HostOp`/`HarnessOp` like `PaneStoreOp`/`HerdrOp`;
  - `"host.*"`/`"harness.*"` matching the module names;
  - `Fake<Port>`, `faults()`, `<port>_cases()`, `run_<port>_conformance`, and the `tests/` file names.
- **Fallback layout.** The fallback `src/harness/world.rs` under `mod world;` follows the repo's `circuit.rs` + `circuit/` layout and leaves `lib.rs` untouched.
- **Dependency direction** (ADR-0021 section 5). Standard library only, no manifest edit, and AC 8's `cargo tree` guard.
- **Size.** The largest estimate is ~520 lines (`harness.rs`), under `scripts/lint.sh`'s 600-line warning and 900-line failure, and the brief says where the overflow goes. `struct_excessive_bools` is not at risk: the quirks are a set, not bool fields.
- **Public repository.** Only neutral names (`demo-c1r1`, `scratch`) appear.

## Notes for O

(PASS: nothing blocks T.)

If you can amend the brief before T:

1. **W-1:** settle the suite fixture shape with #683 and record the choice in decisions.md. If you do not amend, record that the divergence is deliberate and fold the alignment into the W-5 follow-up.
2. **W-2:** add the Decision 10 item and the doc sentence.
3. **W-6 (a) and (b):** the citation and the blast-radius decision.

For F and T as the brief stands:

- **W-3:** put Decisions 5 and 6 in the suites' docs, and list the pins that bind #641 and #642 in the PR body.
- **W-4:** pin `server()` or drop it.
- **W-5 (c):** use `succeeds` to parse pane names; no third `pane_name`.
- **W-6 (c):** keep the ASSUMPTION comments in `harness.rs`.

One follow-up issue for after #638's last slice merges: W-5 (a) (the shared `tests/support` helper), plus W-1 if it is still open.

## Patterns referenced

- `crates/holler-pane-testkit/src/fault.rs:17-134`, `src/conformance/mod.rs:31-100`, `src/pane_store.rs:20-222`, `src/conformance/pane_store.rs:18-116 and 411-417`, `src/fixture.rs:10-31`, `src/feed.rs:216-220`, `tests/pane_store_conformance_test.rs:151-311`: the switch, runner, assertions, case table and mutation pattern this slice extends.
- `crates/holler-pane/src/ports.rs:150-200`, `src/error.rs:262-298 and 395-467`, `src/argv.rs:20-74`, `src/pane.rs:25-89 and 131-150`.
- `docs/adr/ADR-0021.md`: section 2 (84-103), section 4 (156-167), section 5 (169-192), section 9 (304-403), and "Deferred to named stories" (516-528).
- `docs/research/opencode-pane-spike.md:55-275`. Also `docs/research/herdr-api-spike.md:137, 142`: Herdr's `PaneInfo.title?` is set through `pane.report_metadata`, and the spike does not show that it carries the terminal title, so the `shown_session` ASSUMPTION stays open.
- `.claude/worktrees/0683-herdr-fake/docs/handoffs/683-brief.md:294-308` and `.claude/worktrees/0683-herdr-fake/docs/handoffs/683/handoff-A.md` (the sibling slice's fixture shape, and its W-2 and W-6). The repo's shared test-module layout: `crates/{holler-pane,holler-proto}/tests/common/mod.rs`, `crates/holler-hub/tests/pane_support/mod.rs`. Issues #684, #638, #633 (the "Skeleton split" section), #641, #642.
