# Handoff-T-red: #644 part 2 - the OpenCode agent through `launch` and `relaunch`

**Date:** 2026-10-11
**Branch:** `issue-644-implementation` at `93b2cd7` (merge base `origin/main` `7284001`, #802 merged)
**Brief / wireframe reviewed:** `docs/handoffs/644-brief.md`, `docs/handoffs/644/handoff-A.md` (no wireframe: no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (`handoff-A.md`, 0 block, 6 warn). The tests follow A's notes for T. The brief does not yet carry them, so this handoff applies them directly:

- **Warn 2.** The key prints through `text_value`, so `--agent=-` prints `agent "-"`.
- **Warn 3.** The message is the harness's own message, followed once by the reconcile step. It is not "unchanged".
- **Warn 4.** A refused relaunch leaves the record unchanged and its server stopped.
- **Warn 5.** Existing tests are extended rather than copied. Acceptance 2 gets no new-module clause, and Acceptance 4 and 5's failure half are one table in `agent.rs`.
- **Key order** is checked on the raw stdout.

## Tests authored

| Test | Pins | Tier | Why this tier |
|---|---|---|---|
| `launch/agent.rs` `the_text_line_names_the_agent_and_is_unchanged_without_one` | Acc 1, Scope 1 text: `, agent KEY` after the harness port. Without `--agent`, the golden line is exactly today's. | CLI verb over fakes | `Report::text` is private; the verb is the only way to reach it |
| `launch/agent.rs` `a_key_that_reads_as_the_empty_value_is_quoted` | Acc 1, A warn 2: `--agent=-` prints `agent "-"` (`text_value`) | same | same |
| `launch/agent.rs` `data_carries_the_recorded_agent_after_spec_only` | Acc 1, Scope 1 JSON: `data.agent` is the key or `null`, equals `data.pane.opencode_agent`, and is the last key after `spec_only` (checked in the raw output) | same | JSON key order is visible only in the raw output |
| `launch/agent.rs` `spec_only_reports_no_agent` | Acc 1: with `--spec-only --agent KEY`, `data.agent` is `null`, taken from the record and not the request, while P's spec holds the key | same | same |
| `launch/agent.rs` `the_help_of_both_verbs_names_the_agent` | Scope 4: the corrected `--agent` line, the old line absent, and `agent` in both verbs' data-key list | clap parse (`crate::get::help`) | no process needed |
| `launch/agent.rs` `a_launch_the_server_refuses_records_nothing` | Acc 4 + Acc 5 failure half: a table over {stand-in open code `agent-unusable`, the fake's `agent-mismatch`, stand-in with `--profile demo`}. Each row checks exit 3, code, message + step once, `VerifyServing` called, no `CreateSession`, no `CasPut`, the server killed and no record. The `--profile` row checks P's specs equal and the log `[1, 2, 3]`; the others check the log `[1]`. | CLI verb over fakes | the rollback crosses the engine and the verb's `failure_body` |
| `launch/agent.rs` `a_relaunch_the_server_refuses_leaves_the_record_unchanged` | Acc 4 relaunch, both refusals: `StopOwned` twice (B1, then the rollback), the record unchanged, the new server killed | same | same |
| `launch.rs` `launch_with_agent_stores_the_key_in_the_record` (extended) | Acc 2: `data.agent == KEY`, one `CasPut`, record generation 1 | same | extends #700's test (no copy) |
| `relaunch.rs` `relaunch_without_agent_keeps_the_stored_key`, `relaunch_with_agent_replaces_the_stored_key` (extended) | Acc 3: `data.agent` is the kept key or the replaced key | same | extends #700's tests |
| `launch/profiles.rs` `launch_with_profile_adds_the_spec_and_bumps_once` (extended) | Acc 5 success: `--agent` lands in P's spec (`launch_spec()` plus the key), the log stays `[1, 2]`, and `data.agent` is the key | same | extends #644 part 1's test |
| `profile_diff_test.rs` `spec_field_values_serialize_like_the_spec` (extended) | Acc 6: `PATHS` has 16 entries with `opencode_agent` after `model.effort`; the value serializes like the spec, both with an agent and without one (`null`) | unit (pure) | the cheapest tier for the table |
| `profile_diff_test.rs` `diff_reports_an_agent_change` | Acc 6: a key vs none differs (JSON `{"field":"opencode_agent","spec":KEY,"live":null}`), two keys differ, equal keys do not | unit | same |
| `profile_diff_test.rs` `an_absent_agent_prints_default_and_a_key_named_none_prints_none` | Acc 6, decision 6: `(default)` vs `none`, and `(default)` is not a valid key | unit | same |
| `profile_verbs/show.rs` `show_lists_the_agent_and_reports_an_agent_difference` | Acc 7: `opencode_agent: KEY` and `opencode_agent: (default)` in the spec blocks; the difference line `opencode_agent: spec KEY, live (default)`; the JSON difference; the agent-less pane `matches` | CLI verb over fakes | `profile show` is the surface named |

The new field is reached only through `SpecField::ALL` (by its path), and no test names `SpecField::OpencodeAgent` or `FieldValue::Key`. The suite therefore compiles before F, and every failure is an assertion, not a compile error. `crate::get::help` and the rig's `pub(crate)` helpers are reused, and `launch/rig.rs` is not edited. The new `mod agent;` sits in `launch.rs`, and the module index in its doc gains one line, per A's note. File sizes after the change: `launch.rs` 748, `agent.rs` about 300, `relaunch.rs` 633, `profiles.rs` 212, `show.rs` 365, `profile_diff_test.rs` 400. All are under 900.

## RED confirmation

`cargo clippy -p holler-cli -p holler-pane --all-targets -- -D warnings` is clean. The commands and their failures:

```
cargo test -p holler-pane --test profile_diff_test     -> 8 passed; 3 failed
  spec_field_values_serialize_like_the_spec: left (ALL paths, 15) != right (PATHS, 16, "opencode_agent" after "model.effort")
  diff_reports_an_agent_change: panicked "SpecField::ALL has a field at opencode_agent"
  an_absent_agent_prints_default_and_a_key_named_none_prints_none: panicked "SpecField::ALL has a field at opencode_agent"

cargo test -p holler-cli --test pane_verbs              -> 266 passed; 9 failed
  agent::the_text_line_names_the_agent_and_is_unchanged_without_one:
    left  ["launched demo-c1r1 at r2c1 in main: session ses_00000000000000000000000001, harness port 48100"]
    right [... "harness port 48100, agent feature-implementor"]
  agent::a_key_that_reads_as_the_empty_value_is_quoted: right [... "harness port 48100, agent \"-\""]
  agent::data_carries_the_recorded_agent_after_spec_only: data.get("agent") left None, right Some("feature-implementor")
  agent::spec_only_reports_no_agent: left None, right Some(Null)   (data ends "spec_only":true})
  agent::the_help_of_both_verbs_names_the_agent: help still says "Without it, the server's default agent"
  launch::launch_with_agent_stores_the_key_in_the_record: data["agent"] left Null, right "feature-implementor"
  profiles::launch_with_profile_adds_the_spec_and_bumps_once: data["agent"] left Null, right "feature-implementor"
  relaunch::relaunch_without_agent_keeps_the_stored_key: left Null, right "orchestrator"
  relaunch::relaunch_with_agent_replaces_the_stored_key: left Null, right "feature-implementor"

cargo test -p holler-cli --test profile_verbs           -> 44 passed; 1 failed
  show::show_lists_the_agent_and_reports_an_agent_difference: the spec block of demo-c1r1 has no "  opencode_agent: feature-implementor" (15 field lines)
```

Each failure is the missing behavior: the result key or line, the help text, or the 16th field. None is an import error, a setup error or a timeout. Every existing test still passes.

**Green before F, and why that is valid.** `a_launch_the_server_refuses_records_nothing` and `a_relaunch_the_server_refuses_leaves_the_record_unchanged` (Acceptance 4, and the failure half of Acceptance 5) pass today. Scope 3 says the refusal and its rollback are the engine's own (#802, merged) and that the verbs add no check. These tests are regression pins of behavior the brief says needs no code change, not a RED for F. They do check something real:

- The `agent-mismatch` rows fail with the fake's own code only because the hook really restarted the server with another agent.
- The relaunch rows count two `StopOwned` calls.

## Ready for F

RED is valid. F may implement against these tests:

- `Report.agent` from `launched.pane`, after `spec_only`;
- the text clause through `text_value`;
- the `--agent` help line and both verbs' data-key docs;
- `SpecField::OpencodeAgent` in `ALL` after `Effort`, and `FieldValue::Key(Option<String>)` printing `(default)` for `None`.

F must not edit tests. Formatting note: `cargo fmt --all` from this worktree reformats about 180 unrelated files (the repo has no rustfmt config and CI does not check it), so run `rustfmt --edition 2021 <file>` only on the files F changes.
