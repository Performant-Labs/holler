# Handoff-T-green: #851 - `holler --version` prints the commit the binary was built from

**Date:** 2026-10-11
**Branch:** issue-851-implementation (HEAD `bf26366`, a clean tree when the checks ran)
**Issue:** #851
**Handoff-F reviewed:** docs/handoffs/851/handoff-F.md
**Handoff-T-red:** docs/handoffs/851/handoff-T-red.md

## GREEN confirmation

```
$ cargo test -p holler-cli --test build_version_test --test cli_invocation_test --test query_test --test pane_cli_process
build_version_test   test result: ok. 8 passed; 0 failed
cli_invocation_test  test result: ok. 14 passed; 0 failed
pane_cli_process     test result: ok. 36 passed; 0 failed
query_test           test result: ok. 29 passed; 0 failed

$ ./target/debug/holler --version ; ./target/debug/holler -V ; git rev-parse --short=7 HEAD
holler 0.4.0 (bf26366)
holler 0.4.0 (bf26366)
bf26366
```

On a clean tree the binary names HEAD with no `-dirty`. The build script re-ran after F's commit, so the hash is
the new HEAD and not F's mid-phase `5e27fa7-dirty`.

**Spot-check: do the tests fail when the behaviour is removed?** For each case I changed one part of F's code,
ran the target, and restored the file with `git checkout`. `git status` was clean afterwards.

| Mutation | Test that failed | Failure |
|---|---|---|
| M1: delete `build_info::install(...)` in `main` | `query_test::status_docs_carry_version` | `:103` body query status: `left: Some("0.4.0")`, `right: Some("0.4.0 (bf26366-dirty)")` |
| M2: `control_status.rs` reads `env!("CARGO_PKG_VERSION")` again | same | `:117` live `hub status --json`: same left/right |
| M3: `commit_token` ignores `dirty` | `build_version_test::a_dirty_checkout_appends_dirty` | `:49` |
| M4: dirty check counts untracked files | `build_version_test::git_state_of_a_clean_checkout_is_the_short_head_hash` | `:120` |
| M5: clap attribute goes back to a bare `version` | `cli_invocation_test::version_flag_prints_version_and_commit` | `:54` |

Each test fails on its own behaviour. The mutations also showed the end-to-end dirty path: each one edited a
tracked file under a watched `src/`, the next build re-ran the script, and the binary printed `bf26366-dirty`.
The clean rebuild after the restore printed `bf26366` again.

**End-to-end builds with no `.git`.** I extracted `git archive HEAD` into the session scratchpad, outside any
repository, and built `holler` there:

| Build | `--version` |
|---|---|
| no `HOLLER_GIT_SHA` | `holler 0.4.0 (unknown)` |
| `HOLLER_GIT_SHA=abc1234` | `holler 0.4.0 (abc1234)` |
| `HOLLER_GIT_SHA=" "` (blank) | `holler 0.4.0 (unknown)` |

This confirms on the real binary what the unit tests pin as pure functions, including `rerun-if-env-changed`. I
deleted the scratch tree afterwards.

## Tier 1 results

`.github/workflows/ci.yml` and `scripts/lint.sh` define what CI runs. Each command below ran at `bf26366`.

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0; only the warnings that were already there for files over 600 lines | PASS |
| Changelog | `bash scripts/changelog-check.sh` | exit 0 | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| Verify command | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0 | exit 0: 153 result lines, 2382 passed, 0 failed, 32 ignored | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |

`cargo test -p holler-proto` is included in the workspace run.

