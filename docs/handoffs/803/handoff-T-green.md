# Handoff-T-green: Phase 7 - #803, the setup wizard builds panes through `holler pane` and `holler profile apply`

**Date:** 2026-10-12
**Branch:** `issue-803-implementation` at `18cf637` (F's commit), plus T's uncommitted test repair and evidence entries
**Issue:** #803
**Handoff-F reviewed:** `docs/handoffs/803/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/803/handoff-T-red.md`

## GREEN confirmation

```
$ cargo test -p holler-cli --test wizard_scripts
test result: ok. 303 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The RED was 231 passed and 72 failed. All 72 now pass, and so do my two repaired tests (below).

**Test repairs (F's "Tests that look wrong", items 1 to 3).** All three findings were real, and the fixes are in `skilltext_803.rs` only. No production file was touched.

1. **`no_direct_herdr_pane_control_and_no_remote_session_in_the_skill_docs_or_readme`.**
   - **The problem.** The scan ran on raw text with plain substrings. A banned call wrapped across a line, such as `herdr.sh run` followed by `pane split` on the next line, escaped it. Ordinary prose such as "pane runs" or "pane reads" tripped it.
   - **The fix.** The ban now scans `flat()` text and matches whole words through a new `contains_phrase`: the character after the match is not alphanumeric. The `herdr.sh run ` allow-list now exempts an occurrence directly after "never `". That is the one sanctioned mention, `skilltext.rs`'s stop-path pin "never `herdr.sh run server stop`".
   - **The result.** The two pins no longer depend on where a line breaks.
2. **`the_adr_carries_the_wizard_sentence_the_decision_and_item_16_as_built`.**
   - **The problem.** The decision pin ended in a closing backtick after `holler profile apply`, so the ADR could satisfy it only with the bare form. On one line, that form fails `docs_cli_test`, because `<NAME>` is required.
   - **The fix.** The pin now stops before the backtick, so `holler profile apply NAME` satisfies it too. The ADR text F wrote still passes as it is.

**Spot-checks: the tests fail when the behaviour is removed.** I mutated the tree, ran the named tests, then restored the tree with `git checkout`.

| Mutation | Result |
|---|---|
| `SKILL.md` gains a wrapped `` `herdr.sh run`` / ``pane split` `` | `no_direct_herdr_pane_control...` FAILS (it passed this text before the repair) |
| `SKILL.md` gains "The pane runs OpenCode." | the same test passes, so the false positive is gone |
| `herdr.sh`'s `holler_guard` call is disabled | 3 `herdr_803` refusal tests FAIL |
| `base_env` keeps `ANTHROPIC_API_KEY` | `holler_runs_with_the_instances_literals...` FAILS |
| `run`'s allow-list admits `pane split` | `run_refuses_every_pane_and_layout_word...` FAILS |
| `panes.sh teardown` closes panes in forward order | `teardown_closes_every_pane_in_reverse_layout_order...` FAILS |

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| CI workspace test (`test.unit.command`) | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | all ok | 167 binaries, 2624 passed, 0 failed; exit 0 (matches F) | PASS |
| Wizard suite after the repair | `cargo test -p holler-cli --test wizard_scripts` | all ok | 303 passed, 0 failed | PASS |
| Docs doc-test | `cargo test -p holler-cli --test docs_cli_test` | ok | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | ok | 3 passed | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 (after the repair) | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0; 600-line warnings only, every changed test file under 900 (`skilltext_803.rs` 544) | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| Shell | `bash -n panes.sh herdr.sh`; `shellcheck -s bash panes.sh` | clean | clean | PASS |
| Format | `rustfmt --check` on the file T changed | clean | clean (formatted). CI does not run `cargo fmt --check`, and the repo has existing drift elsewhere | PASS |

F's reported results all reproduce. I did not repeat F's mawk and busybox awk runs.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage | Every Acceptance item 1 to 24 and every acted-on warn (W1 to W8, W12) has a test (the handoff-T-red table) | PASS |
| Test quality (§7) | Each test names a behaviour, and the spot-checks above show the key ones fail in isolation. Script tests sit at the script tier and text pins at the text tier. The split into `panes_803` and `panes_803_plan` duplicates no test. The two brittle pins are repaired. | PASS |
| Type safety | Test-only Rust change, and clippy is clean. `Option::is_none_or` is stable (rustc 1.98.1) | PASS |
| Error handling | Covered for `panes.sh` (every validity rule exits 1 naming the key; usage exits 2), the door's refusals (exit 2, `holler` never run) and `run`'s refusals (exit 2, no Herdr call) | PASS |
| Data integrity | The proof scenarios run against a state dir: instance B touches only its own state, A's snapshot is unchanged, a rerun converges, and the teardown leaves an empty registry and no profile | PASS |
| API contract | The plan lines match the CLI's flags (`--spec-only`, `--harness`, `--port-policy none`, `--herdr-session`, `--format=json`). The fake models the real apply envelope and exit 3. | PASS |
| Security | The scrub test sets 11 sentinels, including credential-shaped ones. None reaches `holler`, no `HERDR_*` passes, and no sentinel value is printed. | PASS |
| Migration safety | N/A: no schema. The retired config shape is refused by name (`[[orchestrator]]`, `[[session]]`, attach keys), not misread. | PASS |
| Protocol / goldens | N/A: there is no wire change and no Rust production change | PASS |
| Evidence appendix | F's 14 entries cover the CLI contracts the skill reads. I appended 4 for facts the fake `holler` models from unchanged code: apply's JSON step shape, apply's exit 3 with no steps, the `grid-unreachable` rule for a new row below a row of two panes, and `--herdr-session` agreeing with `HOLLER_HERDR_SESSION` | PASS |
| Playwright / browser | N/A (no UI) | N/A |

## Acceptance criteria status

Every criterion passes, backed by the test named in `handoff-T-red.md`'s table:

- **1 and 3 to 7:** `panes_803`.
- **2:** `panes_803_plan` and `proof_803`.
- **8:** the `panes_803_plan` env tests.
- **9:** `herdr_803`.
- **10:** the repaired `skilltext_803::no_direct...`.
- **11 to 24:** the `skilltext_803` stage, ADR, docs and README pins, plus `proof_803` for 16, 17 and 19.
- **21:** `no_wizard_script_uses_a_bash_4_feature`.

The brief's U1 to U8 stay UNVERIFIED by design: they need the operator's real Herdr, tmux and Claude Code run (#778).

## Blocking issues

None.

## Advisory notes

- **ADR-0021 decision 34** (`docs/adr/ADR-0021.md:1606-1607`) still wraps `holler profile` / `apply` inside one code span. That is the only thing keeping the span out of `docs_cli_test`'s line-based extractor. A reflow that joins those lines would fail CI. With the pin repaired, F (or a later edit) can write `holler profile apply NAME` on one line. This is not blocking: CI is green, and it renders correctly.
- **The door passes `HOLLER_HERDR_SOCKET` for a named instance whenever the caller exports `WIZARD_HERDR_SOCKET`.** `panes.sh env` never prints it for a named instance, but a shell that still holds the default instance's value would pass it through. The adapter would then use that socket for the named session. A test that unsets it for a named instance would be cheap, but the skill always evals the env lines fresh, so this is low risk.
- **The fake reduces a session mismatch to `usage`.** The real verbs differ by verb (`docs/pane-control.md` section 2). `proof_803`'s W12c test pins agreement between the plan and the door, not the real verbs' answer. The evidence entry says so.

T-green complete, no blocking issues. No UI surface: U is N/A, ready for S.
