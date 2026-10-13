# Handoff-F: Phase 5 - #850: a body driver for a Claude Code pane

**Date:** 2026-10-12
**Branch:** issue-850-implementation (worktree `.claude/worktrees/0850-claude-body`, on T's RED commit `b20dff0`)
**Issue:** #850

## What was done

Production code, docs and the wizard's text; no test was written, edited or deleted. Nothing is committed or staged (the
Workflow script commits).

- `crates/holler-body/Cargo.toml`: `holler-pane` moved from the dev-dependencies to `[dependencies]` (the crate rule);
  `holler-pane-testkit` stays a dev-dependency.
- `crates/holler-body/src/config.rs`: `SessionConfig::attaches_to_claude_code` and the one private comparison of a
  harness id with `claude` it delegates to; the attach branch of validation calls it and goes to `claude_row::check` or
  to `check_http_attach` (the OpenCode rule moved out of `parse` unchanged, so `parse` stays under clippy's 100 lines);
  a grammar-comment pointer to ADR 0014 point 7.
- new `crates/holler-body/src/config/claude_row.rs`: the row rule (no `endpoint`, no `interrupt = "http"`, a
  `session_id` in the lower-case 8-4-4-4-12 UUID form), each refusal naming its key.
- new `crates/holler-body/src/claude_attach_driver.rs`: `ClaudeAttachDriver` with `HttpAttachDriver`'s public shape
  (`attach`, `prompt`, `cancel`, `answer`, `pending`, `status`, `session_id`, `shutdown`), `OP_BOUND` (10 s) and the
  one `spawn_blocking` port-call helper.
- new `crates/holler-body/src/claude_attach_driver/turn.rs`: the turn rule (delivery, the 500 ms poll, the 5 s start
  wait, how a turn ends), and the body's one `.send_prompt(` call.
- `crates/holler-body/src/lib.rs`: `pub mod claude_attach_driver`.
- `crates/holler-body/src/session_manager/driver.rs`: the `ClaudeAttach` variant, one arm in each method, and
  `spawn_or_attach(config, claude)`, which refuses a Claude row when the body was handed no harness.
- `crates/holler-body/src/session_manager.rs`: `SessionManager::start_with(&registry, Option<port>)`; `start` calls it
  with `None`; the `rebind` doc no longer says only the HTTP driver re-points.
- `crates/holler-body/src/session_manager/task.rs` (wiring only, +8 lines, 802): `Inner` holds the optional port, set by
  `Inner::with_claude` so `Inner::new` keeps its four arguments (the in-crate `rebind_tests.rs` call sites); the three
  `spawn_or_attach` calls pass it, and `rebind` attaches through `Driver::spawn_or_attach` and stores the `Driver` it
  returns.
- `crates/holler-body/src/connection.rs` (+1 line, 875): `run(state_root, registry, claude)` passes the port to
  `start_with`.
- `crates/holler-body/src/query.rs`: the support rule, `claude_code_support` (a `PATH` lookup of
  `holler_pane::pane::CLAUDE_CODE_COMMAND` through `resolvable`).
- `crates/holler-cli/src/body_cmd.rs`: `holler body run` builds the harness with the wiring's `claude_harness` only when
  a row attaches to a Claude Code process, warns once on stderr when it cannot be built, and passes it to
  `connection::run`.
- `crates/holler-cli/src/pane/wiring.rs`: `claude_harness` is `pub(crate)`; `process_env` (the lookup `connect` used
  inline) is a named `pub(crate)` function, so `body run` reads the environment the same way.
- `crates/holler-cli/src/say_cmd.rs`, `interrupt_cmd.rs`: module docs and the `INTERRUPT_TEXT_UNSUPPORTED` doc (A6's
  stale list).
- `crates/holler-pane/src/tx_switch/plan.rs`: a single process's plan finds its body with `find_driver` over an empty
  server list and gives `check_first` that driver.
- `crates/holler-pane/src/tx_switch.rs`: `relaunched_and_driven`, the drive step for a single process after the relaunch
  (`drive`, then a second `cas_put` at the relaunch's generation); the module doc's single-process paragraph and port-call
  count.
- `crates/holler-pane/src/reconcile.rs`, `reconcile/observe.rs`, `reconcile/observe/process.rs`: `read_bodies` asks the
  hub for any pane in scope; `process::check` takes the pass's `Bodies`, runs `check_driven` and records with
  `record_with`; the `record` helper that pinned DRIVEN went; module docs.
- Docs: `docs/adr/ADR-0014.md` (A2), `docs/adr/ADR-0021.md` (A1, A3, A4), `docs/protocol/v2.md` (A5), `README.md`
  (A6 recipe), `docs/pane-control.md` (A6), `CHANGELOG.md` (the new `## [Unreleased]` bullet and the #831 bullet's
  sentence).
- The wizard's six places (N1, P8), text only: `agent-skills/herdr-setup/SKILL.md` (Stage 0 intro, the connections list,
  Stage 3, Stage 8, Stage 10) and `docs/setup-wizard.md` (item 5). No script and no Stage 8 step changed.
- `docs/handoffs/850/evidence.md`: eight facts of unchanged code the change relies on.

## Design decisions

- **The poll reads activity each tick and asks who holds the session only when the turn ends (W-3).** Each 500 ms tick is
  one `session_activity` (one `claude agents --json` run) of the pid the message was written to; `locate_process` runs
  once more when the process reads idle (same pid: `end_turn`; another pid: `cancelled`; none: `error`) or when a read
  fails (another pid: `cancelled`; anything else: `error`). The alternative, a lookup and an activity read every tick
  (T's sketch), costs two runs a tick for the same outcomes. A `/clear` in the pane mid-turn is caught at the end of the
  turn (no process holds the session: `error`), as the rule asks. Stated in the driver's module doc.
- **A failed or timed-out port call ends the turn `error`** (fail closed), and the message is never written again. The
  outer `OP_BOUND` stops the wait, not the blocked thread (W-3). Folded into ADR 0014 point 9 as one closing sentence.
- **A refusal before delivery other than the adapter's own** (no process holds the session at delivery, the lookup
  failed or timed out) ends `error` with no chunk: the rule gives a chunk only for the adapter's refusal.
- **`cancel` with no turn in flight returns the last turn's stop reason** (`Cancelled` when none ran), as
  `HttpAttachDriver::cancel` does (issue #238), so a cancel racing the end of a turn never invents `cancelled`.
- **`answer` is `DriverError::Unsupported`**, which the wire maps to `-32602` (E: `send_answer_refusal`): a request the
  driver cannot serve, not a connection problem. The session task already answers "nothing pending" while the row is not
  `input-required`.
- **No port, no attach:** a Claude row in a body started with `start` (no harness) fails its attach with
  `DriverError::Startup` naming what `holler body run` needs; the session task logs `attach_failed` and retries, as for any
  attach row.
- **`Inner::with_claude`** instead of a fifth `Inner::new` argument: keeps T's in-crate `rebind_tests.rs` call sites
  as they are (T's note) and the task file's growth to wiring only.
- **The validation calls the same predicate the method does.** Validation runs on the raw row, before a `SessionConfig`
  exists, so the comparison is a private `attaches_to_claude_code(mode, harness)` in `config.rs`, which the public method
  delegates to: one comparison, one literal.
- **W-1, the support probe: `PATH` only, by choice.** Honouring `HOLLER_CLAUDE_BIN` in `holler-body` would copy the CLI's
  variable name into the body or thread a resolved path through `local_support`/`local_caps` and both callers. The
  divergence (a body whose `claude` is named by the variable alone answers `ok:false` while it delivers) is written into
  `docs/protocol/v2.md` section 5.3 and the README recipe. T's test does not pin either way.
- **W-4** (remove a closed pane's row) is in ADR-0021 item 19 and the README recipe; **W-5** (the window between the two
  writes is a refusal) is in item 19 and `tx_switch.rs`'s module doc; **W-6** is folded into ADR 0014's Consequences
  bullet ("the operator writes the row afresh from the pane's record (`holler pane get NAME`)").

## Reuse / extend-vs-new

Extended, as the brief's Reuse map says: the `Driver` enum and its one choice point (`spawn_or_attach`), the session
task (queue, presence, crash isolation, `rebind`), `DriverEvent`/`DriverEventStream::new`/`DriverError`/`Status`/
`StopReason`, `HttpAttachDriver`'s public shape, the wiring's one `claude_harness` construction, `query::resolvable`,
`CLAUDE_CODE_COMMAND`, `HarnessEndpoint::Process`, `PaneName::parse`, and on the pane side `find_driver`, `drive`,
`check_first`, `check_driven` and `record_with`, each reused as written. New objects are the brief's own: the
`claude_attach_driver` module (and its `turn` child) and `config/claude_row.rs`. The driver copies no SSE or poll loop of
`HttpAttachDriver`.

## Architecture notes for A

- New crate edge `holler-body -> holler-pane` (normal dependency; ADR-0021 section 5's new bullet). No edge to an adapter.
- Public API: `holler_body::claude_attach_driver::ClaudeAttachDriver`, `SessionManager::start_with`,
  `SessionConfig::attaches_to_claude_code`; `holler_body::connection::run` gains a third parameter (one caller,
  `body_cmd`). In `holler-cli`, `wiring::claude_harness` and `wiring::process_env` are `pub(crate)`.
- No file under `crates/holler-hub/src` or `crates/holler-proto/src`, and no golden file, changed. No new log event,
  error code, wire field or method.
- The pane engine: a single process's switch/reset now makes `hub.advertised` (always), the store's list (switch, as
  before), `hub.rebind` and a second `cas_put` (with a body); a reconcile pass of single processes alone now asks
  `hub.advertised` once.

## Deviations from spec / wireframe

- **ADR 0014 point 9 wording:** "A process waiting on a permission prompt or a dialog makes the row `input-required`"
  became "... a permission prompt or a question ...". The adapter reports a question (`input needed`) as
  `HoldingQuestion` and a dialog (`dialog open`) as the refusal `harness-dialog-open`, not an activity; the driver maps
  the first to `input-required` and treats the second as a failed read (`error`). One sentence was also appended to
  point 9 (the 10 s bound and never re-sending, W-3).
- **ADR-0021 item 19:** "(A6)" (a reference into the brief) reads "the README's recipe ("A Claude Code pane")", and a
  sentence on removing a closed pane's row (W-4) and one on the refusal window (W-5) were added.
- **The guide's sentence** (A6 (1)) says the row is one "the operator writes from the pane's record" rather than "written
  from the registry" (the W-6 reasoning: no step of Holler derives it).
- **The contract was not a separate commit before the tests:** T's RED commit already holds the tests; the docs land in
  this phase's change. The amendment's text is the brief's, with the folds above.
- The README recipe's command reads `holler pane launch NAME --harness claude --port-policy none` (the documented
  commands test parses every one).

## Tier 1 self-check (incl. tests now GREEN)

- `cargo build -p holler-cli`: clean, no warning.
- `cargo test -p holler-body --test claude_row_test --test claude_attach_driver_test --test claude_session_test --test
  config_test`: 9/9, 13/13, 12/12, 19/19.
- `cargo test -p holler-cli --test claude_attach_mode_test --test say_pane_scenario_test --test hold_single_path_test
  --test pane_claude --test docs_claude_body_test --test wizard_scripts --test docs_pane_control_test --test
  docs_cli_test`: 7/7, 4/4, 6/6, 53/53, 5/5, 308/308, 21/21, 3/3.
- `cargo test -p holler-pane`: every target ok (`driven_single_process_test` 6/6).
- The CI command, `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`: exit 0, 2934
  passed, 0 failed, 38 ignored.
- `bash scripts/lint.sh`: exit 0 (warnings only for files over 600 lines that this story does not create; new files 59,
  158 and 222 lines; `task.rs` 802, `connection.rs` 875).
- `bash scripts/changelog-check.sh`: ok.
- `cargo clippy --workspace --all-targets --keep-going -- -D warnings`: the one error is the existing
  `crates/holler-cli/tests/token_load_test.rs:60` (`unneeded cloning of iterator items`), untouched by #850 and noted by T;
  every other target is clean.
- `cargo machete`: not installed here (as T found). `holler-pane` is used by `holler-body`'s shipped code.

## Evidence appendix

`docs/handoffs/850/evidence.md`

## Tests that look wrong (for T)

None.

## Known issues

- **The wizard's Stage 8 still says to write "one `[[session]]` row per OpenCode pane, and nothing else" to the body
  config it owns, and to re-read it for only such rows.** N1 keeps Stage 8 as it is, so an operator who adds a Claude
  Code row by hand to that file contradicts the stage's own check, and a wizard rerun rewrites the file without it. The
  six places now say the operator writes the row by hand and the wizard derives none; where the row lives in a
  wizard-built fleet is for O or a follow-up story (the brief's "A later story may derive Claude rows").
- `pane import` of a legacy `sessions.toml` with a `claude` attach row that names an `endpoint` now blocks as an invalid
  file instead of skipping it (T's finding 2); kept as is, for O.
- The clippy error in `token_load_test.rs` predates this story and fails `--all-targets` on this toolchain.

## Files changed

- `crates/holler-body/Cargo.toml`
- `crates/holler-body/src/config.rs`
- `crates/holler-body/src/config/claude_row.rs` (new)
- `crates/holler-body/src/claude_attach_driver.rs` (new)
- `crates/holler-body/src/claude_attach_driver/turn.rs` (new)
- `crates/holler-body/src/lib.rs`
- `crates/holler-body/src/connection.rs`
- `crates/holler-body/src/query.rs`
- `crates/holler-body/src/session_manager.rs`
- `crates/holler-body/src/session_manager/driver.rs`
- `crates/holler-body/src/session_manager/task.rs`
- `crates/holler-cli/src/body_cmd.rs`
- `crates/holler-cli/src/pane/wiring.rs`
- `crates/holler-cli/src/say_cmd.rs`
- `crates/holler-cli/src/interrupt_cmd.rs`
- `crates/holler-pane/src/tx_switch.rs`
- `crates/holler-pane/src/tx_switch/plan.rs`
- `crates/holler-pane/src/reconcile.rs`
- `crates/holler-pane/src/reconcile/observe.rs`
- `crates/holler-pane/src/reconcile/observe/process.rs`
- `docs/adr/ADR-0014.md`, `docs/adr/ADR-0021.md`, `docs/protocol/v2.md`, `docs/pane-control.md`, `docs/setup-wizard.md`
- `README.md`, `CHANGELOG.md`, `agent-skills/herdr-setup/SKILL.md`
- `docs/handoffs/850/handoff-F.md`, `docs/handoffs/850/evidence.md`, `docs/handoffs/850/decisions.md` (run records)
