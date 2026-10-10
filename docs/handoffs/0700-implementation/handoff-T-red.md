# Handoff-T-red: Phase 4 — #700 `Pane.opencode_agent`, its guard and the `--agent` flag

**Date:** 2026-10-10
**Branch:** `issue-0700-implementation` (run worktree `<run-worktree>`)
**Brief / wireframe reviewed:** `docs/handoffs/0700-implementation/brief.md` · wireframe N/A (no UI
surface). **A's plan review:** `handoff-A.md` (PASS; DECISION 1 binding — see AC 5/6 below).

## A precondition

Confirmed: A returned PASS on the plan (Phase 3), with DECISION 1 (the record literal writes
`spec.opencode_agent`, not `None`) binding for F and for these tests.

## Tests authored

Ten new tests plus five deliberate extensions, all inside T's boundary (no `src/`, no `Cargo.toml`;
existing targets only). AC → `file::test`:

| AC | Test | Tier and why |
|---|---|---|
| 1 | `crates/holler-pane/tests/argv_env_test.rs::agent_key_accepts_names_and_refuses_malformed_keys` | unit, beside the `EnvVarName` guard tests it mirrors; pins accept (`orchestrator`, `feature-implementor`, `mo`), refuse (`""`, `"a b"`, `"a\nb"`, `"a=b"`, `"a/b"`) each with code `agent-key-invalid`, message states the grammar (names its subject) and never echoes the refused text |
| 2 | `argv_env_test.rs::agent_key_serde_is_a_plain_string_failing_closed` | unit, the `EnvVarName` serde precedent: plain-string round-trip; `"a=b"`, `"a b"`, `""` fail to decode with `agent-key-invalid` in the serde text |
| 2 | `crates/holler-pane/tests/records_test.rs::a_record_with_an_invalid_agent_key_fails_to_decode` | integration-level serde over the whole records (`Pane` and `ProfileSpec`), fail-closed + no-echo at record level; JSON-driven so it **compiles today** |
| 3 | `records_test.rs::pane_opencode_agent_round_trips_set_and_absent` and `::profile_spec_opencode_agent_round_trips_set_and_absent` | serde round-trips, JSON-driven (compiles today): set → field present with the JSON name exactly `opencode_agent` and serialized **after `model`** (textual order, the contract order); absent → field omitted, and an old record with no field loads back with no field |
| 3 (typed half) | `launch_records_what_the_fakes_show` extended with `opencode_agent: None` in the expected record (see AC 5) | the typed None-path pin lives where a typed record is already built |
| 4 | `crates/holler-cli/tests/pane_verbs/launch/guards.rs::an_invalid_agent_key_is_refused_before_anything` | e2e-ish over the rig (the cheapest tier that can observe "before anything"): `"bad key"`, `"a=b"`, `""` → exit 3, code `agent-key-invalid`, `assert_untouched` (call log empty of live calls and writes), no record written |
| 4 | `crates/holler-cli/tests/pane_verbs/relaunch.rs::relaunch_refuses_an_invalid_agent_key_before_anything` | same refusal through the shared `SpecFlags` on `relaunch` |
| 4 (positive parse) | `launch_with_agent_stores_the_key_in_the_record` / `relaunch_with_agent_replaces_the_stored_key` | a valid key proceeds past `validate` on both verbs (parse + refusal in one place per verb) |
| 5 | `relaunch.rs::relaunch_with_agent_replaces_the_stored_key` (given → replaces); `relaunch.rs::relaunch_without_agent_keeps_the_stored_key` (absent → base spec's value; base = `spec_from_pane` of the record); `launch_records_what_the_fakes_show` extension (neither → `None`) | overlay pinned through the verbs (the `effective_spec` unit is `pub(crate)`, not reachable from tests); the keeps-test is DECISION 1's landmine pin — a flag-less relaunch must not reset a stored key |
| 6 (DECISION 1 positive) | `launch.rs::launch_with_agent_stores_the_key_in_the_record` | a launch with `--agent feature-implementor` stores `Some(...)` in the pane record |
| 6 | `crates/holler-pane/tests/profile_snapshot_test.rs::snapshot_copies_every_spec_field_from_the_pane_record` extended (`full_pane()` and the expected literal gain `Some("orchestrator")`; plus a None half asserting `spec_from_pane` of a keyless record holds none) | `spec_from_pane` is pure — the pure-function tier is the cheapest that pins the `--from-current` copy |
| 7 | `crates/holler-cli/tests/fixtures/cli-surface.txt` — the `# #644` launch line (the `--model … --role agent` one) gains `--agent orchestrator`; exactly once, no new leaf | the fixture's own tests are the assertion (`every_surface_line_parses` enforces parse, `fixture_leaf_set_equals_clap_leaf_set` leaf parity) |
| 8 | — | S's inspection check (journaled W3), not a T test |
| 9 | — | no existing test edited in behaviour: the extensions are field lines that are behaviour-neutral once the field exists; today's runs show every untouched test still green (below) |
| 10 | — | F's gate (fmt/clippy) |

New tests by file: `argv_env_test.rs` 2 · `records_test.rs` 3 · `profile_snapshot_test.rs` 0 (one
extended + a helper) · `pane_verbs/launch.rs` 1 (+1 literal line) · `pane_verbs/launch/guards.rs` 1
· `pane_verbs/relaunch.rs` 3 · `launch/rig.rs` 0 (an `agent()` helper + the `launch_spec()` literal
line) · fixture 1 line.

## RED confirmation

Commands run one cargo at a time, from the run root. Every failure below names the missing
type/field/flag or the exact assertion the feature will satisfy — no import typos, no setup errors.

**Runtime RED (targets compile today and fail on the right assertion):**

1. `cargo test -p holler-pane --test records_test` → **11 passed, 3 failed**:
   - `pane_opencode_agent_round_trips_set_and_absent`: `unwrap()` on
     `Error("unknown field \`opencode_agent\`, expected one of \`name\`, … \`probe\`")` — the set-half
     cannot decode before the field exists (the assertion the feature satisfies).
   - `profile_spec_opencode_agent_round_trips_set_and_absent`: same, expected list
     `\`pane\`, … \`expect\``.
   - `a_record_with_an_invalid_agent_key_fails_to_decode`: fails at
     `assert!(msg.contains("agent-key-invalid"))` with today's message
     `"bad key": unknown field \`opencode_agent\`, …` — the decode refuses (unknown field) but not
     yet with the guard's code.
2. `cargo test -p holler-cli --test cli_surface_test` → **2 passed, 1 failed**:
   `every_surface_line_parses`: `1 of 140 surface lines failed to parse: [… "--agent",
   "orchestrator"]: error: unexpected argument '--agent' found` — the flag does not exist yet; the
   leaf-parity and pending tests stay green (no leaf added).

**Compile RED (the API under test does not exist; per the brief this is the legitimate RED for these
ACs):**

3. `cargo test -p holler-pane --test argv_env_test` →
   `error[E0432]: unresolved import \`holler_pane::AgentKey\` — no \`AgentKey\` in the root` (the
   missing guard type; AC 1-2).
4. `cargo test -p holler-pane --test profile_snapshot_test` → 5 errors, all
   `E0560`/`E0609` `no field \`opencode_agent\` on type \`Pane\`/\`ProfileSpec\`` (AC 6).
5. `cargo test -p holler-cli --test pane_verbs` → 8 errors, every one naming the missing API:
   `E0432` `AgentKey` (`launch/rig.rs:31`), `E0560` on the `launch_spec()` literal (`rig.rs:579`)
   and the expected record (`launch.rs:80`), `E0609` on the typed accesses (`launch.rs:129`;
   `relaunch.rs:544/551/565/572`) — AC 4-6. The `--agent` argv strings themselves compile; the
   refusals they will trigger are runtime-red only after the target compiles.

**Green sanity (AC 9, today):** `cargo test -p holler-pane --lib` → 12 passed;
`cargo test -p holler-cli --test profile_verbs` → 44 passed; the 11 pre-existing `records_test`
tests and the 2 other `cli_surface_test` tests all pass unedited. The configured full-suite command
(`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`) is expected to
fail compilation at targets 3-5 until F lands — that is the RED contract, not a stray break.

**Split:** 3 targets compile-RED (`argv_env_test`, `profile_snapshot_test`, `pane_verbs`), 2 targets
runtime-RED (`records_test` 3/14 failing, `cli_surface_test` 1/3 failing).

## Where A/the brief were off (hit while authoring)

- **A's §3 "definitive" literal table over-lists T's compile-forced sites** (the journaled W1
  correction is itself approximate): `launch.rs:57`, `launch/rig.rs` ~330 (`seed_live`) and
  `list.rs:121` are all **spread** literals (`..sample` / `..pane(name)`), not full literals — no
  edit is compile-forced at any of them. The only full literal in T's tree is `launch_spec()`
  (`rig.rs` ~555), which gained `opencode_agent: None`. I still added the field line to the
  expected record at `launch.rs:57` (optional under the spread) because it pins DECISION 1's
  None-half explicitly; `seed_live` and `list.rs` are untouched. No boundary consequence.
- The brief's AC 1 "message states the grammar" is pinned lightly (`message.contains("agent")`,
  the `EnvNameInvalid` precedent names its subject) plus the per-case no-echo checks; wording beyond
  that is left to F.
