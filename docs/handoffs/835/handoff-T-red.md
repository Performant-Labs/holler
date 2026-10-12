# Handoff-T-red: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 2: the merge with `main` and the guide)

**Date:** 2026-10-11 (19:17 MDT)
**Branch:** `issue-835-implementation` at `529467f` (base `2d217dd`, #649 merged), with `origin/main` (`bda1069`) merged in and not yet committed (`MERGE_HEAD` = `bda1069`)
**Brief / wireframe reviewed:** `docs/handoffs/835-brief.md` (unchanged since `f8a1864`), `docs/handoffs/835/handoff-A.md` (pass 4), `docs/handoffs/835/handoff-T-green.md` (cycle 1, BLOCK). No wireframe (no UI surface).

This handoff replaces cycle 1's (`36b0de8`). Cycle 1's tests are unchanged and still GREEN on the merged tree; this cycle adds one test and fixes two test doc comments.

## A precondition

Confirmed: A returned **PASS** on pass 4 (`529467f`: 0 block, 4 warn). This RED follows A's "Notes for T and F", item 1 (W1 to W4).

## The merge (A's W1)

- `git fetch origin`, then `git merge --no-ff --no-commit origin/main`. `origin/main` is `bda1069`, still the tip A reviewed.
- The merge was clean. Git auto-merged `CHANGELOG.md`, `pane_verbs/launch/rig.rs` and `docs/adr/ADR-0021.md`, with no conflict.
- T commits nothing. `MERGE_HEAD` stays in place, so the script's phase commit (`git add -A`, then `git commit`) records a two-parent merge commit. There was no rebase, no squash and no copied file.
- From here on, `main`'s guide (`docs/pane-control.md`), its test (`docs_pane_control_test.rs`) and #840's text are on the branch, so the RED below is the branch's own.

## Tests authored or changed

| Test | Pins | Tier, and why |
|---|---|---|
| `docs_pane_control_test::import_findings_are_named_in_section_3_or_10` (#652's, **unchanged**) | Sections 3 or 10 of the guide name every `ImportFinding`, so they must name `harness-claude-code` and `harness-unreadable` (W4, T-green's blocking issue 1). | Docs conformance, read from `ImportFinding::ALL`. It is the cheapest test there is, and it already exists, so nothing was added for this item. |
| `docs_pane_control_test::section_3_names_every_flag_of_pane_import` (**new**, A's optional W4 pin) | Section 3's `### `holler pane import` subsection names every non-global long flag of the verb, read from the clap tree, so the guide must name `--adopt-claude-code`. | Docs conformance in the same file, using its helpers (`section`, `from_heading`, `assert_none`). It does not duplicate #652's acceptance 17, which looks at *required* flags in the heading only. |
| `pane_verbs/unmanaged.rs`, doc comment of `the_unmanaged_predicate_is_public_and_one_rule` (W2) | Text only. #831 is now the only caller named, and the comment says the import asks `pane::is_claude_code_command`, so a pane the import leaves alone is listed by this rule. | No behaviour change. |
| `claude_code_rule_test.rs`, the module doc (W3) | Text only. "so #831 can reuse it" replaces "so #831 and #832 reuse it". | No behaviour change. |

**On the scope of the new pin.** I first wrote it for every built verb. It then failed on 25 flags of other verbs, which #652's guide summarises rather than lists (`pane get --profile`, `pane relaunch --project`, ...). Those gaps are #652's, not this story's. So I narrowed the pin to `pane import`, whose subsection already lists every flag in its full synopsis. The narrowing is stated in the test's doc comment. The generic form is an operator follow-up, not part of this story.

## RED confirmation

```
$ cargo test -p holler-cli --test docs_pane_control_test
---- import_findings_are_named_in_section_3_or_10 stdout ----
panicked at crates/holler-cli/tests/docs_pane_control_test.rs:103:5:
sections 3 and 10 do not name these import codes:
  `harness-claude-code`
  `harness-unreadable`

---- section_3_names_every_flag_of_pane_import stdout ----
panicked at crates/holler-cli/tests/docs_pane_control_test.rs:103:5:
section 3's `pane import` subsection does not name these flags:
  --adopt-claude-code

test result: FAILED. 15 passed; 2 failed; 0 ignored
```

Both tests fail on their assertion about the missing guide text. Neither fails on a compile error, a missing `[[test]]` entry (`docs_pane_control_test` is declared by #652) or a setup fault.

The new pin is not vacuous. It finds the verb's five other flags (`--from`, `--herdr-session`, `--launcher`, `--orchestrator`, `--dry-run`) in the subsection, and it names only the one flag that is missing.

**The whole merged tree.** I ran CI's test step (`cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`). Result: 154 groups, 2396 passed, **2 failed**, 32 ignored. The two failures are the two tests above, and nothing else fails.

The other Tier 1 checks on the merged tree:

| Check | Result |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `bash scripts/lint.sh` | exit 0. `docs_pane_control_test.rs` is 549 lines. |
| `bash scripts/changelog-check.sh` | ok |
| `cargo test -p holler-cli --test docs_cli_test` | 3 passed |
| `git diff --check -- crates` | clean. `--cached` flags trailing whitespace only in `main`'s own `docs/handoffs/652-brief.md`, which came in with the merge. |

## Ready for F

**RED is valid.** F may implement against these tests. Per A's note 2, F's cycle is text only, with `archChanged: false`, and the merge is already made:

1. **`docs/pane-control.md`** (W4). Each item is listed in handoff-A W4:
   - **Section 1:** the new exclusion.
   - **Section 3:** the synopsis with `[--adopt-claude-code]` (refused as `usage` until #831), rows for `harness-claude-code` and `harness-unreadable`, the `observe-failed` row narrowed to a failed harness call, and `usage` in the exit-code paragraph.
   - **Section 10:** the fact restated with the rule and its limits, step 1's dry-run check, and the closing paragraph.
   - **Section 12:** drop the #835 line.
   - Keep the words the guide's other tests need (A lists them).
2. **`ImportFinding::ObserveFailed`'s doc** (`import/findings.rs`): narrow it the same way.
3. **#840's sentences** (W2): `unmanaged.rs:63-67` and `:83-84`, and ADR-0021 section 14.
4. **This story's #832 claims** (W3): `pane.rs:146` and ADR-0021 section 13.

Any `holler pane import ... --adopt-claude-code` line in a scanned fence must parse (`docs_cli_test`).
