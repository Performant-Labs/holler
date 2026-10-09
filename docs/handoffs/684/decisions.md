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

## F (Workflow Phase 6, implement; Phase 5 in the role doc) — 2026-10-09T13:12:43-06:00
- **Decided:**
  - Filled the four stubs with the brief's public API exactly, with no extra public item: `host.rs` (`HostOp`, `FakeHost`), `conformance/host.rs` (9 cases), `harness.rs` (`HarnessOp`, `Quirk`, `ServerState`, `ServerView`, `TuiView`, `FakeHarness`) and `conformance/harness.rs` (`HarnessRig`, 15 cases). Added one CHANGELOG entry after slice a's. No `lib.rs`, `conformance/mod.rs`, manifest or lock-file change.
  - Reused slice a without copying it: each fake holds a `FaultSwitch` and calls `enter` first, the ops implement `PortOp::as_str`, and both suites use `run_cases`, `succeeds`, `expect_code` and `expect_eq`. The harness suite folds the rig into the subject so `run_cases` stays unchanged. The frozen `timeout` uses the fault switch's `Timeout { op: op.as_str() }` shape.
  - `harness.rs` is 502 lines, so the brief's `harness/world.rs` fallback was not needed. The three ASSUMPTION comments sit on the `impl HarnessPort` methods (A's W-6(c)).
  - Each state struct derives `Default` and counts what it minted (`FIRST_PID + pids_minted`; the session number is `sessions_minted + 1`), so there is no hand-written state constructor. `freeze`, `thaw` and `kill` share one `World::signal` rule.
  - Folded A's warnings into the docs: W-2 (the fakes share no state) in both fake module docs; W-3 (decisions 5 and 6 with their reasons, and case 7's reading of #641) in the suite docs; W-4 (`server()` names #644 as its consumer); W-5(c) (pane names parsed with `succeeds("PaneName::parse", ..)`, no third `pane_name`).
- **Assumed:**
  - `navigate` (a person moving a TUI by hand) does not reach the server. The brief lists only its no-TUI and unknown-id errors, and no test drives it against a frozen server.
  - `delete_session` sends every TUI showing the id home, whatever its port. Ids come from one counter, so an id names one session across all data directories.
  - Freezing a frozen server and thawing a running one are `Ok` (a signal to a process in that state is harmless). The brief left both open.
- **Hedged:**
  - The brief asks for each ASSUMPTION comment on "one line". Each is one comment whose first line carries the verbatim prefix, wrapped at the file's 100-column width, so AC 7's `grep -c` prints 3. Joining each onto one ~200-character line is a mechanical change if S wants it.
  - The host suite's `holds` (pids) and the harness suite's `holds` (session ids) are two six-line suite-local helpers over different element types. A shared generic one would need a `conformance/mod.rs` edit, which this slice may not make. It is a candidate for the W-5 follow-up.
