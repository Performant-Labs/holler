# Handoff-F: #846 linkedhost-fakes (fold the LinkedHost fakes and the #649 near-copies)

**Date:** 2026-10-12
**Branch:** issue-846-implementation
**Issue:** #846

## What was done
- `crates/holler-cli/tests/verb_harness/linked_host.rs` (new): the one `LinkedHost<H, L>` (`impl HostPort`), the
  `HarnessLink` trait, the two named modes `OnStart { Propagate, Swallow }` and `OnStop { KillServed(&'static [u16]), KillNothing }`,
  and the two ready-made links `FakeHarnessLink<F>` and `FakeHarnessesLink<'a>`.
- `verb_harness/mod.rs`: `pub mod linked_host;`; the "included by" doc line now names the four targets (A warn 2).
- `profile_verbs/apply/rig.rs`: its `LinkedHost` removed; `with_ports` builds the shared one with
  `FakeHarnessLink { on_start: Propagate, on_stop: KillServed(&RIG_PORTS) }`. Module doc points at the shared module.
- `pane_verbs/launch/rig.rs`: its `LinkedHost` removed; a private `HookedLink` (implements `HarnessLink`) keeps the
  server-start hook firing and swallows a start refusal; `linked` becomes `OnStop::KillServed(&RIG_PORTS)` or `KillNothing`.
- `pane_claude/rig.rs`: its `LinkedHost` removed; `with_ports` builds the shared one with `FakeHarnessesLink(&self.harnesses)`.
  One unused import (`PaneError`) dropped.
- `pane_integration/bind.rs`: its `LinkedHost` removed; the launch pin builds
  `Arc::new(LinkedHost { host: FakeHost::new(), link: FakeHarnessLink { harness: Arc::clone(&harness), on_start: Propagate, on_stop: KillNothing } })`.
  `Recording` uses `CallLog`; the conformance run and the unbuildable `LazyHerdr` use the shared helpers.
- `pane_integration/traps.rs` (new): `NO_HUB`, `OPENCODE_MARK`, `names_the_missing_hub(message, state)` and `opencode_trap()` (W-3, W-6).
- `pane_integration/wrappers.rs` (new): `CallLog`, `herdr_suite_over(wrap)` and `unbuildable_lazy_herdr()` (W-7).
- `pane_integration/no_hub.rs`, `wired.rs`: use `opencode_trap()`, `OPENCODE_MARK`, and (no_hub) the shared no-hub text.
- `pane_integration/fake_hub.rs`: the two no-hub assertions build their text from `NO_HUB` (full "`{NO_HUB} at {state}`" string kept at
  the first, phrase-only check kept at the second).
- `pane_integration/observe.rs`: uses `herdr_suite_over`, `CallLog` and `unbuildable_lazy_herdr`.
- `pane_integration/main.rs`: declares `traps` and `wrappers`; the module doc lists `observe` and the two shared modules.

## Design decisions
- **`HarnessLink::started(argv)` takes no pane name.** The brief's sketch had `started(name, argv)`; no link reads the name
  (the fakes key a server by the name inside the argv), so it is left out rather than added as an unused parameter.
- **`OnStop` carries the port list** (`KillServed(&'static [u16])`) instead of a `ports` field plus a `kill_on_stop` bool. This
  answers A's warn 4: `pane_integration`'s "kill nothing" no longer needs a redundant empty `ports`, there are no bare bools,
  and there is no fourth knob.
- **The kill loop lives once, as `OnStop::end(harness, name)`**, not as a free `kill_served` fn. It is called from
  `FakeHarnessLink::stopped` and from the launch rig's `HookedLink::stopped`, the two callers the brief named; the launch
  rig's `linked` flag selects the variant, so there is no second loop.
- **`Borrow` for host and harness** (`H: Borrow<FakeHost>`, `F: Borrow<FakeHarness>`): `&FakeHost`/`&FakeHarness` for the three
  rigs, an owned `FakeHost` and an `Arc<FakeHarness>` for `bind.rs` (the `Arc` is shared with `Resolving`, and the host goes into an
  `Arc<dyn HostPort>`). It did not fight `'static`, so no fallback to `Arc` was needed.
- **`FakeHarnessesLink` has no mode knob**: its only user swallows and ends both fakes' processes; a mode would be unused.
- **`OPENCODE_MARK` const** in `traps.rs`: beyond the brief's "one writer", the two readers also use the const, so
  `grep -rn "opencode-ran" crates/holler-cli/tests` prints one line.
- **`Recording`/`Observing` lost their `note`/`calls` wrappers** and call `self.calls.note(..)` / `inner.calls.list()` directly;
  the asserted values are unchanged.
