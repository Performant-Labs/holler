# Handoff-T-red: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code`

**Date:** 2026-10-11 (18:18 MDT)
**Branch:** `issue-835-implementation` at `49e4508` (base `2d217dd`, #649 merged)
**Brief / wireframe reviewed:** `docs/handoffs/835-brief.md` (issue, amendment §1-§7, rounds 2 and 3), `docs/handoffs/835/handoff-A.md` (pass 3). No wireframe (no UI surface).

## A precondition

Confirmed: A returned **PASS** on the plan (pass 3, `49e4508`, 0 block, 6 warn). This RED follows A's "Notes for T and F" (W1, W3 to W6).

## What F must add for the suite to compile (the API the tests name)

The tests name these, and nothing else that is new. All are in the brief except the names marked *chosen here*:

| Item | Where | Notes |
|---|---|---|
| `HostPort::foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>`, **required** | `holler-pane/src/ports.rs` | brief §1 and R2-1 |
| `HerdrPort::observe_pane` with **no default** | `holler-pane/src/ports.rs` | R3-1. `observe_panes` keeps its default |
| `HostOp::ForegroundCommands`, with `as_str` = `"host.foreground_commands"` | `holler-pane-testkit/src/host.rs` | variant name *chosen here*; `as_str` must equal the adapter's `OP_*` |
| `FakeHost::foreground_commands`: the basename of `argv[0]` for each process `run` started, oldest first. A fresh session answers `[]`. The name goes away with the process (`exit_process`, `stop_owned`, `end_session`), and a missing session is `pane-not-found`. Faults and the call log apply as for every method. | `holler-pane-testkit/src/host.rs` | brief §1, A's reuse map |
| `pub const CLAUDE_CODE_COMMAND: &str = "claude"` and `pub fn is_claude_code_command(name: &str) -> bool` (exact match) | `holler_pane::pane` (`pane.rs`) | names *chosen here*; R2-7 fixes the home |
| `ImportFinding` codes `harness-claude-code` (`info`) and `harness-unreadable` (`problem`) | `holler-pane/src/import/findings.rs` | R2-4 |
| `--adopt-claude-code` on `PaneImport` | `holler-cli/src/pane/import.rs` | brief §4 |
| The forwards in `TmuxHost`, `Unwired`, `BindingHost` and `Wired<dyn HostPort>` | production | the compiler lists them |

T already added the forwards that live in **test files**: `Mutant` (`host_conformance_test.rs`), `TestHost` and `TestHerdr` (`ports_test.rs`), the three `LinkedHost`s (`pane_integration/bind.rs`, `pane_verbs/launch/rig.rs`, `profile_verbs/apply/rig.rs`) and the new `LostChain` (`import/harness.rs`).

A's notes 6 ("`TestHerdr` gets `observe_pane`", "`LinkedHost` ... forwards `foreground_commands`") assigned these to F. They live in test files, which are T's (F writes no test), so T did them.

## Tests authored

### The import's behaviour (`holler-cli`, `crates/holler-cli/tests/pane_verbs/import/harness.rs`, new)

These run in-process over the import rig's fakes, the cheapest layer that runs the verb end to end. The rig needed no new scenario method: `FakeHost::run` starts a process in a pane's session.

| Test | Criterion it pins |
|---|---|
| `a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode` | Issue Work 1, brief §2 and §3. Three panes: A's session runs `bash`, B's runs `bash` and then `claude`, C's runs nothing. A real run records A and C as OpenCode, unchanged. B gets exactly one finding, `harness-claude-code` at `info`, and the row `skipped` with null role, position and session. B gets no record, and `fleet` holds A and C only. There is no harness call for B (2 `health` and 2 `list_sessions` for 3 panes). |
| `a_dry_run_reports_the_pane_left_alone_and_writes_nothing` | Issue Work 1. The dry run shows the same finding, makes no store write and changes no file. The text form shows `B skipped - - -` and the line `  info harness-claude-code: `. |
| `a_pane_left_alone_blocks_nothing_even_unlabelled` | Brief §3 ("one finding and no others"). A Claude Code pane with no labelled Herdr pane still has only `harness-claude-code`, and the real run is not refused. |
| `adopt_claude_code_is_usage_naming_831_and_writes_nothing` | Issue Work 2, brief §4. `--adopt-claude-code` alone and with `--dry-run`, in JSON and text. Each exits 2 with `usage` and a message containing `not_implemented_message(831)`. Text puts one line on err. No Herdr, host or harness call, no store write, no file changed. |
| `a_failed_ps_is_harness_unreadable_and_blocks` | R2-6, A's W4. Replaces #650's `a_failed_ps_is_observe_failed_and_the_pane_imports` (`join.rs`, now removed with a pointer comment). A failed `ps` gives `harness-unreadable` at `problem`, the row `blocked`, and a finding of the pane that names `host.ps`. The real run is refused with `import-blocked`. |
| `a_failed_command_name_read_is_a_problem_and_blocks` | Brief §2 and A's W4. The read answers `unavailable`: `harness-unreadable`, with the message naming `host.foreground_commands`. The read answers `pane-not-found`: `tmux-session-missing`. Both are a problem, the row `blocked` and a real run refused. |
| `a_lost_chain_is_harness_unreadable_and_blocks` | R2-6. A host whose `ps` panics loses the chain. The pane gets `harness-unreadable` and the real run is refused. |

### The verb's records and surface (`holler-cli`)

| Test | Criterion | Tier and why |
|---|---|---|
| `pane_verbs/import.rs::the_adrs_name_the_switch_and_the_two_new_codes` | Brief §4 and R2-2. ADR-0021 §9's `pane import` row names `--adopt-claude-code`. ADR-0021 has a "Decided (#835)" paragraph and names both codes in backticks. ADR 0003's line has `[--adopt-claude-code]`. | A text read. It follows the existing `adr_0021_lists_the_import_codes`. |
| `tests/fixtures/cli-surface.txt` (+1 line, `# #835`), pinned by `cli_surface_test::every_surface_line_parses` | Brief §4 | the fixture of ADR 0003's CLI |

### The finding vocabulary (`holler-pane`, `tests/import_test.rs`, edited)

- `NEW_CODES` goes from 10 to 12 entries: `harness-claude-code` and `harness-unreadable`.
- The level map pins `harness-unreadable` at `problem` and `harness-claude-code` at `info`. `harness-claude-code` is `info`, not `skipped`. Brief §3 records that difference from `harness-not-opencode`.
- RED is in `import_finding_codes_are_the_pinned_set`.
- Unit tier: the one code table.

### The rule (`holler-pane`, `tests/claude_code_rule_test.rs`, new)

| Test | Pins |
|---|---|
| `the_rule_is_the_exact_command_name_claude` | `CLAUDE_CODE_COMMAND == "claude"`, and the predicate accepts it (R2-7). |
| `no_other_command_name_is_claude_code` | The match is exact. It rejects `bash`, `sh`, `opencode`, `node`, `env`, the empty name, case variants, padded names, `claude-code`, `xclaude` and a path. |

This is the cheapest tier for the rule's edges. The CLI tests above pin how it is wired in.

### The port contract

**`ports.rs` doctests**, two `compile_fail,E0046` (A's W1):
- An impl of `HerdrPort` without `observe_pane` must not compile (R3-1).
- An impl of `HostPort` with only the four older methods must not compile (R2-1).
- They are doctests because "required" is a compile-time property.
- They are doc comments only, on the two trait docs.

**Host conformance suite** (`holler-pane-testkit/src/conformance/host.rs`), case 10 `foreground-commands-names-what-run-started`:
- A session never ensured is `pane-not-found`.
- After `run(sleep 30)` the session's list names `sleep` within 2 s, polling every 20 ms.
- It never asserts an empty list, because a real session lists its shell (R2-8).
- It holds on real tmux (`real_tmux_test`, ignored by default) and on the fake.

**`host_conformance_test.rs`**:
- `DOCUMENTED_CASES` grows to 10.
- The `Mutant` forwards `foreground_commands`, so `the_unbroken_host_wrapper_passes` still holds.
- Two breaks, `ForegroundNamesNothing` and `ForegroundOfMissingIsEmpty`. Each has a test asserting that the suite fails on case 10.

**`fake_host_test.rs`** pins what only the fake has:
- `host_op_names_are_port_dot_method` and `a_wedged_host_times_out_every_method` are extended.
- `foreground_commands_names_each_process_by_its_programs_basename`: basename, order, per session, and an empty fresh session.
- `foreground_commands_follow_the_processes_out`: `exit_process`, `stop_owned` and `end_session`.
- `a_fault_on_foreground_commands_fails_that_call_only`.

### The adapter (`holler-adapter-host`)

**`tests/fake_tmux_test.rs`** (818 to 820 lines, R2-3):
- `METHODS` gets `HostOp::ForegroundCommands`.
- `call` gets its arm.
- So `every_method_is_bounded_and_reaps_its_child` pins `Timeout { op: "host.foreground_commands" }`.
- And `a_missing_tmux_binary_is_unavailable_from_every_method` covers it too.

**`tests/foreground_test.rs`** (new; uses `common::host`, never `TmuxHost::new`). The fake tmux fills in whatever `-F` format the adapter sends, so these tests pin what is read, not how the format is spelled:

| Test | Pins |
|---|---|
| `foreground_commands_names_every_live_pane_in_tmux_order` | Live panes in tmux order, with the dead one dropped. One `list-panes -s -t =demo-c1r1:` call. The format holds `#{pane_current_command}` and no `start_command` (names only, I7). |
| `foreground_commands_of_a_missing_session_is_pane_not_found` | Three missing-session stderrs answer `pane-not-found`. |
| `another_tmux_failure_is_unavailable_with_its_first_line` | Any other failure is `unavailable` with stderr's first line only. |
| `a_malformed_listing_is_unavailable` | Strict parsing. A dead flag that is not `0` or `1` makes the whole answer `unavailable`. This implies the format carries `#{pane_dead}`, as `PS_FORMAT` does (A's reuse map: "a strict parser beside `parse_ps` that drops dead panes"). |

### The decorators (`holler-cli`, `tests/pane_integration/`)

- `bind.rs::the_wired_host_passes_the_host_conformance_suite` is new (A's W6). It runs the host suite over `WiredHost::new(Ok(Arc::new(FakeHost::new())))`.
- The existing `BindingHost` run picks up case 10 with no edit, which pins that forward.
- `wired.rs` gets one line: with a malformed `HOLLER_TMUX_SOCKET`, `host.foreground_commands` answers `usage` naming the variable.
- No recording host is written (A's W6).

### R3-3, a regression pin (`tests/pane_integration/observe.rs`, +2 tests)

- The tests:
  - `pane_list_over_the_wirings_herdr_chain_shows_the_cwd_and_the_screen_change`
  - `pane_list_over_the_wirings_herdr_chain_invents_no_screen_change`
- They run `holler pane list` over `BindingHerdr(LazyHerdr(FakeHerdr))` with `seed_unmanaged_pane`, in JSON and text:
  - The pane's working directory is shown in both forms.
  - A time the adapter reports is shown.
  - When it reports none, the time is `null` in JSON and `-` in text, never invented.
- **They pass before F starts, on purpose.** #649's D33 already makes the chain forward (A's W1). They guard the chain once the default goes away.
- R3-2's per-decorator tests are **not** copied. D33's tests in `observe.rs` and `bind.rs` already pin them.

## RED confirmation

The final RED is a **compile RED rooted in the missing port method**. The test kit's conformance suite now calls `foreground_commands`, so the test-kit lib does not build, and every dependent test target fails with it:

```
$ cargo test --workspace --no-run
error[E0599]: no method named `foreground_commands` found for reference `&dyn HostPort` in the current scope
   --> crates/holler-pane-testkit/src/conformance/host.rs:230:14
error[E0599]: no method named `foreground_commands` found for reference `&dyn HostPort` in the current scope
   --> crates/holler-pane-testkit/src/conformance/host.rs:239:18
error: could not compile `holler-pane-testkit` (lib) due to 2 previous errors

$ cargo test -p holler-pane --no-run
error[E0432]: unresolved imports `holler_pane::pane::is_claude_code_command`, `holler_pane::pane::CLAUDE_CODE_COMMAND`  --> tests/claude_code_rule_test.rs:11
error[E0407]: method `foreground_commands` is not a member of trait `HostPort`                                         --> tests/ports_test.rs:219
```

**A compile error is unavoidable for any test of a new required trait method**, and A's W1 accepts it ("any test that calls it is RED until F adds it"). So the RED was also proven at assertion level in two steps.

**Step 1: before any group-B code was written.** The tests that use only today's API were run on the unchanged production code:

```
cargo test -p holler-pane --test import_test
  import_finding_codes_are_the_pinned_set ... FAILED
    left: {.. 16 codes ..}   right: {.. + "harness-claude-code", "harness-unreadable" ..}
cargo test -p holler-pane --doc ports
  ports::HerdrPort (line 173) - compile fail ... FAILED   "Test compiled successfully, but it's marked `compile_fail`."
  ports::HostPort (line 296)  - compile fail ... FAILED   "Test compiled successfully, but it's marked `compile_fail`."
cargo test -p holler-cli --test cli_surface_test
  every_surface_line_parses ... FAILED   "unexpected argument '--adopt-claude-code' found"
cargo test -p holler-cli --test pane_verbs import::      (38 passed; 6 failed)
  harness::a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode  exactly one harness-claude-code about hj-c1r2 (left: 0, right: 1); B was recorded ("recorded":3)
  harness::a_dry_run_reports_the_pane_left_alone_and_writes_nothing               exactly one harness-claude-code about hj-c1r2 (left: 0, right: 1)
  harness::a_pane_left_alone_blocks_nothing_even_unlabelled                      the run succeeds (left: 3 import-blocked, right: 0)
  harness::a_failed_ps_is_harness_unreadable_and_blocks                          exactly one harness-unreadable about hj-c1r1 (left: 0, right: 1)
  harness::adopt_claude_code_is_usage_naming_831_and_writes_nothing              "... must parse: error: unexpected argument '--adopt-claude-code' found"
  the_adrs_name_the_switch_and_the_two_new_codes                                 the import row names the switch: | `pane import` | ...
cargo test -p holler-cli --test pane_integration observe::   7 passed (R3-3's 2 regression pins pass, as intended)
cargo test -p holler-pane --test ports_test                   8 passed (TestHerdr's observe_pane compiles today)
```

The `--adopt-claude-code` test fails when the argv is parsed, because the switch does not exist yet. That is the missing feature, not a harness fault.

**Step 2: after group B was written, with a temporary stub.**
- **The stub was scaffolding only and was reverted.** It added the bare API (the trait method, the variant, the constant and the predicate), with every body `Err(NotImplemented)` or `false` and no behaviour.
- **The six production files were restored from backups.** `sha256sum -c` passed for all six, and they were touched so that cargo rebuilds them.
- **What the stub showed:**
  - The whole suite compiles, with no warning.
  - `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
  - `cargo test --workspace --no-fail-fast` fails only the tests of this story, each on an assertion. 28 fail, and the run has no unrelated failure:

```
fake_host_test::foreground_commands_names_each_process_by_its_programs_basename   left: Err(NotImplemented) right: Ok([])
fake_host_test::foreground_commands_follow_the_processes_out                      left: Err(NotImplemented) right: Ok(["sleep"])
fake_host_test::a_fault_on_foreground_commands_fails_that_call_only               left: Err(NotImplemented) right: Err(Unavailable { what: "made-up" })
fake_host_test::a_wedged_host_times_out_every_method                              right: Err(Timeout { op: "host.foreground_commands" })
host_conformance_test::the_fake_passes_the_host_conformance_suite                 case "foreground-commands-names-what-run-started": expected `pane-not-found`, got `not-implemented`
host_conformance_test::the_unbroken_host_wrapper_passes                           (same case)
fake_tmux_test::every_method_is_bounded_and_reaps_its_child                       host.foreground_commands: expected Timeout, got Err(NotImplemented)
fake_tmux_test::a_missing_tmux_binary_is_unavailable_from_every_method            host.foreground_commands: expected Unavailable
foreground_test (all 4)                                                           Err(NotImplemented) where a list, pane-not-found or unavailable is due
pane_integration bind::the_binding_host_passes_the_host_conformance_suite         case 10 fails
pane_integration bind::the_wired_host_passes_the_host_conformance_suite           case 10 fails
pane_integration wired:: (both)                                                   host.foreground_commands: want `usage` naming HOLLER_TMUX_SOCKET
pane_verbs import::harness:: (all 7) and import::the_adrs_...                     as in step 1; plus a_failed_command_name_read_is_a_problem_and_blocks and
                                                                                  a_lost_chain_is_harness_unreadable_and_blocks: exactly one harness-unreadable (left: 0, right: 1)
claude_code_rule_test::the_rule_is_the_exact_command_name_claude                  left: "" right: "claude"
import_test::import_finding_codes_are_the_pinned_set                              as in step 1
cli_surface_test::every_surface_line_parses                                       as in step 1
ports.rs doctest HerdrPort                                                        compiled successfully (the stub kept observe_pane's default)
```

Three kinds of test pass under the stub:
- `ports.rs`'s `HostPort` doctest. It goes green once the method is required, which shows it is satisfiable.
- `no_other_command_name_is_claude_code`. It is a negative pin, so an always-`false` stub passes it.
- The two new mutant tests. Every host fails case 10 under the stub. They become meaningful once the fake passes, and are spot-checked in phase 6.

## Ready for F

**RED is valid; F may implement against these tests.**

- Build order: the port method and `HostOp` variant first, which un-blocks every target. Then `FakeHost`, the adapter, the forwards, the rule in `pane.rs`, the import's chain and findings, and the switch.
- Then the docs: ADR-0021 §2, §9, §13's "Decided (#835)" paragraph and §14; ADR 0003; `CHANGELOG.md`; the module docs of `ports.rs`.

Notes for F:
1. **`import/rig.rs`'s read allow-list** already admits `HostOp::ForegroundCommands`. The import may call nothing else new.
2. **The message for a failed read** must name `host.foreground_commands`, as `failed_message` does. The `harness-unreadable` finding of a failed `ps` must have a finding of that pane naming `host.ps`.
3. **A claude pane** gets exactly one finding, makes no harness call and blocks nothing, even when it would have had a join problem.
4. **The switch's refusal** must come before any port call, including Herdr's, so it belongs in `Input::read` step 1.
5. **R3-3's pins** (in `observe.rs`) and the `HostPort` doctest pass as soon as the API exists. The `HerdrPort` doctest needs `observe_pane`'s default removed.
6. **#650's test of a failed `ps` is gone from `join.rs`.** Its replacement is in `harness.rs`.
