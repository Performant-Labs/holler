# Brief: #846 linkedhost-fakes

Repo: Performant-Labs/holler. Issue: #846. Rigor: in-session. UI surface: no. Kind: test-only.

## Problem
"test(pane): fold the three LinkedHost fakes and the near-copies in the #649 rigs into one support module." The anti-duplication
gate of #649 (`docs/handoffs/649/handoff-A-dup.md`, W-O) counted three `LinkedHost`s. At `6eff9cc` there are **four** in
`holler-cli`'s tests (the fourth, `pane_claude`, landed with #831 after the gate). Each wraps a `FakeHost` so that `run` starts the
fake harness process the argv describes and `stop_owned` ends it (#802), in near-identical code. Fold them into one shared
module, and fold the cheap near-copies W-3, W-6 and W-7 that sit inside the `pane_integration` target.

## Evidence (verbatim; the outside reviewer cannot read the repo)
The four definitions, at `6eff9cc`. Differences are marked `// DIFF` here for the reader only; they are not in the source.

```
crates/holler-cli/tests/profile_verbs/apply/rig.rs:640-684   (struct LinkedHost<'a> { rig: &'a Rig })
    fn run(..) { self.rig.host.run(name, cwd, argv)?;
                 if let Some(started) = self.rig.harness.start_server(argv) { started?; }   // DIFF: start error PROPAGATES
                 Ok(()) }
    fn stop_owned(..) { self.rig.host.stop_owned(name)?;
                 for port in RIG_PORTS { if self.rig.harness.server(port).is_some_and(|s| s.name == *name) {
                                           self.rig.harness.kill(port)?; } }   // DIFF: kills servers named for the pane
                 Ok(()) }
    ensure_session / ps / foreground_commands: plain delegation to self.rig.host
```
```
crates/holler-cli/tests/pane_integration/bind.rs:517-545   (struct LinkedHost { host: FakeHost, harness: Arc<FakeHarness> })
    fn run(..)  { self.host.run(..)?; if let Some(started) = self.harness.start_server(argv) { started?; } Ok(()) }   // propagates
    fn stop_owned(..) { self.host.stop_owned(name) }                                   // DIFF: kills nothing
    built at bind.rs:630 as Arc::new(LinkedHost { host: FakeHost::new(), harness: Arc::clone(&harness) })  // OWNS the host
```
```
crates/holler-cli/tests/pane_verbs/launch/rig.rs:688-737   (struct LinkedHost<'a> { fakes: &'a Fakes, linked: bool, hooks: &'a Hooks })
    fn run(..)  { self.fakes.host.run(..)?;
                  if let Some(started) = harness.start_server(argv) {
                      let port = started.ok().and_then(|pid| RIG_PORTS.into_iter().find(|&p| harness.server(p).is_some_and(|s| s.pid == pid)));
                      fire(self.fakes, self.hooks, On::ServerStart, Called { pane: None, port }); }   // DIFF: start error SWALLOWED, hook fired
                  Ok(()) }
    fn stop_owned(..) { self.fakes.host.stop_owned(name)?; if self.linked { for port in RIG_PORTS { ..kill(port)?; } } Ok(()) }  // DIFF: only when `linked`
    built at launch/rig.rs:311 (`let host = LinkedHost { .. }`); `linked: false` is the "plain FakeHost" rig (launch/rig.rs:260)
```
```
crates/holler-cli/tests/pane_claude/rig.rs:420-445   (struct LinkedHost<'a> { rig: &'a Rig }); built at rig.rs:198
    fn run(..)  { self.rig.host.run(..)?; let _ = self.rig.harnesses.start(argv); Ok(()) }   // DIFF: FakeHarnesses (OpenCode + Claude router); error SWALLOWED
    fn stop_owned(..) { self.rig.host.stop_owned(name)?; self.rig.harnesses.stop(name); Ok(()) }
```
The shared skeleton, identical in all four: `ensure_session`, `ps` and `foreground_commands` delegate to the `FakeHost`;
`run` is "host runs, then start what the argv describes"; `stop_owned` is "host stops, then end what ran for the pane".

Where the shared code can live. Every one of the four targets already includes `tests/verb_harness/mod.rs` by `#[path]`:
```
crates/holler-cli/tests/pane_claude/main.rs:19       #[path = "../verb_harness/mod.rs"] mod verb_harness;
crates/holler-cli/tests/pane_verbs/main.rs:17        (same)
crates/holler-cli/tests/profile_verbs/main.rs:13     (same)
crates/holler-cli/tests/pane_integration/main.rs:30  (same)
crates/holler-cli/tests/verb_harness/mod.rs:14-21    use clap::Parser; use holler_cli::...; use holler_pane::Ports; ...  pub mod parse;
```
`pane_cli_process` (`tests/pane_verbs/process/main.rs`) builds only `verb_harness/parse.rs` by its own `#[path]`, so a new sibling
module is not built there. The test kit already provides the fakes (`holler_pane_testkit::{host::FakeHost, harness::FakeHarness,
harnesses::FakeHarnesses}`); `holler-cli` has it as a dev-dependency (`Cargo.toml:465`).

The near-copies in scope (W numbers are the #649 gate's):
```
W-3  pane_integration/wired.rs:28-41   fn trap_dir()  -- writes an executable `opencode` script that touches `opencode-ran`, in a /tmp "hlr649-" dir
     pane_integration/no_hub.rs:50-80  struct Traps   -- the same OpenCode trap (prefix "hlr649-", dir in /tmp), plus a herdr UnixListener
W-3b pane_integration/scratch/mod.rs  Scratch::harness resolves with `rig::sessions_for(&[])` (an empty-map resolver spelled inline)
W-6  the no-hub text ("names the missing hub and its state dir") is asserted three ways: no_hub.rs:121-136 (`names_the_missing_hub`),
     fake_hub.rs:340 and :415 (D3 / watch "unavailable naming the missing hub"), and scenario.rs
W-7  pane_integration/observe.rs:27-55 repeats bind.rs:293-316 (the herdr conformance run over a wrapper); `Observing`'s call log (:62-81)
     repeats `Recording`'s (bind.rs:164-178); the unbuildable-`LazyHerdr` closure (:198-205) repeats bind.rs:135-142; main.rs's module doc omits `observe`
```
Not in scope (see Out of scope): W-1 (`alive`, `scratch/mod.rs:89`, third copy of `holler-adapter-host/tests/common/mod.rs:224`),
W-2 (`real_tmux::tmux_on_path`, `holler-adapter-host/tests/real_tmux/mod.rs:130`), W-4 (`Feed<E>`, production code).

Size pressure: `lint.sh` warns at 600 lines and fails at 900 (`scripts/lint.sh:43-50`). Today `pane_verbs/launch/rig.rs` is 852,
`profile_verbs/apply/rig.rs` 776, `pane_integration/bind.rs` 693, `pane_claude/rig.rs` 482. The fold must not grow any of them.

## Acceptance criteria
1. **One definition.** `grep -rn "impl HostPort for" crates/holler-cli/tests` prints exactly one line, in the new module; and
   `grep -rnE "struct LinkedHost" crates/holler-cli/tests` prints exactly one line (the same file). (Today: four of each.)
2. **Unchanged behaviour, proved by characterisation.** The four targets, run before the change and after it with the same command,
   pass with the same test count: `cargo test -p holler-cli --test pane_verbs --test profile_verbs --test pane_claude --test pane_integration`
   (the opt-in scenario stays skipped, as today). T records the per-target `test result: ok. N passed` lines before (at the base) and after.
3. **No production code changes.** `git diff --stat origin/main...HEAD` lists only paths under `crates/holler-cli/tests/`,
   `docs/handoffs/`, and `CHANGELOG.md` if the repo's convention asks for it for test-only work (F checks the previous test-only
   PRs, e.g. #849). `git diff origin/main...HEAD -- 'crates/*/src' Cargo.toml 'crates/*/Cargo.toml'` is empty.
4. **The two behaviours that differ are kept, not unified by accident.** Profile apply and `pane_integration` still propagate a
   start error from `run`; `pane_verbs` launch and `pane_claude` still swallow it. `pane_verbs` launch still fires its
   `On::ServerStart` hook with the started port, and `linked: false` still kills nothing. Observable: the existing launch hook rows and the
   "server the fake refuses to start" rows pass unchanged (AC 2), and the new module's own doc states the two modes by name.
5. **The near-copies in scope are one each.** `trap_dir` and `Traps` share one trap builder (`grep -rn "opencode-ran" crates/holler-cli/tests`
   shows one writer); `observe.rs` has one generic suite runner, one `CallLog` and one unbuildable-`LazyHerdr` builder used by both it and `bind.rs`;
   `sessions_for(&[])` appears once in `pane_integration`; the no-hub assertion helper is defined once and used by `no_hub.rs`,
   `fake_hub.rs` and `scenario.rs`; `main.rs`'s module doc lists `observe`.
6. **Gates.** `cargo clippy -p holler-cli --tests -- -D warnings` is clean; `bash scripts/lint.sh` shows no file over 900 lines and none
   newly over 600; `cargo test -p holler-cli` passes (the CI workspace test with its one skipped timing test is the F-phase command).
   No `cargo fmt` over the workspace: format only the touched files with `rustfmt --edition 2021 <file>` if at all.

## Files
Production: none.

New: `crates/holler-cli/tests/verb_harness/linked_host.rs`, declared `pub mod linked_host;` in `tests/verb_harness/mod.rs`. Chosen
because all four targets already include `verb_harness` and `pane_cli_process` does not build it. Shape (a sketch, F may refine names):
- `pub struct LinkedHost<H: Borrow<FakeHost>, L: HarnessLink> { pub host: H, pub link: L }` with the one `impl HostPort`. `Borrow` serves
  `&FakeHost` (the three rigs), and `Arc<FakeHost>` or an owned `FakeHost` (`pane_integration`'s `Arc::new(LinkedHost { host: FakeHost::new(), .. })`).
- `pub trait HarnessLink { fn started(&self, name: &PaneName, argv: &Argv) -> Result<(), PaneError>; fn stopped(&self, name: &PaneName) -> Result<(), PaneError>; }`
  `LinkedHost::run` = `host.run(..)?; link.started(..)`; `stop_owned` = `host.stop_owned(..)?; link.stopped(..)`.
- Two ready-made links: `FakeHarnessLink { harness, ports: &[u16], on_start: PropagateErrors | SwallowErrors, kill_on_stop: bool }` (profile apply and
  `pane_integration`: propagate; `pane_integration` kills nothing) and `FakeHarnessesLink(&FakeHarnesses)` (`pane_claude`: `start`, swallowed; `stop`).
- `pane_verbs` launch keeps a ~15-line private `HarnessLink` impl (its hook firing needs `Fakes` and `Hooks`, which are rig-local), reached
  through the shared `LinkedHost`. Its `linked` flag becomes `kill_on_stop` of the same kill loop, not a second loop.
- The shared "kill every server of `RIG_PORTS` named for the pane" loop lives once (profile apply and launch each had it).

Edited (tests only): `profile_verbs/apply/rig.rs` (drop `LinkedHost`, `:640-684`; build site `:224`), `pane_verbs/launch/rig.rs` (`:688-737`; build site `:311`),
`pane_claude/rig.rs` (`:420-445`; build site `:198`), `pane_integration/bind.rs` (`:517-545`; build site `:630`), plus for the near-copies:
`pane_integration/{wired.rs, no_hub.rs, fake_hub.rs, scenario.rs, observe.rs, main.rs, scratch/mod.rs}`. The pane_integration helpers
(trap builder, `CallLog`, suite runner, no-hub assertion) go in a new `pane_integration/common.rs` (declared in its `main.rs`), not in
`verb_harness`, because they are one target's glue.

Reuse map (extend, do not duplicate): `verb_harness/mod.rs` as the home; `holler_pane_testkit` fakes (no new fake type in the testkit);
`herdr_conformance_test.rs:50-62`'s `scratch_fake`/`fixture(port)` pattern for the suite runner.

Blast radius: about 12 files, all under `crates/holler-cli/tests/`, one new file in `verb_harness/` and one in `pane_integration/`.

## Decisions already made (MO)
- D1 Home is `tests/verb_harness/linked_host.rs`, not `tests/support/` (that one is for the real-binary hub rigs and is included only by `pane_integration`).
- D2 No testkit change. A `LinkedHost` in `holler-pane-testkit` would be the cleaner home long term but is a production-crate (non-test) edit
  with a public API; out of scope. The module doc says so.
- D3 Fold four, not three; the issue's "three" predates #831.
- D4 The propagate/swallow difference is real behaviour of the existing tests and is kept as a named mode (AC 4); unifying it is a behaviour change.
- D5 Scope of near-copies: W-3, W-6, W-7 only (all inside `pane_integration`, same author, same target). W-1, W-2 (cross-crate test includes)
  and W-4 (production) are not folded here.
- D6 Characterisation, not RED/GREEN: see Test plan.

## Out of scope
- Any production code, any `Cargo.toml`, the testkit crate, and the adapter crates' test directories (W-1, W-2).
- W-4 (`Feed<E>` iterator): production client code; file separately if wanted.
- The `start_server` helper in `pane_verbs/doctor/rig.rs:410` and the direct `start_server` calls at `launch/rig.rs:382`, `:483` and
  `profile_verbs/rig.rs:252`: not `LinkedHost`s; leave unless F's fold makes one trivially redundant.
- Splitting the big rig files for size; the fold only has to shrink them.
- Changing what any test asserts.

## Test plan
This is a refactor with no behaviour change, so there is no failing assertion to write. T's RED step is a **characterisation**:
1. **Baseline (before F).** On the unchanged tree, run the four targets (AC 2 command) and record each `test result:` line and test count.
   Also record the duplication as the failing check: the two greps of AC 1 print 4 lines each today (that is the "RED"; no new test file is added).
2. **GREEN (after F).** The same four runs pass with identical counts; the greps print one line each; AC 3's empty production diff; AC 6 clippy and lint.
3. **Mutation spot-check (T, cheap).** In the new module, flip one mode (e.g. make `FakeHarnessLink` swallow where profile apply propagates):
   at least one profile-apply row that depends on a start error must fail. If none fails, T reports the mode as untested and says so in its handoff
   (do not add a behaviour test to fix it here).
Order: O brief, A (up-front, rigor in-session), T baseline, F, T verify, A (anti-duplication), S. No D or U.

## Risks
- **Hidden behaviour difference.** `start error propagates` (profile apply, `pane_integration`) versus `swallowed` (launch, claude) is the
  one place a naive fold changes behaviour. Mitigated by AC 4 and the mutation check.
- **Lifetimes.** `pane_integration` owns the host in an `Arc<dyn HostPort>`; the others borrow it. The `Borrow` generic is the planned answer;
  if it fights `Arc<dyn HostPort>` ('static) F may take `Arc<FakeHost>` there.
- **Dead-code and `unused` warnings.** Each target includes `verb_harness` whole, so the unused links warn in targets that do not use them.
  F checks `scripts/lint.sh`; the module must not add an un-linked `#[allow]` (house rule: an allow carries an issue link, here `// #846`).
- **Sibling clashes.** See below.

## Clashes (sibling worktrees at `6eff9cc`; none has a commit yet, so this is from the issues and the files they name)
| Sibling | Likely files | Overlap with #846 | Handling |
|---|---|---|---|
| #844 OpenCode adapter: the TUI query reports no tmux session without a UTF-8 locale | `crates/holler-adapter-opencode/src/*`, its tests, possibly `holler-cli/tests/pane_integration/scratch/*` (the real-OpenCode scenario) | Low. Only if #844 edits the scratch module or `scenario.rs`, which #846 touches for W-3b/W-6 | Land in either order; the conflict, if any, is a trivial hunk in `scratch/mod.rs`. #846 rebases. |
| #845 switch after relaunch (times out until the TUI accepts select-session) | may touch the `pane_integration` rig or scenario (`scenario.rs`, `scratch/verbs.rs`, `bind.rs`) and `pane_verbs/{relaunch,switch}` | **Medium.** `bind.rs` and `scenario.rs` are both edited by #846 | Whichever merges second rebases. If #845 adds a case to `pane_integration/bind.rs` that builds a `LinkedHost`, #846 converts it at rebase (one line: `LinkedHost { host, link }`). Ask O to merge #845 first if both are ready. |
| #847 remote_admin_test flake | `tests/remote_admin_test.rs`, `tests/support/remote_admin_rig.rs`, possibly `support/mod.rs` | None: #846 does not touch `tests/support/` or the root test files | Independent. |
| #848 skilltext_w5_rename (wizard rename scan skips docs/handoffs/) | the wizard rename scan test (`docs_cli_test` pattern) | **Indirect, and positive.** #846 commits `docs/handoffs/846-brief.md` and later handoffs, which the scan walks; if the scan trips on handoff text, #846's CI fails until #848 lands | O: merge #848 before #846's PR, or confirm the scan passes with the brief present. T runs the scan test once. |
| #696 one bounded subprocess runner (needs #641, #642, #663 merged) | adapter `src/` and `holler-pane` `run_probe`; adapters' fake-`kill` seam tests | None on files. Its "adapters' existing tests keep passing, including the fake-`kill` seam" is unaffected: #846 never touches adapter crates | Independent. |
CHANGELOG.md, if the repo asks for a line, is the usual one-hunk conflict with every sibling; keep all entries.

## Open questions for the operator
1. Fold the cross-crate near-copies W-1 (`alive`) and W-2 (`tmux_on_path`) too? The brief says no (D5); they need edits under `holler-adapter-host/tests/`.
2. Does a test-only change owe a `CHANGELOG.md` line in this repo? F checks the last test-only PR; the brief assumes the existing convention.
