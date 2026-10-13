# Handoff-T-red: Phase 4 - #850: a body driver for a Claude Code pane

**Date:** 2026-10-12
**Branch:** issue-850-implementation (worktree `.claude/worktrees/0850-claude-body`, `main` at `ef881fc`)
**Brief / wireframe reviewed:** `docs/handoffs/850-brief.md` (Acceptance 1 to 10, plan review W-1 to W-6 of `handoff-A.md`); no wireframe (`uiSurface: false`)

## A precondition

Confirmed: A returned PASS on the plan (`docs/handoffs/850/handoff-A.md`, Phase 3, 0 blocks, 6 warns).

## Tests authored

The tests are staged by explicit path and not committed. Each row is a file, the behavior it pins, the tier and why. The names are the brief's, sharpened where the brief left a shape open.

| File (new unless marked) | Acceptance | Tier and why |
|---|---|---|
| `crates/holler-body/tests/claude_row_test.rs` | 1 | `holler-body` integration over `config::parse`: the row rule (a UUID `session_id`, no `endpoint`, no `interrupt = "http"`; each refusal names its key), F10, the ACP-bridge spawn row and an OpenCode attach row unchanged, and the source scan that `attaches_to_claude_code` is the one place the literal `"claude"` is compared and that validation, the driver choice and the support probe call it. Cheapest layer: a pure parse. |
| `crates/holler-body/tests/claude_attach_driver_test.rs` | 2 | `holler-body` over the test kit's `FakeClaude` as the `HarnessPort`: fail-closed attach, one delivery with no streamed text, the turn rule (idle after busy; the 5 s start wait; relaunch between prompts; relaunch during a turn is `cancelled`; a vanished process is `error`; a refused delivery is one chunk then `error`; a permission wait is `input-required` with nothing pending), the cancel rule (in flight refused naming `holler interrupt --pane NAME`, idle applies), answer refused, the call set (`locate_process`, `session_activity`, `send_prompt` only, never `login_status`, I7) and the `OP_BOUND` of a stalled call (I5). The fake is the cheapest sufficient tier: no process, no socket. |
| `crates/holler-body/tests/claude_session_test.rs` | 3 | The same session task over `SessionManager::start_with(&registry, Some(port))`: the advertised row, omitted until attached, presence through `working`, `input-required`, `working`, `idle`, the queue, the re-attach after a turn ended `error`, cancel and replace at the task, shutdown touches nothing, two rows share one port, no port refuses naming the missing configuration, and `session/rebind` at the wire (`handle_rebind`): held target swaps, unheld is `-32602` "session not found on the endpoint", mid-turn is `-32009`, a lookup past the deadline is `-32004` "endpoint did not answer". |
| `crates/holler-cli/tests/claude_attach_mode_test.rs` (+ `[[test]]`) | 4 | `holler-cli` process level, a real hub and a real `holler body run`: `say` is one inbox frame, an operator hold refuses it (exit 4) and writes nothing, an interrupt in flight is refused and the turn runs on, the roster row, a row nobody holds stays off the roster, `body support claude` is a PATH lookup, and no login is read or printed (I7). The only tier that proves `body_cmd` builds and hands over the harness. |
| `crates/holler-cli/tests/claude_rig/mod.rs` | 4, 5 | Shared rig, `#[path]`-included: the `#!/bin/sh` program that stands for `claude` (`agents --json` from a file the test rewrites; argv and environment names logged; any other command exits 2), and the unix-socket listener that stands for the inbox (`<state>/claude-inbox/<pid>`, `0700`/`0600`). |
| `crates/holler-cli/tests/say_pane_scenario_test.rs` (+ `[[test]]`) | 5 | One hub, two bodies (OpenCode on the fake server, Claude on the program and listener), two pane records through `HubPaneStore`: `say --pane` is delivered to each; an operator hold refuses both (exit 4); after a body stops, `pane-not-driven` (exit 3) for both, nothing written; a parked record refuses a bare `say <label>/<name>` (`pane-parked`) and it is delivered once lifted. Replaces the ignored step of `pane_claude/mixed.rs`. |
| `crates/holler-cli/tests/pane_claude/driven.rs` (rewritten) | 6 | In-process verbs over the fakes: `switch` is relaunch, `rebind` (after the target is of record), a second compare-and-swap of DRIVEN; `reset --first` goes through the hub (`Prompt(Check)`, `Rebind`, `Prompt(Send)`), the engine's `send_prompt` never called; no body means no drive step and DRIVEN as stored; a failed rebind is `acted` with the reconcile step, and the remedy `switch PANE <sor>` relaunches and re-points; an interrupt moves no body; `doctor` records DRIVEN and reports a body that is behind (`not-fixable`, the switch as remedy); a lost second write is healed by the next pass; a prompt between the two writes is refused and not delivered (W-5). |
| `crates/holler-pane/tests/driven_single_process_test.rs` | 6 | `driver_of` with an empty server list (by record, by stored DRIVEN, by target, none, exclusion, ambiguous). Pure. |
| `crates/holler-cli/tests/docs_claude_body_test.rs` (+ `[[test]]`) | 8, 10 | Source/docs scan: the contract is amended (ADR 0014 points 7 to 11, ADR-0021 section 5/15, `v2.md`, README, guide, CHANGELOG), none of A6's stale sentences is left (ADR, guide, CHANGELOG, module docs, `pane_claude` comments), the ignored step is gone, and the wizard's six places no longer say a Claude Code pane has no body row "until #850" or "yet (#850)". |
| `crates/holler-cli/tests/hold_single_path_test.rs` (extended) | W-2 | Source scan: exactly one `.send_prompt(` call in `holler-body`, in `claude_attach_driver*`. |

