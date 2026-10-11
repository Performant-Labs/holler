# Handoff-T-red: #838 herdr-session-question

**Branch:** issue-838-implementation
**Brief reviewed:** docs/handoffs/838-brief.md

## A precondition
Confirmed: A returned PASS (handoff-A.md).

## Tests authored
All in `crates/holler-cli/tests/wizard/skilltext_838.rs` (registered in `wizard/main.rs`), unit tier (text assertions on the folded SKILL.md / docs, plus one `instance.sh` run).
- `the_session_question_sits_between_the_menu_and_the_workspace_name` (AC1 order; markers "Present this as an actual choice", "the Herdr session question", "Suggest **two dictionary words")
- `the_question_offers_the_default_and_a_new_named_session_only` (AC1, AC2 refusal wording)
- `the_named_branch_states_the_pattern_the_refusal_and_the_proposed_values` (AC2)
- `the_named_branch_states_the_outside_a_pane_rule_and_state_dir_env` (AC3)
- `stage1_verify_reports_the_targeted_session` (AC1 report)
- `stage3_plan_names_the_herdr_session_in_both_cases` (AC4)
- `stage0_session_row_keeps_its_prefix_and_lists_every_session_with_status` (AC5)
- `the_docs_document_the_question` (AC6; needs a `### Choosing the Herdr session` heading inside "Running an instance beside another")
- `the_proposed_values_for_a_named_session_pass_instance_sh` (AC7; passes already by design)

## RED confirmation
`cargo test -p holler-cli --test wizard_scripts skilltext_838`: 8 failed, 1 passed (the instance.sh one). Each failure is an assertion about missing text, e.g. `no \`the Herdr session question\``, `Stage 3 lacks \`Herdr session: default (unnamed)\``, `Stage 0 row lacks \`its status (running or stopped)\``, `no \`### Choosing the Herdr session\``. No compile or missing-target errors.

## Ready for F
RED is valid. F must use the exact marker phrases above (the session question text must contain "the Herdr session question" and "an existing config is not re-asked"). Note `instance.sh` does not expand `~` (resolved line keeps `~/.holler-work`).
