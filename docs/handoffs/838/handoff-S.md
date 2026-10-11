# Handoff-S: #838 spec audit

**Date:** 2026-10-11  **Branch:** issue-838-implementation  **Issue:** #838
**Reviewed:** brief, handoff-A, T-red, F, T-green, full diff.

## A precondition
Handoff-A verdict PASS. Met. (The a-dup commit reports PASS; no separate a-dup file in the directory, noted only.)

## T precondition
T-green: 285 passed, 0 blocking, RED then GREEN confirmed (revert spot-check: 7 failed). Met.

## Acceptance criteria
| # | Proving test / evidence | Status |
|---|---|---|
| 1 | skilltext_838: the_session_question_sits_between_the_menu_and_the_workspace_name; the_question_offers_the_default_and_a_new_named_session_only; stage1_verify_reports_the_targeted_session | met |
| 2 | the_named_branch_states_the_pattern_the_refusal_and_the_proposed_values | met |
| 3 | the_named_branch_states_the_outside_a_pane_rule_and_state_dir_env | met |
| 4 | stage3_plan_names_the_herdr_session_in_both_cases | met |
| 5 | stage0_session_row_keeps_its_prefix_and_lists_every_session_with_status | met |
| 6 | the_docs_document_the_question | met |
| 7 | the_proposed_values_for_a_named_session_pass_instance_sh | met |
| 8 | wizard_scripts 285 green, lint.sh and changelog-check pass; clippy failure is pre-existing in an untouched file (token_load_test.rs:60), flakes pass alone | met (see notes) |

## Spec compliance
All five decisions implemented: full second instance, default recommended with no table, ports as defaults with collide.sh authoritative, existing config not re-asked with Verify stating the session, existing session refused. The extra "not `default`" rule mirrors instance.sh and is consistent.

## Quality audit
Text only; no Rust production code, no instance.sh/herdr.sh/collide.sh change. Test file 219 lines, allow carries `// #838`. CHANGELOG entry links #838. Docs heading is `###`, avoiding the herdr.rs slicing. Privacy grep of the diff: no personal hostnames or names. Commits are Conventional with Co-Authored-By trailers. PR body disclosure is for O to check after the PR opens.

## Scope check
Exactly the brief's scope, plus the CHANGELOG entry.

## Verdict
PASS

## Advisory notes
Clippy on local rustc 1.93.1 flags a pre-existing test; confirm CI toolchain. Add the AI disclosure to the PR body per CONTRIBUTING.md.
