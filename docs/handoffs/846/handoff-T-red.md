# Handoff-T-red: #846 linkedhost-fakes (characterisation baseline)

**Date:** 2026-10-12
**Branch:** issue-846-implementation (HEAD `90d62fd`, production tree identical to base `6eff9cc`)
**Brief / wireframe reviewed:** docs/handoffs/846-brief.md; no wireframe (no UI)

## A precondition
Confirmed: A returned PASS on the amended brief (`90d62fd`, `docs/handoffs/846/handoff-A.md`).

## Tests authored
None, by design (brief D6, Test plan). #846 is a test-only refactor with no behaviour change, so there is no failing
behaviour assertion to write, and adding a new test would only add redundant tests. The "RED" is (a) the duplication
greps of AC 1 and (b) the per-target baseline that F's fold must reproduce exactly. No test file is added, so no
`[[test]]` entry is needed in `crates/holler-cli/Cargo.toml`.

## RED confirmation

### (a) Duplication check (AC 1) fails today, as the RED requires
`grep -rnE "HostPort for LinkedHost" crates/holler-cli/tests` prints 4 lines (expected after: 1, in `verb_harness/linked_host.rs`):
```
profile_verbs/apply/rig.rs:644, pane_verbs/launch/rig.rs:694, pane_claude/rig.rs:424, pane_integration/bind.rs:524
```
`grep -rnE "struct LinkedHost" crates/holler-cli/tests` prints 4 lines (expected after: 1):
```
pane_claude/rig.rs:420, pane_integration/bind.rs:519, profile_verbs/apply/rig.rs:640, pane_verbs/launch/rig.rs:688
```
`grep -rn "opencode-ran" crates/holler-cli/tests` shows two writers (AC 5 wants one): `no_hub.rs:67` and `wired.rs:36`
(readers `no_hub.rs:104`, `wired.rs:49` are assertions, not writers).

### (b) Baseline for AC 2 (five targets, unchanged tree)
Command: `cargo test -p holler-cli --test pane_verbs --test profile_verbs --test pane_claude --test pane_integration --test profile_apply_scenario`
All five targets built and passed (no warnings of note, exit 0). Per-target results to reproduce after F:

| Target | Result line |
|---|---|
| pane_claude | `test result: ok. 43 passed; 0 failed; 1 ignored` |
| pane_integration | `test result: ok. 71 passed; 0 failed; 4 ignored` |
| pane_verbs | `test result: ok. 347 passed; 0 failed; 0 ignored` |
| profile_apply_scenario | `test result: ok. 5 passed; 0 failed; 3 ignored` |
| profile_verbs | `test result: ok. 118 passed; 0 failed; 0 ignored` |

Total 584 passed, 8 ignored (the opt-in real-tmux/OpenCode scenarios, skipped as today). Note `pane_integration` takes about
150 s wall time on this machine (it spawns real hubs); that is the baseline, not a hang.

### (c) `wc -l` of the rig files at the base (AC 6: none may grow)
| File | Lines |
|---|---|
| crates/holler-cli/tests/pane_verbs/launch/rig.rs | 852 |
| crates/holler-cli/tests/profile_verbs/apply/rig.rs | 776 |
| crates/holler-cli/tests/pane_integration/bind.rs | 693 |
| crates/holler-cli/tests/pane_claude/rig.rs | 482 |
| crates/holler-cli/tests/pane_integration/observe.rs | 321 |

## Ready for F
Confirmed: the characterisation baseline is recorded and green, and the duplication greps fail as the RED (4/4/2).
F may fold against it. Notes for F and the GREEN pass: the AC 2 command must show "five" targets with the counts above;
mutation spot-check (flip propagate to swallow in the shared link) is T's job in Phase 6; per-process `pane_integration`
is slow, so run it once per check.