- **Evidence:**
  - `cargo test -p holler-pane-testkit`: every test GREEN, including the 56 new ones (host conformance 12, fake host 12, harness conformance 13, fake harness 19) and slice a's 32 unchanged.
  - `cargo build --workspace` exit 0. `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's command) exit 0: 109 test binaries, 1164 passed, 0 failed, 5 ignored. `bash scripts/test-hooks.sh` exit 0 (11 ok).
  - Clean: `cargo clippy --workspace --all-targets -- -D warnings` (exit 0, no diagnostics), `bash scripts/lint.sh` (exit 0, no testkit warning), `rustfmt --check --edition 2021` on the four files, `cargo machete`, `scripts/changelog-check.sh`. The `cargo tree` dependency rule printed nothing; `grep -c` of the ASSUMPTION prefix printed 3.
  - A throwaway crate outside the repo ran every mutant through the suites and printed every failing case with its detail. Each mutant fails its named case for the intended reason. The extra failures are expected knock-ons: `RunIsNoop` fails every case that needs a started process, `PsOfMissingIsEmpty` also fails case 2's re-check, and a separate data dir also fails case 15's attach on the second port.
  - Facts in unchanged code that the diff relies on are in `evidence.md`.

## T-green (Workflow Phase 7; Phase 6 in the role doc) — 2026-10-09
- **Decided:** Phase 7 is GREEN with no blocking Tier 2 issue; no test was changed (F flagged none as wrong) and no production code was touched.
- **Assumed:** F's two "advisory" items (two-line ASSUMPTION comments, two `holds` helpers) are not blocking: the AC `grep -c` prints 3 and the helpers are suite-private.
- **Hedged:** Mutation spot-check covered three behaviors (frozen, killed, stop_owned) by hand; F's wider mutant readout was not re-run.
- **Evidence:** `cargo test -p holler-pane-testkit` all GREEN (56 new + 32 slice a); workspace CI command 1164 passed / 0 failed / 5 ignored; clippy, lint, changelog-check, machete, dependency grep, ASSUMPTION grep all as expected; three mutants each failed a named test and were reverted.

## A (Phase 7, anti-duplication gate) — 2026-10-09T13:25:08-06:00
- **Decided:**
  - PASS on `e410e9d...d6f5850`, with no blocks and three `warn`s (see `handoff-A-dup.md`).
  - F extended slice a and built no parallel path. Both fakes call `FaultSwitch::enter` first in every port method, and both ops implement `PortOp::as_str`. Both suites run on `run_cases`, `succeeds`, `expect_code` and `expect_eq`, unchanged, and the harness suite folds its rig into the subject. Every error is a closed `PaneError` variant, and the frozen `timeout` has the fault switch's shape. The public surface is the brief's API, item for item.
  - The warns:
    - W-1 (carried forward from Phase 3): `HarnessRig` here and #683's `HerdrFixture<H>` are two shapes for one idea.
    - W-2: small copies that the no-edit rule forces (per-fake `lock`, `holds`/`lacks`, the rig's `scratch`/48100, `assert_fails_on`). New since Phase 3: #682 makes `feed::lock` `pub(crate)`, so after the merges there is one shared lock beside five per-struct copies.
    - W-3: `lib.rs:27-28` ("empty stubs") becomes stale. #681's gates gave it to #684, but this brief's AC 9 forbids the edit.
  - All three go to one cleanup issue. It does not exist yet, so O opens it.
- **Assumed:**
  - Slices b to e merge in some order before #640, #641 or #642 call a suite, so the cleanup can settle the fixture shape before any adapter depends on it.
  - #682's `feed::lock` promotion and #683's planned move to it land as their worktrees have them now; neither has merged.
- **Hedged:**
  - W-1 stays a `warn`, not a `block`. Both shapes are now real, but the brief specifies `HarnessRig` (Decision 8), neither shape is on main, and the role's rule for an inconsistent codebase with no dominant pattern is `warn`.
  - W-3 is a doc item outside this diff. I recorded it only because a sibling's gate assigned it to this slice.
- **Evidence:**
  - Read in full: the four source files and four test files of the diff, the brief, `handoff-A.md`, `handoff-F.md`, `handoff-T-red.md`, `handoff-T-green.md`, `evidence.md` and this journal. Also read slice a's `fault.rs`, `conformance/mod.rs`, `pane_store.rs`, `conformance/pane_store.rs`, `fixture.rs`, the tail of `feed.rs`, `lib.rs` and the mutation part of `tests/pane_store_conformance_test.rs`.
  - Workspace greps: the only other `HostPort`/`HarnessPort` impls are the CLI's `Unwired` placeholder and `holler-pane`'s signature pins. There are no other `Quirk`, `ServerState`, `ServerView`, `TuiView` or rig types. The added lines touch no overlay candidate and name no personal infrastructure. All five commits use the GitHub no-reply address.
  - Sibling worktrees (read only): #682's diff to `feed.rs`, `fixture.rs`, `pane_store.rs` and `conformance/pane_store.rs`; #683's `conformance/herdr.rs`, lock helpers and handoff-A row 3; #681's handoff-A W-6, handoff-A-dup row 2 and `lib.rs` diff.
  - `gh issue list` searches found no test-kit cleanup issue.

## S (Phase 10, spec audit) — 2026-10-09T13:33:19-06:00
- **Decided:**
  - PASS on `e410e9d...d30873e` (see `handoff-S.md`). Every brief AC (1 to 11) and every acceptance item of issue #684 has a proving test or evidence. The public API, the behaviour tables, the 7 exact error values and decisions 1 to 11 are implemented as the brief states them.
  - Accepted F's declared deviation: the three ASSUMPTION comments are wrapped at 100 columns, not one physical line. The prefix and text are verbatim, and AC 7's `grep -c` prints 3.
  - Accepted the brief's two adjustments to the issue's wording as recorded, not silent: `shown_session` keeps answering on a frozen or killed server (decision 1), and `serve` restarts a killed server. Also accepted F's three behaviours that the brief left open: `navigate` does not reach the server, `delete_session` sends TUIs home on any port, and freezing a frozen server or thawing a running one is `Ok`.
  - The scope matches the brief. The issue's short Blast radius list is not a deviation, because the issue's own scope requires the suites and mutation checks, and slice a's stubs assign `conformance/{host,harness}.rs` to #684.
- **Assumed:**
  - F's and T-green's Tier 1 records (clippy, workspace tests, rustfmt, lint, machete, changelog-check, test-hooks) are accurate. I did not re-run Tier 1. Git shows that no file under `crates/` changed after F's commit, so those runs cover HEAD.
- **Hedged:**
  - The commit trailers carry no session link. That is the workflow script's commit format, the same on every merged pipeline run, and the squash merge replaces it, so I did not count it against the slice. There is no PR yet, so the PR-body AI disclosure and the list of what binds #641 and #642 are left to the run's agent after `gh pr create` (advisory 1).
  - The cleanup issue that A-dup asked O to open (one fixture shape, `tests/support`, `holds`/`lacks`, the lock helper, fixture constants, the stale `lib.rs:27-28`) still does not exist. It is advisory for O, and should be settled before #640 or #642 writes its suite runner.
  - `thaw_brings_a_frozen_server_back` does not call `serve` or `attach_tui` after the thaw. Other tests cover both, so this is not blocking.
- **Evidence:**
  - Read in full: issue #684, the brief, every handoff in `docs/handoffs/684/`, the four source files, the four test files and the CHANGELOG diff.
  - `git diff --name-only` / `--stat` for the scope and untouched files. `git show --name-only` per commit, and `git diff b7d3ce2 HEAD -- tests` / `git diff 75cca26 HEAD -- crates` (both empty), for phase separation. `git show b7d3ce2:<src>` showed the src files were still stubs at RED.
  - `wc -l` (largest 576). `grep -c` ASSUMPTION = 3. A `grep` for `unwrap`, `expect`, `panic`, `unreachable` and `assert` in `src/` (only the two `text`-fenced usage examples). A `grep` of `pub` items against the brief's API.
  - `cargo tree --offline` (the only `holler-*` crates are `holler-pane` and its `holler-proto`; the forbidden count is 0).
  - The privacy grep of added lines (clean). `.githooks/commit-msg` against the six subjects. The spike lines 175-180 and 264-275 for the ASSUMPTION citations. `gh pr list` (no PR yet), and `gh issue list` searches (no cleanup issue).
