# Handoff-T-green: #846 linkedhost-fakes (fold the LinkedHost fakes and the #649 near-copies)

**Date:** 2026-10-12
**Branch:** issue-846-implementation
**Issue:** #846
**Handoff-F reviewed:** docs/handoffs/846/handoff-F.md
**Handoff-T-red:** docs/handoffs/846/handoff-T-red.md

## GREEN confirmation
`cargo test -p holler-cli --test pane_verbs --test profile_verbs --test pane_claude --test pane_integration --test profile_apply_scenario`
(re-run by T, exit 0). Identical to the T-red baseline in every target:

| Target | Baseline | After F |
|---|---|---|
| pane_claude | 43 passed, 1 ignored | 43 passed, 1 ignored |
| pane_integration | 71 passed, 4 ignored | 71 passed, 4 ignored (151.5 s) |
| pane_verbs | 347 passed, 0 ignored | 347 passed, 0 ignored |
| profile_apply_scenario | 5 passed, 3 ignored | 5 passed, 3 ignored |
| profile_verbs | 118 passed, 0 ignored | 118 passed, 0 ignored |

No test was added, removed or edited (the diff touches only support/rig modules), so the counts are the proof of unchanged behaviour.

### Mutation spot-check (each applied to `verb_harness/linked_host.rs`, then reverted; tree clean afterwards)
| Mutation | Run | Result |
|---|---|---|
| `OnStop::KillServed` ends nothing | pane_verbs, profile_verbs, pane_claude | FAILS: 27 `pane_verbs` rows (launch/relaunch: server stop, rollback, same-port relaunch). Pinned. |
| `FakeHarnessesLink::stopped` does nothing | pane_claude | FAILS: 8 rows (close, relaunch, reset, switch, doctor, acceptance scenario). Pinned. |
| `FakeHarnessLink` with `OnStart::Propagate` swallows the start refusal | pane_verbs, profile_verbs, pane_claude, and (separately) pane_integration | **Still GREEN everywhere.** The Propagate mode is **not pinned by any existing row.** |

Per the brief's Test plan step 3, the Propagate mode is reported as untested here and no behaviour test was added (out of scope).
The shared code's `Propagate` arm is logically identical to the code it replaced (`started?` in profile apply's and `bind.rs`'s `LinkedHost::run`),
so the same mutation of the old code would have passed too: this is a pre-existing gap, not one the fold introduced. The Swallow mode of the launch
and claude rigs is pinned by the "server the fake refuses to start" rows (launch's own `HookedLink`) and by `pane_claude`'s rows, which pass unchanged.

## Tier 1 results
| Check | Result |
|---|---|
| `bash scripts/lint.sh` | exit 0; no file over 900; the only warns among touched files are `launch/rig.rs` 834, `apply/rig.rs` 740, `bind.rs` 636, all over 600 at the base too and shorter now. PASS |
| `bash scripts/changelog-check.sh` | `changelog-check: ok`. PASS |
| `cargo clippy -p holler-cli --test pane_verbs --test profile_verbs --test pane_claude --test pane_integration --test profile_apply_scenario -- -D warnings` | clean. PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | fails only on `crates/holler-cli/tests/token_load_test.rs:60:31` ("unneeded cloning of iterator items"), a file this change does not touch (matches F's note; local clippy 0.1.93). Not caused by #846. Advisory |
| `cargo test -p holler-cli --test docs_cli_test --test wire_selftest` | 3 passed, 3 passed. PASS |
| `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0, 167 `test result: ok` lines, no failure (includes the `wizard` skilltext_w5_rename scan with the handoffs present). PASS |
| `cargo machete` | not installed on this machine; not run. No dependency or `Cargo.toml` changed (AC 3), so it cannot be affected. |

## Tier 2 results
- **AC 1, one definition:** `grep -rnE "HostPort for LinkedHost"` prints `verb_harness/linked_host.rs:51` only; `grep -rnE "struct LinkedHost"` prints `linked_host.rs:46` only. PASS.
- **AC 3, no production change:** `git diff --stat origin/main...HEAD` lists only `crates/holler-cli/tests/**` and `docs/handoffs/**`; the diff over `crates/*/src`, `Cargo.toml`, `crates/*/Cargo.toml` is empty. PASS.
- **AC 4, the modes are kept and named:** `OnStart { Propagate, Swallow }` and `OnStop { KillServed, KillNothing }` are documented in the module doc; launch keeps `HookedLink` (hook fired with the started port, refusal swallowed) and `linked: false` maps to `KillNothing`. The kill loop is `OnStop::end`, defined once and called from `FakeHarnessLink::stopped` and `HookedLink::stopped`. Behaviour held by the AC 2 run; Propagate is the untested mode noted above. PASS (with the advisory).
- **AC 5, near-copies:** `grep -rn "opencode-ran" crates/holler-cli/tests` prints one line (`pane_integration/traps.rs:11`); `NO_HUB`/`names_the_missing_hub` are defined once in `traps.rs` and used by `no_hub.rs` and `fake_hub.rs` (the full "{NO_HUB} at {state}" string and the phrase-only check keep their strengths); `herdr_suite_over`, `CallLog`, `unbuildable_lazy_herdr` are in `wrappers.rs`, used by `bind.rs` and `observe.rs` (neither imports the other); `main.rs` lists `observe`. `scratch/`, `out.rs`, `scenario.rs` are untouched. PASS.
- **AC 6, sizes:** `wc -l` before -> after: `launch/rig.rs` 852 -> 834, `apply/rig.rs` 776 -> 740, `bind.rs` 693 -> 636, `pane_claude/rig.rs` 482 -> 453, `observe.rs` 321 -> 269; new files 149 / 74 / 36 lines. None grew; none newly over 600. PASS.
- **Test quality:** no tests added or deleted, so no redundancy introduced. No `#[allow]` added (`git diff -U0` finds none). No secret handled.
- **No-blind-sleeps / flake rules:** nothing timing-related changed.
- **Protocol/golden/doc surfaces:** none touched.
- **Evidence appendix:** `docs/handoffs/846/evidence.md` exists; the tests rely on nothing outside the diff beyond what it lists.

## Acceptance criteria status
1. One definition: PASS (greps above).
2. Unchanged behaviour: PASS (five targets, identical counts).
3. No production code changes: PASS.
4. Differing behaviours kept: PASS; Propagate mode unpinned (pre-existing, see Advisory).
5. Near-copies one each: PASS.
6. Gates: PASS for lint, changelog, the five touched targets' clippy and sizes; workspace clippy fails on the pre-existing `token_load_test.rs` only.

## Blocking issues
None.

## Advisory notes
- `OnStart::Propagate` (profile apply and `pane_integration`'s launch pin) is exercised by no row; a start refusal could be swallowed there and the suite stays green. It was equally unpinned before the fold. A follow-up could add one apply row over a port another pane's server holds.
- `cargo clippy --workspace --all-targets -- -D warnings` fails on `token_load_test.rs:60` under clippy 0.1.93, unrelated to #846. CI on its own toolchain may differ.
- `holler-pane-testkit/src/harnesses.rs:9-12` still says the pre-#831 rigs "keep their own linking"; stale, out of scope (D2). The D2 follow-up issue is still to be filed by O.
- `cargo machete` was not available locally; CI covers it.
