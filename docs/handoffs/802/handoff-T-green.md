# Handoff-T-green: Phase 7 (Workflow script; the role doc's Phase 6), round 3 - #802 after the diff gate's B-1: `verify_serving` answers for the spec, not for whether the server is up

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, head `7ee7d8e`)
**Issue:** #802 (closes #695)
**Handoff-F reviewed:** `docs/handoffs/802/handoff-F.md` (round 3)
**Handoff-T-red:** `docs/handoffs/802/handoff-T-red.md` (round 2; round 1 at `git show 26148ba:docs/handoffs/802/handoff-T-red.md`)
**Diff gate:** `docs/handoffs/802-diff-result-r1.md`
**Brief:** `docs/handoffs/802-brief.md` as amended in `0dc4e8b`

Earlier T-greens:
- Round 2 (PASS): `git show 2a32fc8:docs/handoffs/802/handoff-T-green.md`.
- The B-1 BLOCK: `git show dab3cd3:...`.
- Round 1 (PASS, with the per-AC table and mutation checks M1 to M6): `git show 3fe111d:...`.

**VERDICT: PASS.** The suite is GREEN, and Tier 1 and Tier 2 are clean. F's round 3 changes doc
comments, the ADR and the CHANGELOG only. I took three test-side items the diff gate left in T's
lane. Each is a test-only edit, and each has a mutation check that catches its mutant.

## What F changed, checked

`git diff 2a32fc8 7ee7d8e -- crates/` holds only `///` and `//!` lines in four files: `ports.rs`,
`tx_launch.rs`, `tx_launch/server.rs` and `holler-adapter-opencode/src/lib.rs`. No code changed.

I checked F's reply to B-1 against the brief, and the brief backs it:
- Line 11, decision 5: "here the OpenCode implementation checks nothing beyond `Ok(())` for a spec
  with no agent".
- Line 33, Scope 3: "`verify_serving` returns `Ok(())` for a spec without an agent".
- Line 41: "implementing `verify_serving` for real (#642 part 3)" is Out of scope.

The test `hermetic_test::ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies` (nothing
sent) still passes. The new doc text says that a server gone after the health wait fails the
session of record's call and rolls back. Before this round no test showed that, so I added one
(T3 below).

## Test changes this round (test-only, no production code)