Existing tests changed, all test-only:

- `pane_claude/lifecycle.rs`: `reset_first_on_a_claude_pane_is_not_driven_and_stops_nothing` now expects the plan to ask the hub `[Advertised]` (it asked nothing before); doc updated.
- `pane_claude/main.rs`, `mixed.rs`, `rig.rs`: comments no longer name #850 as future; `mixed.rs` loses the ignored `say_delivers_a_message_to_each_pane_of_the_scenario` and its unused import.
- **`holler-body/tests/config_test.rs::presence_doc_lists_every_session_idle`** used `harness = "claude"` + `mode = "attach"` + `endpoint` + `ses_1` as an arbitrary example. The row rule refuses that row, so the example's harness id is now `codex`. **This contradicts the brief's "`config_test.rs` stays green unedited".**
- **`holler-cli/tests/pane_verbs/import/join.rs::a_pane_that_is_not_opencode_is_skipped_and_the_others_import`** built a legacy `sessions.toml` row by replacing `"opencode"` with `"claude"`. `pane import` parses that file with `holler_body::config`, so the same row is now an invalid file and blocks the import (`import-blocked`) in place of being skipped `harness-not-opencode`. The test now replaces it with `codex`. See "Findings for O" 2.
- `crates/holler-body/Cargo.toml`: `holler-pane` and `holler-pane-testkit` as dev-dependencies (F moves `holler-pane` to `[dependencies]`, the crate rule). `crates/holler-cli/Cargo.toml`: the three `[[test]]` entries. `Cargo.lock` follows.

## The contract these tests pin for F

Where the brief left a shape open, the tests fix it. F implements these names; if a name must change, T changes the test.

