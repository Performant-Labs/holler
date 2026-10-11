# Handoff-T-green: Phase 7 - #649 hub wiring and the scratch integration scenario  (third cycle: test-only rework after the diff gate's round-2 findings)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `dfb395c` plus this phase's edits)
**Issue:** #649
**Handoff-F reviewed:** `docs/handoffs/649/handoff-F.md` (second cycle; F has not run since)
**Handoff-T-red:** `docs/handoffs/649/handoff-T-red.md` (second cycle)

This handoff replaces the second cycle's (`dfb395c`), which git history keeps. That cycle's spot-checks still hold at
this tree: the W-T row against the unset read and M8, and the 6 s pin against a 5 s bound. So do the first cycle's
M1 to M7, and the acceptance table below. **No production file has changed since `22aa539`.** This phase changes
one test comment and `evidence.md`.

## The round-2 findings

Both findings say the code may not compile. The reviewer could not see enough source to tell. A build settles both,
and both are refuted.

| Finding | What it claims | What the build shows | Kind |
|---|---|---|---|
| `tests/pane_integration/bind.rs:466-510`, the D8 pin test | The test may not compile. The reviewer saw neither `run_verb_with`'s signature nor `FakeProber`, so it could not check the `Ports` bounds or `Prober: Send + Sync`. | `cargo build -p holler-cli --all-targets` is clean. `cargo test -p holler-cli --test pane_integration bind::launch_resolves_attach_tuis_pane_from_the_bindings_before_the_record_exists` gives `ok. 1 passed`. | Test-only: the reviewer lacked source. Fixed below. |
| `src/pane/wiring.rs:100-107`, `from_env`'s `LazyHerdr` closure | `move \|\| connect_herdr(&herdr_config)` borrows a local, so the closure is not `'static` and will not compile. | It compiles, with clippy `-D warnings` clean. `move` moves the owned `Result<HerdrConfig, PaneError>` into the closure. `&herdr_config` then borrows the closure's own copy, so the closure is `'static`. | Production code, so it is F's to answer. **No production change is needed.** I made none. |

## Test repairs this phase

- **`evidence.md`, new section "The source that round 2 of the diff gate asked for (T, third cycle)".** It has four
  entries, each copied verbatim from unchanged source and checked line by line with a script:
  - `run_verb_with(argv, format, ports: Ports<'_>)` (`tests/verb_harness/mod.rs:54`);
  - the `Ports` struct, seven `&dyn` port references (`holler-pane/src/ports.rs:356-367`);
  - `Prober: Send + Sync` and `ProfileScope: Send + Sync`, and the test kit's `impl Prober for FakeProber` and
    `impl ProfileScope for FakeProfileScope`;
  - the constructors `FakeProber::new()` and `FakeProfileScope::new(Arc<dyn ProfileStore>, Arc<dyn PaneStore>, Actor)`,
    and both fake stores' trait impls.
- **`bind.rs`, the D8 test:** two comment lines before the scope. They say that `Ports` takes all seven ports, and that
  the profile store, the scope and the prober only fill it, because the test asserts on the resolver's answer and the
  pane store. This answers the finding's observation that the test "never asserts on the profile store". The test's
  behaviour is unchanged. The file is 579 lines.

## GREEN confirmation

| Run | Result |
|---|---|
| `cargo test -p holler-cli --test pane_integration bind::launch_resolves_…` | `ok. 1 passed` |
| `cargo test -p holler-cli --test pane_integration` (after this phase's edits) | `ok. 61 passed; 0 failed; 4 ignored; finished in 119.04s` |
| Opt-in run (`HOLLER_TEST_PANE_INTEGRATION=1 … -- --ignored --test-threads=1`) | Not re-run. No production or harness code has changed since the second cycle's 4/4 at `22aa539`, and this phase adds only a comment. |

## Tier 1 results

| Check | Command | Actual | |
|---|---|---|---|
| Build | `cargo build -p holler-cli --all-targets` | clean | PASS |
| Clippy (CI flags) | `cargo clippy --workspace --all-targets -- -D warnings` | clean | PASS |
| Workspace tests (CI's `test.unit.command`) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | 151 binaries: 2312 passed, 1 failed, 32 ignored. The figures are the same as F's and the second cycle's. The failure is `wizard_scripts`' rename scan. | PASS for this story |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed | PASS |
| Lint | `bash scripts/lint.sh` | exit 0. The only warning is `holler-pane/src/error.rs is 713 lines`, a file this story does not touch. | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | PASS |
| Unused deps | `cargo machete` | none | PASS |

The rename scan's hits are pipeline scratch only. `git grep -l agent-skills/setup-wizard -- ':!docs/handoffs'` finds
nothing, and `docs/handoffs/` is removed before the push.

**Cross-check against F's handoff:** 61/0/4, 2312/1/32, 3/3, 3/3, and clean clippy, lint, changelog and machete all
reproduce. There is no discrepancy.

## Tier 2 results

| Check | Method | |
|---|---|---|
| Coverage per criterion | Unchanged from the second cycle. Every criterion keeps its test (table below). | PASS |
| Test quality | The D8 test fails when its behaviour is removed: in the first cycle's M6, `BindingHerdr` offers no pane id and the test fails. The new comment names what the test asserts on. Nothing redundant was added. | PASS |
| Type safety | The build and clippy `-D warnings` are clean. Both "will not compile" claims are refuted by building. | PASS |
| Evidence appendix | Every unchanged-source fact the D8 test relies on now has a verbatim entry. Each line was checked against its file by script, and one range was corrected from 356-368 to 356-367 before saving. | PASS |
| Error handling, data integrity, API contract, security | No change this phase. The second cycle's results stand. | PASS |
| Migration safety, Playwright | N/A: no schema, and no browser surface. | N/A |

## Acceptance criteria status

The criteria all PASS, backed by the same tests as the second cycle's table (`dfb395c`):
- `no_hub::*`, `store_only::*` and `env::malformed_adapter_settings_…`;
- `fake_hub::*` (timeout, unavailable, the lost write, the 6 s pin);
- `wired::*` (the relative socket and the non-UTF-8 socket), `guards::*` and `watch::*`;
- `bind::launch_resolves_attach_tuis_pane_…` (`attach_tui` resolves before the record exists);
- the opt-in `scenario::*` (the scenario, the profile scenario, the grid, and the server argv under tmux);
- the D28 sentinel checks.

## Blocking issues

None. F's production code does not need to change for either round-2 finding. In F's own pass, the `wiring.rs` finding
needs only an answer, for example a line in `handoff-F.md` saying the closure is `move` and the build is clean. A code
change is not needed.

## Advisory notes

- The second cycle's advisories stand:
  - a non-UTF-8 `HOLLER_HERDR_WORKSPACES` reads as no workspace;
  - the `support/` tree is not bare-`rustfmt` clean;
  - the rename scan fails while `docs/handoffs/` is tracked.
- Both round-2 findings came from source the reviewer could not see. If F wants to head off a repeat on the `wiring.rs`
  closure, an `evidence.md` entry is the cheaper route. `LazyHerdr::new`'s signature is in the diff (`bind.rs`), so
  it needs no excerpt.

`T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.`