| # | Test | Why | Tier |
|---|---|---|---|
| T1 | `fake_tmux_test::run_escapes_the_cwd_for_tmux`: a guard asserts that `fake.dir_str()` holds no `#` or `;` | Diff gate W-3: the expected `-c` joins the fixture prefix unescaped, so a prefix with either character would make the assertion depend on the fixture. | unit-ish (fake tmux) |
| T2 | `run_checks_the_session_before_the_argv` renamed to `run_checks_the_session_then_the_argv_then_the_cwd`. Both calls now break all three: missing session or not, empty argv, and `cwd = ""`. The second call asserts that the usage message is the argv's, not the directory's. | Diff gate NV-2. `TmuxHost::run`'s doc (in this PR) promises "the session is checked first, then the argv, then `cwd`", but only session-before-argv was pinned. No other file referenced the old name. | fake tmux |
| T3 | New: `launch::serve::a_server_gone_after_verify_fails_the_session_of_record_and_rolls_back`. `rig.after(VerifyServing, kill(PORT))`, then a plain launch. Asserts: exit 1, `unavailable`, the reconcile step once, harness steps exactly `[ServeCommand, Health, VerifyServing, CreateSession]`, `StopOwned` and Herdr `Close` called, no Herdr pane left, no record. | Gives the round-3 contract text (`ports.rs`, `server.rs` step 4, ADR-0021 decision 5) and F's B-1 rebuttal a test, so the next diff gate can verify the claim. It kills the fake server for real; no fault is injected. | engine over the rig's fakes (the claim is about the run's rollback) |

Mutation spot-checks. Each mutant was applied to `src/`, run, and the file restored from a copy.
`git status` shows only the two test files.
- **M7**: `check_cwd` before `check_argv` in `TmuxHost::run`. T2 fails: "a bad argv comes before a
  bad cwd: cannot run a program in the tmux session demo-c1r1: its directory must be ...". This is
  new coverage; the old test passed this mutant.
- **M8**: both checks before the session read. T2 fails: "a missing session comes before a bad
  argv and cwd: Err(Usage { message: "an empty argv has no program to run" })".
- **M9**: `session_of_record(...)` error swallowed in `bring_up` (`tx_launch.rs:660`). T3 fails:
  `left: [ServeCommand, Health, VerifyServing, CreateSession, AttachTui]`,
  `right: [ServeCommand, Health, VerifyServing, CreateSession]`.

T3 passed 5 of 5 runs of `launch::serve`, 16/16 each time, 1.20 s.

## GREEN confirmation

```text
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
148 binaries, passed=2073 failed=0 ignored=25, exit 0          # on 7ee7d8e, before my test edits; equals F's
$ cargo test -p holler-cli --test pane_verbs                   # after my edits
test result: ok. 262 passed; 0 failed                           # 261 + T3
$ cargo test -p holler-adapter-host --test fake_tmux_test      # after my edits
test result: ok. 29 passed; 0 failed
$ cargo test -p holler-adapter-opencode --test hermetic_test ac8
5 passed (ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies ... ok)
```

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Verify command | the workspace command above | 2073 / 0 / 25, exit 0. PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings`, after touching the `holler-pane`, `holler-adapter-opencode` and `holler-adapter-host` roots; re-run after my edits | exit 0. PASS |
| Lint | `bash scripts/lint.sh` | exit 0, size warnings only (`fake_tmux_test.rs` 818, `tx_launch.rs` 820). PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok`. PASS |
| Unused deps | `cargo machete` | none. PASS |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane -p holler-pane-testkit -p holler-adapter-opencode` | exit 0. PASS |
| rustfmt | `rustfmt --edition 2021 --check` on F's four changed `.rs` files and my two | exit 0. PASS |
| docs_cli_test | `cargo test -p holler-cli --test docs_cli_test` | 3 passed. PASS |
| wire_selftest | `cargo test -p holler-cli --test wire_selftest` | 3 passed. PASS |
| Hook tests | `bash scripts/test-hooks.sh` | exit 0. PASS |

**Cross-check of F's numbers.** Every command F lists gave the same result here: workspace
2073/0/25, `docs_cli_test` 3, ac8 nothing sent, and `pane_verbs` 261 before T3. The opt-in
`real_opencode_test` was not re-run. Only doc comments changed since round 2's 9/9, and it needs a
private opencode server.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Round 3 is doc-only | `git diff 2a32fc8 7ee7d8e -- crates/`: only doc comment lines | PASS |
| Docs match behaviour | The new `verify_serving` paragraph ("answers for the spec, not for whether the server is up"; "a server gone since then fails the engine's next call to it ... which rolls back the same way") is now pinned by T3. The OpenCode "sends nothing" doc is pinned by ac8. Step 4 of `server.rs`, `Plan::observe`'s order (W-1) and the ADR/CHANGELOG wording agree with the code. | PASS |
| B-1 vs the brief | Brief lines 11, 33 and 41 (quoted above) require `Ok(())` with no check for a no-agent spec. F's refusal of the health-call remediation follows the brief. | PASS |
| Coverage per AC | AC 1-12 as in `3fe111d`, plus AC 6 as in round 2. AC 7's rollback now also has the server-gone path (T3). AC 4's `run` check order has T2. | PASS |
| Test quality | T1 is a fixture precondition, not a new behaviour. T2 replaces a test rather than adding one, so the suite does not grow. T3 is the only new test, it names one behaviour, and M9 shows it fails for the right reason. It does not duplicate `a_failed_attach_rolls_back`, which fails a later step through a fault, nor `a_verify_refusal_rolls_back_and_stops_the_server`, which is a refusal, not an unreachable server. | PASS |
| Timing | T3 uses the rig's synchronous hook, with no sleep or duration assertion. 5/5 stable. | PASS |
| Error paths / security | T3 asserts the `unavailable` code and exit 1. T2 keeps the "never echo" rules covered by the neighbouring tests. No secret is involved. | PASS |
| Evidence appendix | F's three new entries checked against source: `tx_launch.rs:676-684`, `lib.rs:451-452` and `lib.rs:409-415`, plus the `.map_err` at `tx_launch.rs:607-608`. All verbatim. I appended two entries for T3's reliance on unchanged fake code: `harness.rs:351-356` (`create_session` reaches the server first) and `harness.rs:424-430` (a killed server is `unavailable`). | PASS |
| Protocol, goldens, Playwright | No such change or surface | N/A |

## Acceptance criteria status

- AC 1-12: PASS, all GREEN in the workspace run, with the backing tests from `3fe111d` and round 2.
- AC 4 adds T2 for `run`'s check order. AC 7 adds T3 for a server gone after the health wait.

## Blocking issues

None.

## Advisory notes

- The diff gate's other items:
  - NV-1: the AC 6 tests are GREEN in the workspace run.
  - NV-3: the doctests are GREEN in the workspace run, as F's `--doc` run also showed.
  - NV-4: needs the opt-in real run (9/9 in round 2).
  - W-2 and NIT-1 to NIT-3: F's call, and I have nothing to add from the tests.
- If the outside reviewer still prefers a health call in `verify_serving`, that reverses brief
  decision 5. It is the operator's call, and it would turn ac8's "nothing sent" assertion into the
  thing to change.
- Carried over:
  - `tx_launch.rs` is at 820 lines and `fake_tmux_test.rs` at 818. The lint fails at 900.
  - W-11(a), W-13 and W-15, as in round 2.