**Cross-check against handoff-F:** my counts match F's: 8/14/36/29 on T's targets, and 2382 passed, 0 failed,
32 ignored on the verify command. There is no discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage of each AC | See the table below. Each AC has a test. | PASS |
| Test quality (test-quality.md §7) | Each test names one behaviour, and M1 to M5 show each fails for the right reason. Tiers: pure functions and a temp repo for the resolver, the process level only where the producer runs in the binary. No duplication: `pane_cli_process` checks only the two leading tokens and leaves the exact format to `cli_invocation_test`. The suite fits a small story: 8 unit tests plus 3 changed process tests. | PASS |
| Type safety | Clippy with `-D warnings` is clean. The new code has no `unsafe` and no casts. | PASS |
| Error paths | Pinned by tests: no checkout gives `None`, and a blank `HOLLER_GIT_SHA` counts as unset. Checked by reading the code: when `git status` fails after `rev-parse` succeeded, the tree counts as dirty, so the build never claims to be clean. | PASS |
| Data integrity | Not applicable: no store or schema change. | N/A |
| API contract / wire | No golden file is in the branch diff (`git diff --name-only bda1069 HEAD`). The `version` key and its type are unchanged and no field is added. Only the value gains ` (COMMIT)`. `PROTOCOL_VERSION` is unchanged. | PASS |
| Docs | `docs_cli_test` passes. ADR 0003 and v2.md §10 match. | PASS |
| Security / secrets | The build script embeds only a one-word token and no path, so `check-binary-paths.sh` still holds for release builds. The tests print no secrets. | PASS |
| File size and allows | `build_version_test.rs` has 176 lines, `query_test.rs` 585 and `cli_invocation_test.rs` 423, all under 900. The new `#![allow]` carries `// #851`. | PASS |
| `[[test]]` registration | `build_version_test` is declared in `crates/holler-cli/Cargo.toml`. | PASS |
| Migration safety | Not applicable. | N/A |
| Playwright / UI | Not applicable: no UI surface. | N/A |

**Evidence appendix:** I added two T entries to `evidence.md`. `body caps` and `body query status` run through
`holler_body::query::local_status` in the CLI process (`body_cmd.rs:269-273`), and `local_caps` embeds that same
status (`holler-body/src/query.rs:86-87`). `query_test` relies on both facts, and F's entries did not include
them.

## Acceptance criteria status

| Criterion | Status | Backing test |
|---|---|---|
| AC 1a: a build from a git checkout prints HEAD's 7-character hash | PASS | `cli_invocation_test::version_flag_prints_version_and_commit` (real binary vs `git rev-parse --short=7 HEAD`); `build_version_test::git_state_of_a_clean_checkout_is_the_short_head_hash`, `a_checkout_uses_the_head_hash`, `git_state_of_a_linked_worktree_is_its_own_head` |
| AC 1b: `HOLLER_GIT_SHA=abc1234` with no `.git` prints `abc1234` | PASS | `build_version_test::no_checkout_reads_holler_git_sha`, plus the `git archive` build above |
| AC 1c: neither prints `unknown` | PASS | `build_version_test::neither_is_unknown`, `git_state_outside_a_checkout_is_none`, plus the `git archive` build above |
| AC 2: a dirty tree ends `-dirty` (inside the parentheses, as T's RED contract specifies) | PASS | `build_version_test::a_dirty_checkout_appends_dirty`, `git_state_of_a_changed_tracked_file_is_dirty`; seen end to end in M1 to M5 |
| AC 3: no other output changes and the golden files are unchanged | PASS | the workspace suite including the `holler-proto` golden tests; no golden file in the diff; `pane_cli_process` checks the first two tokens |
| Work 3: the same string wherever the version is shown | PASS | `query_test::status_docs_carry_version`: body query status, body caps, hub query status, `hub status --json`, and the human `hub status` line |
| Work 4: ADR 0003 row and CHANGELOG | PASS | `changelog-check` and `docs_cli_test` pass, and the ADR 0003 and v2.md rows are updated (F) |

## Blocking issues

None.

## Advisory notes

- Cargo's re-run triggers (git files, `src/` paths) have no automated test, as F records. I confirmed two
  triggers by hand (above). F's fixture table covers the rest. An automated test would need a fixture workspace
  that runs `cargo build` from inside a test. That costs more than it is worth for a display string.
- I did not run CI's Windows leg locally. `version_flag_prints_version_and_commit` accepts `-dirty`, so a
  Windows checkout that git reports as changed (line endings) still passes. A Windows build with no git on
  `PATH` also passes, through the test's `None` branch.
- The dirty flag can be stale after edits to files outside the watched paths. F records this in
  `docs/releasing.md`.

T-green complete, no blocking issues. No UI surface, so U does not apply. Ready for S.
