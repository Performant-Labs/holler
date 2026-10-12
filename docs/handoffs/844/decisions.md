
## A (up-front plan review) — 2026-10-12
- **Decided:** BLOCK. The `-u` in `tmux_command` is the right place and shape. The brief must cover `pane_integration/guards.rs`, which pins `LANG=C.UTF-8` (`:109`, `:128`), and must correct "ambient `LANG`" (the rig inherits nothing). Four warns (LC_CTYPE removal, helper rationale, server cleanup, fixture require-vs-skip).
- **Assumed:** tmux picks UTF-8 from `LC_ALL`, then `LC_CTYPE`, then `LANG` (from memory of tmux's client code; not re-verified here).
- **Hedged:** whether finding 1 is a block or a warn; chosen block because an always-run test in AC 3's own command fails as the brief is written.
- **Evidence:** read the brief, `tui.rs`, `fake-tmux`, `fake_tmux.rs`, `tui_test.rs:190-230`, `rig.rs`, `guards.rs:100-128`, `scratch/mod.rs:234-260`, workspace `Cargo.toml` lints, `scripts/lint.sh`; grep for `tmux_command`, `C.UTF-8`.

## A (up-front plan review, second pass) — 2026-10-12
- **Decided:** PASS. The amended brief fixes the first block (`guards.rs` and the scratch doc line are in Files; AC 3 says no locale variable). Three warns: real-tmux test runs by default against the repo's `#[ignore]` convention (say why), `lib.rs:234` also contains `"-u"` (env -u, keep AC 4 scoped to `tui.rs`), CHANGELOG heading is `### Bug Fixes`.
- **Assumed:** tmux's `LC_ALL` > `LC_CTYPE` > `LANG` order is still from memory; the brief's removals cover all three either way.
- **Hedged:** whether the default-run real-tmux test is drift or a justified deviation; chosen warn because the reason (a pin must fail CI) is sound and the test is isolated.
- **Evidence:** read the amended brief, `tui.rs`, `attach.rs:195-225`, `fake-tmux`, `fake_tmux.rs`, `tui_test.rs`, `rig.rs`, `guards.rs:95-135`, `scratch/mod.rs:228-262`, `real_tmux_test.rs` header, `docs/testing.md:447`, CHANGELOG headings; grep for `tmux_command`, `Command::new`, `"-u"`, `LANG|LC_`.

## T (author / RED) — 2026-10-12
- **Decided:** RED valid. `ac11c` fails on the missing `-u`; the new real-tmux test fails with `NoPane` under `LANG=C`; the fixture's require-`-u` fails 12 `attach_test` tests until F adds it. The `guards.rs`/`scratch` LANG removal is done now as test-support edits. The real-tmux test runs by default (skips if tmux is absent) so CI can fail on a regression.
- **Assumed:** the test's three locale cases cover `LANG=C`, `LC_ALL=C` and none; `LC_CTYPE` is removed in each so an ambient value cannot hide the failure.
- **Hedged:** the opt-in #649 scenario was not run (needs real OpenCode); the `-u` patch used to prove GREEN is reachable was reverted.
- **Evidence:** `cargo test -p holler-adapter-opencode --no-fail-fast` failures above; with a temporary `.arg("-u")` the whole package passed; `cargo test -p holler-cli --test pane_integration` 71 passed.

## F (implement) — 2026-10-12
- **Decided:** `.arg("-u")` in `tui::tmux_command`, after the socket flag and before the call's arguments; doc comment and a module-doc "UTF-8 output" rule; CHANGELOG bullet under `### Bug Fixes`; one sentence in ADR-0021 item 1 of "`HarnessPort` as built (#642)". `lib.rs:234` (`env -u`) untouched.
- **Assumed:** `-u` is accepted by every supported tmux (present since tmux 1.x, per the brief's risk list).
- **Hedged:** A's warn 1 (document why the real-tmux test is not `#[ignore]`) left to T/docs; `docs/testing.md` not edited, as it is not in the brief's Files.
- **Evidence:** `cargo test -p holler-adapter-opencode` all green incl. `utf8_locale_test` (ran, not skipped) and `ac11c`; clippy `-D warnings` and `cargo doc` clean; `pane_integration` 71 passed / 4 ignored; `changelog-check` ok; `lint.sh` exit 0.

## T (verify / GREEN) — 2026-10-12
- **Decided:** PASS. Package suite, full workspace suite and `pane_integration` are green; removing `-u` fails `ac11c`. No test needed repair (F flagged none).
- **Assumed:** the workspace clippy error in `token_load_test.rs` is pre-existing because that file is untouched by the diff.
- **Hedged:** `cargo machete` is not installed in this environment, so it was not run.
- **Evidence:** `cargo test --workspace --no-fail-fast` exit 0 (168 ok results); `pane_integration` 71 passed / 4 ignored; `git diff origin/main --name-only` lacks `token_load_test.rs`.
