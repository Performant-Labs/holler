# Handoff-F: Phase 5 - Implement

**Date:** 2026-10-10
**Branch / worktree:** `issue-0700-implementation` (run worktree `<run-worktree>`, base `e49c5aa`; changes uncommitted in the tree)
**Contract:** `handoff-T-red.md` (10 new tests + 5 extensions; 3 targets compile-RED, 2 runtime-RED)

**Status:** Implemented. Locally 1761 tests pass and 1 fails. The failure is a defect in one test's
input literal, not a missing behaviour (see "Tests I think are wrong"). Until T repairs that one
literal, the plugin's T-green run will report **RED**.

(This replaces the earlier F attempt's handoff, which was blocked on file-write permission and
wrote no code. This time the writes went through, and the design in that handoff was applied
unchanged.)

## What I changed

10 files, +91 / -13. No new files, no `Cargo.toml`, no `error.rs`, no test files.

| File | Behaviour | Test(s) it satisfies |
|---|---|---|
| `crates/holler-pane/src/argv.rs` | `AGENT_KEY_INVALID` (open code via `RefusalCode::from_static`) and `AgentKey` beside `EnvVarName`: `parse` accepts a non-empty token of ASCII letters, digits, `-` and `_`, and refuses anything else as `PaneError::Refused { agent-key-invalid }`. The message states the rule and never the text. Also `as_str`, a transparent `Serialize`, and `Deserialize` through `deserialize_parsed` (fails closed). Module doc gains the third guard. | `argv_env_test::agent_key_*`, `records_test::a_record_with_an_invalid_agent_key_fails_to_decode` |
| `crates/holler-pane/src/lib.rs` | `pub use argv::{AgentKey, …}` and the module-list doc line | every test importing `holler_pane::AgentKey` |
| `crates/holler-pane/src/pane.rs`, `profile.rs` | `opencode_agent: Option<AgentKey>` directly after `model`, `#[serde(default, skip_serializing_if = "Option::is_none")]` | `records_test::{pane,profile_spec}_opencode_agent_round_trips_set_and_absent` (field name, after `model`, omitted when `None`, old records load) |
| `crates/holler-pane/src/tx_launch.rs` | `Plan::record` writes `opencode_agent: spec.opencode_agent.clone()` (**A's DECISION 1**, not `None`) | `launch_with_agent_stores_the_key_in_the_record`, `relaunch_without_agent_keeps_the_stored_key` |
| `crates/holler-pane/src/profile_snapshot.rs` | `spec_from_pane` copies `opencode_agent`. The doc table gains its row (A's W5). | `profile_snapshot_test::snapshot_copies_every_spec_field_from_the_pane_record` (set and None halves) |
| `crates/holler-cli/src/pane/args.rs` | `--agent KEY` in `SpecFlags` (after `--role`). `SpecValues.agent: Option<AgentKey>`. `validate()` types it with `AgentKey::parse`, so a bad key is refused there. `validate()` is the first fallible step after the pane name in both verbs, so it runs before any port call or record write. The validate doc list gains the `--agent` line. | `guards::an_invalid_agent_key_is_refused_before_anything`, `relaunch_refuses_an_invalid_agent_key_before_anything`, `cli_surface_test::every_surface_line_parses` |
| `crates/holler-cli/src/pane/launch.rs` | `effective_spec` overlay: `values.agent.clone().or(base.and_then(\|b\| b.opencode_agent.clone()))`, the `command`/`check` precedent. Given → replaces, absent → base, no base → `None`. | `relaunch_with_agent_replaces_the_stored_key`, `relaunch_without_agent_keeps_the_stored_key`, `launch_records_what_the_fakes_show` (None) |
| `crates/holler-pane-testkit/src/fixture.rs` | `sample_pane` and `sample_spec` gain `opencode_agent: None` (compile-forced full literals). The doc lines say "the server's default OpenCode agent". | AC 9: every existing test is unchanged |
| `docs/adr/ADR-0021.md` | One dated row after `model` in §1's Pane table (AC 8). It names the whole delta (both records, `--agent`, the record write, `--from-current`) per A's W4. No other ADR prose was touched. | AC 8 (S inspects it) |

The relaunch verb file needed no edit, because it shares `SpecFlags` and `effective_spec`. Every
other `Pane {` / `ProfileSpec {` site in the workspace is a spread or a mutation. A full
`cargo test --workspace --no-run` compiled with no error and no warning after the literals above,
which confirms A's W1 and T's correction.

## Self-check

All commands were run one cargo at a time, from the worktree root.

- **Configured suite**, with `--no-fail-fast` so that every target reports: `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`
  → **140 test binaries, 1761 passed, 1 failed**, exit 101. The one failure is
  `argv_env_test::agent_key_serde_is_a_plain_string_failing_closed` (below).
- **Per RED target:**

  | Target | T-red | Now |
  |---|---|---|
  | `holler-pane --test argv_env_test` | E0432 `AgentKey` | 7/8 (the 1 is the test defect) |
  | `holler-pane --test records_test` | 3/14 fail | 14/14 |
  | `holler-pane --test profile_snapshot_test` | E0560/E0609 | 5/5 |
  | `holler-cli --test pane_verbs` | 8 compile errors | 219/219 |
  | `holler-cli --test cli_surface_test` | 1/3 fail | 3/3 |

- **`cargo clippy --workspace --all-targets -- -D warnings`** (CI's exact invocation): exit 0, no warnings.
- **fmt:** `rustfmt --check --edition 2021` on every `.rs` file I touched: exit 0.
  `cargo fmt --all --check` does report diffs, but only in files this run did not touch
  (`holler-body`, `holler-cli`, `holler-hub`, `holler-load-test`, `holler-proto`, about 180
  hunks). None of my touched files is among them. These diffs already existed. CI runs no fmt
  step. I did not reformat those files, because they are outside the boundary.
- **No-echo check by hand:** the refusal message `an OpenCode agent key must be non-empty and
  use only ASCII letters, digits, '-' and '_'` contains none of `a b`, `a=b`, `a/b`, `a\nb`,
  `bad key`. It contains `agent` (AC 1's "states the grammar").
- The authoritative verdict is the plugin's T-green run, not this self-check.

## Tests I think are wrong (if any)

**One: `crates/holler-pane/tests/argv_env_test.rs::agent_key_serde_is_a_plain_string_failing_closed`, line 164.** I did not edit it.

```rust
for bad in [r#""a=b""#, r#""a b""#, r#"""#] {
```

The third literal `r#"""#` is a Rust raw string whose content is **one** `"` character. It is not
the JSON empty string `""`, which would be `r#""""#` (or `"\"\""`). serde_json therefore fails
while lexing (`EOF while parsing a string at line 1 column 1`) before any `Deserialize` impl runs.
That error can never contain `agent-key-invalid`. Observed panic:

```
panicked at crates/holler-pane/tests/argv_env_test.rs:166:9:
": EOF while parsing a string at line 1 column 1
```

The two cases before it (`"a=b"`, `"a b"`) pass. T didn't see this at T-red because the target
failed to compile then (E0432), so the runtime RED of this one assertion was never observed.

**Evidence that the intended behaviour is implemented.** I built a throwaway crate outside the
worktree against this `holler-pane`, then deleted it:

```
"\"\""        -> Err("agent-key-invalid: an OpenCode agent key must be non-empty and use only ASCII letters, digits, '-' and '_'")
"\""          -> Err("EOF while parsing a string at line 1 column 1")
"\"a=b\""     -> Err("agent-key-invalid: …")
"\"ok-key_1\"" -> Ok(AgentKey("ok-key_1"))
```

**Suggested repair (T's to make):** change `r#"""#` to `r#""""#` on line 164. With that one
literal fixed, the test exercises the empty-key decode it intends, and the suite should be fully
GREEN.

I did not make the implementation cover for it. Mapping a JSON syntax error to
`agent-key-invalid` would misreport malformed JSON as a bad key, and it would break the
`EnvVarName` precedent of passing decode errors through unchanged.

## Deviations from the brief's boundary (for A / S)

1. **`crates/holler-pane/src/lib.rs` is edited but is not on the brief's "F may edit" list.**
   T's tests import `holler_pane::AgentKey` from the crate root, and T-red's "Ready for F" names
   "root re-export" as F's to land. The edit is one `pub use` item plus the module-list doc
   line. It is inside the issue's blast radius (`holler-pane/**`).
2. `profile_scope.rs` and `conformance/pane_store.rs` were **not** edited, because they hold no
   full literal (A's W1). `CHANGELOG.md` is untouched (journalled out of scope).
3. Not done because they were out of scope: `verb_harness/parse.rs` `SPEC_FLAG_SETS` and
   `docs_rows.rs`'s static flag list (both test files, T's call; A's W6 optional). ADR-0021's
   deferred bullet at about line 677 still words "recording `Pane.opencode_agent`" as #644's.
   DECISION 1 moved that to #700, but the brief allows only one ADR row, and the new row states
   that the launch record writes it. S may want to note that.

## Ready for T(green)

Implemented against the RED contract. Every behaviour T pinned holds. The plugin's run will show
**1 failure**, `agent_key_serde_is_a_plain_string_failing_closed`, caused by the malformed
`r#"""#` input above. The repair is a one-literal test edit, which is T's to make, not an F
rework. With it, I expect the suite GREEN (1762/1762 on the configured command). Nothing is
committed: the production diff is in the worktree, where the plugin's run will pick it up.