- **Formatting:** only the new `traps.rs` was run through `rustfmt --edition 2021`; the edited `pane_integration` files are not
  rustfmt-clean at the base (long lines from #649), so they were not reformatted (no unrelated churn). The new
  `linked_host.rs`, `wrappers.rs` and the three rig files pass `rustfmt --check`.
- **No `CHANGELOG.md` line:** previous test-only refactors (#800 `43703ba`, #735 `e9afc8b`) added none; `changelog-check.sh`
  only requires the `## [Unreleased]` heading.

## Reuse / extend-vs-new
Extended `tests/verb_harness/` (the brief's home, D1) with one sibling module; the test kit's `FakeHost`, `FakeHarness` and
`FakeHarnesses` are reused as they are (no new testkit type, D2). `herdr_suite_over` follows `herdr_conformance_test.rs:50-62`'s
`scratch_fake`/`fixture(port)` pattern. The two new `pane_integration` modules are the brief's "new module(s) of that target",
neutral to `bind` and `observe` (A warn 3: both import from `wrappers`, neither from the other).

## Architecture notes for A
Tests only. New modules: `verb_harness::linked_host` (built by `pane_verbs`, `profile_verbs`, `pane_claude`,
`pane_integration`; not by `pane_cli_process`, which includes only `verb_harness/parse.rs`), `pane_integration::{traps, wrappers}`.
Nothing under `pane_integration/scratch/`, `out.rs` or `scenario.rs` changed or imports the new modules. No `#[allow]` added.

## Deviations from spec / wireframe
- `kill_served` is `OnStop::end` (see Design decisions); `on_start`/`kill_on_stop`/`ports` became `on_start: OnStart` and
  `on_stop: OnStop`. The brief allowed refining names.
- The module doc cannot cite a follow-up issue number for D2: none is filed yet (see Known issues).

## Tier 1 self-check (incl. tests now GREEN)
- `cargo clippy -p holler-cli --test pane_verbs --test profile_verbs --test pane_claude --test pane_integration --test profile_apply_scenario -- -D warnings`: clean.
- `cargo clippy -p holler-cli --tests -- -D warnings`: fails only on `tests/token_load_test.rs:60:31` ("unneeded cloning of iterator
  items"), a file this change does not touch (pre-existing with the local clippy 0.1.93).
- AC 2 command, after the fold (identical to T's baseline):
  ```
  pane_claude             test result: ok. 43 passed; 0 failed; 1 ignored
  pane_integration        test result: ok. 71 passed; 0 failed; 4 ignored   (152 s)
  pane_verbs              test result: ok. 347 passed; 0 failed; 0 ignored
  profile_apply_scenario  test result: ok. 5 passed; 0 failed; 3 ignored
  profile_verbs           test result: ok. 118 passed; 0 failed; 0 ignored
  ```
- `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (the F-phase command): exit 0, 167 `test result: ok` lines, no failure.
- AC 1: `grep -rnE "HostPort for LinkedHost" crates/holler-cli/tests` -> `verb_harness/linked_host.rs:51` only;
  `grep -rnE "struct LinkedHost"` -> `verb_harness/linked_host.rs:46` only.
- AC 5: `grep -rn "opencode-ran" crates/holler-cli/tests` -> `pane_integration/traps.rs:11` only.
- AC 3: `git diff` touches only `crates/holler-cli/tests/` (plus this handoff); nothing under `crates/*/src` or any `Cargo.toml`.
- AC 6: `bash scripts/lint.sh` exit 0; no file over 900, none newly over 600. `wc -l` before -> after:
  `launch/rig.rs` 852 -> 834, `apply/rig.rs` 776 -> 740, `bind.rs` 693 -> 636, `pane_claude/rig.rs` 482 -> 453,
  `observe.rs` 321 -> 269; new `linked_host.rs` 149, `wrappers.rs` 74, `traps.rs` 36.

## Evidence appendix
`docs/handoffs/846/evidence.md`

## Tests that look wrong (for T)
None. (No test assertion was changed; the mutation spot-check of AC 4 is T's in Phase 6.)

## Known issues
- The D2 follow-up issue (fold `LinkedHost` into `holler-pane-testkit`) is not filed; `linked_host.rs`'s doc says "folding it into
  the test kit is a follow-up" with no number. O to file it and, if wanted, add the number to the module doc.
- `holler-pane-testkit/src/harnesses.rs:9-12` still says the three pre-#831 rigs "keep their own linking"; now stale, but the
  testkit is out of scope (D2). Worth fixing with the follow-up.
- `token_load_test.rs:60` clippy failure under `--tests` is pre-existing and outside this change.

## Files changed
None in production code (test-only story). Test-support files created or modified:
- crates/holler-cli/tests/verb_harness/linked_host.rs (new)
- crates/holler-cli/tests/verb_harness/mod.rs
- crates/holler-cli/tests/profile_verbs/apply/rig.rs
- crates/holler-cli/tests/pane_verbs/launch/rig.rs
- crates/holler-cli/tests/pane_claude/rig.rs
- crates/holler-cli/tests/pane_integration/bind.rs
- crates/holler-cli/tests/pane_integration/observe.rs
- crates/holler-cli/tests/pane_integration/no_hub.rs
- crates/holler-cli/tests/pane_integration/wired.rs
- crates/holler-cli/tests/pane_integration/fake_hub.rs
- crates/holler-cli/tests/pane_integration/main.rs
- crates/holler-cli/tests/pane_integration/traps.rs (new)
- crates/holler-cli/tests/pane_integration/wrappers.rs (new)