- `holler_body::claude_attach_driver::ClaudeAttachDriver::attach(&SessionConfig, Arc<dyn HarnessPort>) -> Result<Self, DriverError>` (async). A row nobody holds is `DriverError::NotFound`. `prompt`, `cancel`, `answer`, `pending`, `status`, `session_id`, `shutdown` have `HttpAttachDriver`'s signatures and event types.
- `SessionManager::start_with(&SessionRegistry, Option<Arc<dyn HarnessPort>>) -> SessionManager`; `start` stays.
- `SessionConfig::attaches_to_claude_code()` in `config.rs`, and the literal `"claude"` appears once in the shipped code of `holler-body`, in `config.rs`.
- The cancel refusal is `DriverError::Cancel` whose message holds `holler interrupt --pane <row name>`; `answer` is refused with a message that says "pane"; a refused delivery is exactly one `Chunk` that holds the adapter's message (it contains "behind a dialog" for both the fake and the real adapter), then `Done(Error)`; a permission or question wait is `State(InputRequired)` with `pending()` empty; busy again is `State(Working)`.
- Timing: the poll is under 2 s (a turn ends at the next ticks after idle-after-busy); the start wait is 5 s (±1 s asserted: 4 to 9 s from the write); the stalled call is bounded below 11.5 s (`OP_BOUND` 10 s); the port calls run in `spawn_blocking` (a current-thread test runtime would hang otherwise).
- Holler-cli: `body support claude` is a PATH lookup: `ok:true`, `how: "attach: claude"`, else `ok:false`, `reason: "claude not found on PATH"`. The test sets `PATH` and unsets `HOLLER_CLAUDE_BIN`, so **W-1 (whether the probe also honours `HOLLER_CLAUDE_BIN`) is not pinned either way**; O or F decides and, if the probe honours it, T adds the case.
- Pane engine: the plan of a single process runs `find_driver` with an empty server list (the store's list is read for a switch and for the exclusion rule, `list_sessions` is never called on a single process: `FakeClaude` answers `server-unsupported`); `switch`/`reset` rebind after the relaunch and write DRIVEN by a second `cas_put` only when a body was found; `reconcile` asks the hub once per pass for a single process too and records DRIVEN through `record_with`.
- **`Inner::new`**: in-crate tests `holler-body/src/session_manager/task/rebind_tests.rs` call `Inner::new(config, presence, presence_tx, name)` twice. F should keep that four-argument shape (give `Inner` the port by a setter or field), or ask T to update those two call sites. Not a pinned name; found by the throwaway run below.

## RED confirmation

All runs from the worktree; the build was warm (`cargo test --workspace --no-run`). Per target (a workspace run stops at the first uncompilable target).

| Command | Result | Why it is the right reason |
|---|---|---|
| `cargo test -p holler-body --test claude_row_test` | 6 failed, 3 passed | `parses` fails `Invalid{field:"endpoint", reason:"attach mode requires \`endpoint\`"}`; `with_an_endpoint` gets `Ok(..)` where a refusal is expected; interrupt-http, no-session-id and the bad ids are refused naming `endpoint` where the test expects `interrupt`/`session_id`; the source scan fails "config.rs defines `fn attaches_to_claude_code`". The 3 passes are pinned invariants (F10, the ACP spawn row, an OpenCode attach row). |
| `cargo test -p holler-body --test claude_attach_driver_test --test claude_session_test` | **compile error**, nothing runs | `E0432 unresolved import holler_body::claude_attach_driver` and `E0599 no function start_with on SessionManager`. The API under test does not exist and cannot be exercised through an existing one. See "RED validity" below. |
| `cargo test -p holler-cli --test claude_attach_mode_test` | 7 failed (180 s: 30 s each) | The body prints `error: config: session[0] "c1r1": \`endpoint\`: attach mode requires \`endpoint\`` and exits; each test's panic message shows the body log with that line. `body_support_claude_is_a_path_lookup`: `{"feature":"claude","ok":false,"reason":"not configured"}` against `ok:true`. |
| `cargo test -p holler-cli --test say_pane_scenario_test` | 4 failed (123 s) | The OpenCode body is up and the Claude body's log is `attach mode requires \`endpoint\``; the wait for the row `plum/cl` expires. |
| `cargo test -p holler-cli --test pane_claude` | 9 failed (8 in `driven.rs`, 1 in `lifecycle.rs`), 44 passed | e.g. `calls.hub` is `[]` where `[Advertised, Rebind]` is expected; `reset --first` is `pane-not-driven` ("no body drives demo-c1r2") where it is delivered; `doctor` finds no `shown-driven-mismatch` for a behind body; `lifecycle`: `[]` against `[Advertised]`. `an_interrupt_moves_no_body_and_writes_no_driven` passes (pinned invariant). |
| `cargo test -p holler-cli --test hold_single_path_test` | 1 failed | "exactly one HarnessPort::send_prompt call in the body: []". |
| `cargo test -p holler-cli --test docs_claude_body_test` | 3 failed, 2 passed | 19 items of the amendment missing (17 anchors, the guide's sentence, the CHANGELOG bullet); 25 stale sentences found (ADR-0021, guide, CHANGELOG, `say_cmd.rs`, `interrupt_cmd.rs`, `tx_switch.rs`, `plan.rs`, `reconcile*.rs`, `session_manager.rs`); 10 wizard contexts (`SKILL.md` x5 places, `setup-wizard.md`). The 2 passes are guards (the ignored step is gone; the wizard still derives OpenCode rows only). |
| `cargo test -p holler-pane --test driven_single_process_test` | 6 passed | A characterization: `driver_of` is #839's pure join and already works over an empty list; the drive rule relies on it. Not a RED. |
| `cargo test -p holler-cli --test wizard_scripts`, `--test pane_verbs`, `-p holler-body --test config_test` | pass | Baseline for the edited existing tests. |
| `bash scripts/lint.sh`, `bash scripts/changelog-check.sh` | exit 0 | New files are 219 to 441 lines. `cargo machete` is not installed here. |

## Validation that the suite can go GREEN

A throwaway sketch of the implementation (the driver, the row, `start_with`, the support arm, the CLI hand-over, the plan/drive/reconcile changes; **in a scratch copy under the scratchpad, outside the repository, discarded; no production code was written in the worktree**) was run against these tests to prove each is satisfiable and not flaky: `claude_row_test` 9/9, `claude_attach_driver_test` 13/13 and `claude_session_test` 12/12 (5 repeats, 12 s and 8 s), `claude_attach_mode_test` 7/7 and `say_pane_scenario_test` 4/4 (3 repeats), `pane_claude` 53/53, `pane_verbs` 407/407, `config_test` 19/19, clippy `-D warnings` clean on the changed crates' targets. Over the whole workspace the sketch broke exactly two existing tests, both repaired above, and the in-crate `rebind_tests.rs` call sites noted under "The contract".

## RED validity

- Valid RED by assertion: Acceptance 1, 4, 5, 6, 8, 10 and W-2, each failing on the feature's assertion (or on the body refusing the config it must accept), not on a typo or a missing `[[test]]` entry (the three `[[test]]` entries are declared and each target builds).
- **Compile-RED, unavoidable:** Acceptance 2 and 3 (`claude_attach_driver_test.rs`, `claude_session_test.rs`) name two new public items, the `claude_attach_driver` module and `SessionManager::start_with`. They cannot fail on an assertion before the item exists, and T may not add the skeleton. The same behaviors are also RED by assertion one layer up (4 and 5, through a real body), and the sketch run above shows both files compile and pass against the contract in "The contract". Every other target of the workspace still builds, so `cargo test -p <crate> --test <name>` runs the rest; a bare `cargo test --workspace` fails to compile until F adds the module and `start_with`.
- Tests that pass before the story (declared, each pins an invariant the design rests on): `a_claude_uuid_never_parses_as_a_routable_name`, `a_claude_spawn_row_is_still_the_acp_bridge`, `an_opencode_attach_row_is_unchanged`, `an_interrupt_moves_no_body_and_writes_no_driven`, the six cases of `driven_single_process_test.rs`, `the_ignored_say_step_of_the_scenario_is_gone`, `the_wizard_still_derives_opencode_rows_only`, and the W-6 phrase guard inside `the_sentences_a_claude_body_replaced_are_gone`.
- Real time: `a_turn_too_short_to_see_ends_after_the_start_wait` (about 5 s) and `a_stalled_port_call_is_bounded` (about 12 s) are time rules; each waits on the clock because the rule is the clock. It also holds a 2 s no-end window with a plain sleep: the assertion is that nothing happens. `a_relaunch_during_a_turn_ends_it_cancelled` replaces the process with two calls on the fake (stop, then start): a poll that lands between them would read "no process" and end the turn `error`. The window is microseconds against a 500 ms poll (about 1e-4 per run); 5 repeats did not hit it. If it ever does, the fix is an atomic replace in the fake.

## Findings for O (none blocks F)

1. The brief says `config_test.rs` stays green unedited; one case there used an incidental `claude` attach row and had to change (above).
2. `holler pane import` parses a legacy launcher `sessions.toml` with `holler_body::config`. A `claude` attach row with an `endpoint` there (which could never have worked) is now an invalid file and blocks the import, where it was skipped `harness-not-opencode`. The brief does not mention `import`. Decide whether that is the wanted behavior (the sketch keeps it), or whether the importer should skip such a pane.
3. W-1 stays open (above); W-3 and W-4 are statements for D3 and item 19, which `docs_claude_body_test` does not pin by wording (F writes them; S checks).
4. `cargo clippy --workspace --all-targets -- -D warnings` fails on this toolchain in an existing file the story does not touch (`crates/holler-cli/tests/token_load_test.rs:60`, `unneeded cloning of iterator items`). Not from #850.
5. Acceptance 7 (no hub, proto or golden change) is a diff check, not a cargo test: `git diff --stat origin/main...HEAD` must list nothing under `crates/holler-hub/src`, `crates/holler-proto/src`, or a golden file. S runs it.

## Ready for F

Confirmed: the RED is valid for F to implement against, with the one compile-RED caveat above. F may start with the amendment commit (P1), then the row (P2), the driver (P3), the CLI (P4), the drive rule (P5) and the docs and the wizard text (P7, P8). F writes no tests.
