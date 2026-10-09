# Decision journal: #684 (test kit slice e: `FakeHost`, `FakeHarness` and their conformance suites)

## A (Phase 3, up-front plan review) — 2026-10-09T12:49:55-06:00
- **Decided:**
  - PASS with six `warn`s (W-1 to W-6 in `handoff-A.md`); nothing blocks T.
  - The plan extends slice a: it reuses `FaultSwitch` and `enter`, `PortOp::as_str`, `run_cases`, `succeeds`, `expect_code` and `expect_eq`, the `CASES` table, and the `Break`/`Mutant` pattern and naming. It returns closed `PaneError`s only and adds no dependency.
  - Every closed code the suites hold #641 and #642 to is inside its stated meaning in ADR-0021 section 9 and `error.rs` (`pane-not-found`, `session-not-found` and `unavailable` are tagged "(#638-#642.)"; `usage` covers a missing argument). So, unlike #683, no ADR amendment is needed.
  - The two fakes sharing no state is accepted, but must be documented (W-2).
  - The copies of `assert_suite_fails_on` in this slice are accepted. They are consolidated into `tests/support/mod.rs` by one follow-up after #638's last slice merges (W-5). Phase 7 will not reject them.
- **Assumed:**
  - #683 merges the `HerdrFixture<H>` suite shape its brief has at 9d79d0e.
  - The real adapters can meet every suite case. I reasoned through each case against the OpenCode spike and tmux behaviour, but nothing here runs them.
- **Hedged:**
  - W-1 (one suite-fixture shape for slices d and e) is a decision across slices, for O or the operator; my lean is #683's bundle.
  - `unavailable` for "no TUI in the pane" is the closest reading of a closed code in this slice. I judged it inside "a harness cannot be reached".
  - Host case 7 (`stop_owned` of a missing session is `Ok`) narrows #641's "a missing session is a typed error" to `run` and `ps`. I judged that sound, but it binds #641, so it belongs in the PR body (W-3).
- **Evidence:**
  - Read: the brief; issues #684, #638, #633, #641 and #642; ADR-0021 sections 2, 4, 5, 7 and 9 and its "Deferred" and "Decisions taken" lists; `docs/research/opencode-pane-spike.md`; the `title` rows of `herdr-api-spike.md`.
  - Code read: all of `crates/holler-pane-testkit/src` and its slice a tests; `crates/holler-pane/src/{ports,error,argv,pane,probe}.rs`; `scripts/lint.sh`; the workspace lints and `clippy.toml`.
  - Sibling slices: #683's brief and its `handoff-A.md` (its W-6 on scaffolding copies), and #682's brief (its test plan).
  - A standalone `rustc --edition 2021` compile of the brief's `run_cases` fold (`S = (H, HarnessRig)`) built and ran, so the fold needs no change to `run_cases`.

## T (Phase 4, author / RED) — 2026-10-09T13:20:00-06:00
- **Decided:**
  - Wrote the four test files of AC 1 to 6 (`host_conformance_test.rs`, `fake_host_test.rs`, `harness_conformance_test.rs`, `fake_harness_test.rs`), all integration tests at the crate's public API; no `[[test]]` entry is needed (testkit has default autotests, `Cargo.toml` untouched).
  - RED is "does not build on the missing items" (the brief's test plan). Confirmed only `E0432` on the brief's API names. Because `E0432` hides type errors in test bodies, also compiled and clippy-checked the tests against a temporary signature-only copy of the API, then restored the stubs with `git checkout`.
  - Each of the two suites' mutation tests share a local `assert_fails_on(result: Conformance, case)` (A's W-5(a) shape); the quirk and data-dir mutants run the fake itself via `fresh`, the others use a `Mutant` wrapper.
  - Added two tests beyond the brief's names: `a_fresh_session_has_no_process_and_survives_stop_owned` (Decision 7), `attaching_again_replaces_the_tui_and_a_refused_attach_leaves_it` (a brief behaviour row the suite does not reach), and `exiting_a_process_is_not_a_call_through_the_port`.
  - Pinned `server()` through `ServerView` equality in five tests (A's W-4).
- **Assumed:**
  - `PaneName`, `PaneId` and `PaneError` keep their derived `Debug`/`PartialEq`; the signature-only compile proves this for today's code.
  - `FakeHost::exit_process(&a, 1)` fails because host pids start at 10_000 (brief), so pid 1 is never held.
  - `SelectBroadcasts` may call `HarnessRig::sample().panes` since the mutation tests always run the sample rig.
- **Hedged:**
  - The tests only compile against stubs here; whether F's real behaviour matches every table row is verified at GREEN (Phase 6), not now.
  - `fake_harness_test.rs` is 576 lines, near the 600-line `lint.sh` warning; if F's behaviour needs more tests, split rather than grow it.
- **Evidence:**
  - `cargo test -p holler-pane-testkit --no-run` (RED, E0432 only); the same with signature stubs (builds, six binaries); `cargo clippy -p holler-pane-testkit --tests -- -D warnings` against the stubs (clean); `rustfmt --check --edition 2021` on the four files; `bash scripts/lint.sh` exit 0.