- `tests/verb_harness/parse.rs` (`SPEC_FLAG_SETS`, behind `assert_spec_flags_accepted`) is **outside
  T's boundary**, so `--agent` is not added to the shared flag-set list; its parse coverage comes
  from the verb-level tests above. (A's W6 said optional; it is also out of reach.) The same applies
  to `docs_rows.rs`'s static flag list — skipped, not required for green.

## Ready for F

Confirmed: the RED is valid. Every new test fails for the right reason (missing `AgentKey` /
`opencode_agent` / `--agent`, or the specific unknown-field/parse refusal), nothing passes
vacuously, and every pre-existing outcome is unchanged. F may implement against these tests: the
guard (`AgentKey::parse`, `as_str`, serde through `deserialize_parsed`, root re-export), the field
on `Pane` and `ProfileSpec` (after `model`, `#[serde(default, skip_serializing_if =
"Option::is_none")]`), `spec_from_pane`'s copy, the `--agent` flag in `SpecFlags`/`SpecValues`/
`validate()` refusing `agent-key-invalid` before anything, the `effective_spec` overlay
(`values.agent.or(base.and_then(…))`), and DECISION 1's record line
(`opencode_agent: spec.opencode_agent.clone()` in `Plan::record`).

No mutation testing was run (operator rule, this run). Test files staged by explicit path (8
files); nothing committed.

RED: CONFIRMED
