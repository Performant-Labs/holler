# Handoff-F: Phase 5 - #640 part 2 of 3: the socket transport, `HerdrAdapter` and a simulated Herdr

**Date:** 2026-10-09
**Branch:** issue-640-implementation (base `61d0e8e`, T-red PASS)
**Issue:** #640 (epic #633), part 2 of 3. The PR says `Part of #640`.

| Field | Value |
|-------|-------|
| GitHub issue | #640 |
| Working branch | `issue-640-implementation` |
| Build plan phase | part 2 of 3 (brief "Scope" table) |
| Input documents read | `docs/handoffs/640-brief.md` (all 1831 lines), `handoff-A.md`, `handoff-T-red.md`, `decisions.md`; `src/{lib,protocol,layout,plan,transport,adapter}.rs`; every test file and `tests/wire_herdr/{mod,serve}.rs`; `holler-pane` `ports.rs`/`error.rs`; `holler-pane-testkit` `conformance/herdr.rs`, `herdr.rs`, `fault.rs`; the `checked_add` deadline sites in `holler-hub/src/panes/store.rs:268` and `holler-pane-testkit/src/feed.rs:201` |
| Acceptance criteria count | 43 (AC 1-43, AC 12a included; AC 14 is T's mutant run at GREEN) |
| Handoff document path | `docs/handoffs/640/handoff-F.md` |

No confirmation wait: this is a Workflow run with no human at this phase. The operator's standing rule is to decide and report.

## What was done

- `crates/holler-adapter-herdr/src/transport.rs`: `UnixSocketTransport::exchange` is filled in. Each exchange runs on a worker thread named `herdr-exchange`: it connects, writes the request line, reads the reply line and closes. The caller waits with `recv_timeout` until the one deadline. A private `Exchange` carries the socket, the method and the deadline, and builds every message. Module docs added. One doc-only fix to the pinned trait doc: `"herdr.<method>"` became `` `herdr.<method>` ``, because rustdoc read `<method>` as an unclosed HTML tag (`RUSTDOCFLAGS=-D warnings cargo doc` failed on T's stub too). No signature changed.
- `crates/holler-adapter-herdr/src/adapter.rs`: `connect`, `connect_with` and all seven `HerdrPort` methods are filled in. T's `#[allow(dead_code)]` on `transport` is removed. Private helpers: `deadline`, `call`, `supported_server`, `grid`, `extent_of`, `workspace_grid`, `place`, `confirm`, plus free functions `grid_tab`, `changed_under`, `validate`, `deadline_after` and `usage`. Private struct `WorkspaceGrid`. `#[derive(Debug)]` on `HerdrAdapter`. Module docs added.
- `CHANGELOG.md`: the AC 43 entry under `[Unreleased]` / Enhancements, right after the part-1 entry, linking #633, #649 and #640.
- `docs/handoffs/640/evidence.md`: 9 source facts for the diff gate (below).

## Design decisions

1. **The transport, as Decision 3 says.** The caller checks the time left first: a passed deadline is `Timeout` with no thread and no connection (AC 11). Then it spawns the worker and waits with `recv_timeout(deadline.saturating_duration_since(now))`.
   - `Timeout` from the wait is `Timeout { op: "herdr.<method>" }`.
   - `Disconnected` (no answer) is `unavailable`.
   - The worker computes the time left before each socket call. Zero is `Timeout` with no call made. Otherwise it sets `SO_RCVTIMEO`/`SO_SNDTIMEO` to that value.
   - `WouldBlock`/`TimedOut` is the same `Timeout`, and `Interrupted` is retried.
   - The worker sends at most one answer, with `let _ = answer.send(..)`. A failed send (the caller stopped waiting) is silent. Nothing in the worker can panic: no indexing, no `Instant + Duration`, no unwrap.
2. **Write loop instead of `write_all`.** The write timeout is reset to the time left before every `write` syscall. `write_all` would reuse one timeout across its internal syscalls. `Ok(0)` is `unavailable` (`WriteZero`). See Deviations.
3. **Bounded read.** The reply is read in chunks of at most 64 KiB into a buffer capped at `READ_LIMIT = MAX_REPLY_BYTES + 1`.
   - A newline within those bytes gives a line of at most `MAX_REPLY_BYTES`.
   - `READ_LIMIT` bytes with no newline is `unavailable`, naming the limit in decimal (`16777216`, AC 8), and reading stops.
   - Bytes after the first newline are ignored. End of stream with some bytes returns them (AC 9). End of stream with none is `unavailable` (AC 7). Non-UTF-8 is `unavailable` (AC 10).
4. **Connect.** `Interrupted` is retried while time remains. Each attempt is a fresh socket from `UnixStream::connect`. Any other error is `unavailable`, naming the path and the `io::ErrorKind` in its `Display` form ("entity not found", "connection refused", "invalid input parameter" for a path that is too long, AC 3/4).
5. **Quoting in messages.**
   - Paths, labels and Herdr pane ids are quoted with `{:?}`, so every message stays on one line whatever the text holds (ADR-0021 §9). The `Debug` form of an ordinary path contains its display form, which is what AC 3 and AC 27 check.
   - The version string is never quoted by the adapter: the `version()` form-check message names no text.
   - No message carries the request line or reply bytes (AC 12).
   - Alternative considered: `path.display()`, as the hub's store messages do. Rejected because it can break the one-line rule.
6. **`ensure_pane` shape (Decision 11).**
   - Order: the deadline is taken on entry, then the session and the configured workspace are checked, with no request (AC 23).
   - `workspace_grid` sends `session.snapshot`, then `SessionState::workspace`. When the workspace exists it sends `layout.export` of the grid tab and runs `grid_of`; otherwise the result is an empty `GridMap` with `tab: None`.
   - Then `plan_splits` with a one-cell `Target`, and a slice match: `[]` is the occupant, `[step]` is `place` then `confirm`, and more steps are `unavailable`.
   - The `pane.split` result goes through `changed_under`, which rewrites only `PaneNotFound` into `unavailable` (AC 17). Every other error passes unchanged.
   - `confirm` re-exports the grid tab: either the tab `parse_workspace_created` returned, or the one read before the split. `position_of(new id)` must equal `spec.grid`. Otherwise it is `unavailable`, naming the id and where it landed (AC 16), and nothing is closed or moved.
7. **Three states that cannot happen** each get an `unavailable` rather than `unreachable!` (denied):
   - an empty plan for a cell with no pane;
   - a `Split` whose `from` has no pane or whose workspace has no tab;
   - a plan of more than one step.

   `plan.rs` rules each of them out (evidence entries 2-4).
8. **The version gate (Decisions 7 and 8).** `connect_with` runs validate, then `ping` + `parse_pong` + `check_supported` under one deadline, and keeps nothing. `version()` sends its own ping, re-gates, and then refuses an empty version or one with a control character as `unavailable`. `connect` does not run that form check, because Decision 7 lists only ping, parse and gate.
9. **A finding 3 adopted.** Validation refuses, after the zero check and before the workspaces, a `timeout` for which `Instant::now().checked_add(timeout)` is `None`: `usage`, naming `timeout`. Every call takes its deadline with `checked_add`, so no `Instant + Duration` can panic anywhere. See Deviations.
10. **A finding 1 not adopted (kept Decision 3).** Port methods return the transport's `Timeout` unchanged, so `op` names the wire method (`herdr.layout.export`, `herdr.pane.split`, `herdr.ping`), as the brief and T's transport tests pin. `FakeHerdr`/`PortOp` name the port method (`herdr.ensure_pane`). That divergence is listed under Known issues for the operator and the part-3 ADR rows.
11. **A finding 2 followed without changing part 1.** There is no third `excerpt` and no edit to `protocol.rs`, because the brief freezes part 1. Herdr-sent text goes through `{:?}` (decision 5).
12. **A finding 4(b) applied (docs only).** The doc comments of `Transport`, `connect` and `connect_with` say the trait and `connect_with` are a test seam, and that production builds an adapter only through `connect`.

## Reuse / extend-vs-new

The brief's Reuse map is followed row by row:

- **Requests, decoding and the gate:** every request is a part-1 `protocol::Request`, decoded by `decode_reply`. The parsers are `parse_pong`, `check_supported`, `parse_snapshot` with `SessionState::workspace`, `parse_layout_export`, `parse_workspace_created`, `parse_pane_info`, `parse_read` and `expect_ok`. There is no `json!` in `src/adapter.rs` or `src/transport.rs` (AC 36).
- **Tree to `GridPos`:** `layout::grid_of` and `GridMap::{at, position_of, cells}`, the one tree walk.
- **The step:** `plan::plan_splits` with `Target`, `Extent` and `Step`. Range and reachability are not re-checked.
- **The port:** `holler_pane::{HerdrPort, HerdrSpec, HerdrPane, HerdrSnapshot, Key, PaneId, PaneError}`, with no parallel error type.
- **The socket client:** follows `holler_hub::control::send_over` as a pattern only, with no dependency, and closes its three gaps: an unbounded connect, a per-read timeout and an unbounded `read_line`.
- **New objects:** two new modules, which the brief itself specifies (Files table; ADR-0021 §5). The only new private types are `transport::Exchange` and `adapter::WorkspaceGrid`, and each is the state of one call.

## Architecture notes for A

- **Layers:** `adapter` uses `transport`, `protocol`, `layout` and `plan`; `transport` uses `protocol::Request`. There is no cycle and no new normal dependency: `cargo tree -e normal --depth 1` lists only `holler-pane` and `serde_json`. Nothing outside `crates/holler-adapter-herdr/**` and `CHANGELOG.md` changed. Part-1 sources and every test file are untouched.
- **Public surface:** the pinned items, plus `#[derive(Debug)]` on `HerdrAdapter`. No pinned signature changed.
- **Pattern for #642 (A finding 6):** this is the first bounded-exchange code in an adapter: a worker thread per exchange, per-syscall timeouts from one deadline, `recv_timeout`. `DEFAULT_TIMEOUT` is the first code constant for I5's 10 s. If #642 needs the same, A's note stands: raise a shared home rather than copy it.
- **No state:** the only fields are `config` and `transport`, and there is no `Mutex`, `Cell` or `OnceLock` in `adapter.rs` (AC 37).

## Deviations from spec / wireframe

1. **Decision 6, one more `usage` case:** a timeout too long to add to the clock (A finding 3). It refuses only configs that would otherwise panic. No test pins it.
2. **Decision 2 says "writes `request.to_line()` with `write_all`"**, and Decision 3 says the worker sets the write timeout "before each syscall". F follows Decision 3 with its own write loop. The bytes on the wire are identical, and partial writes are handled the same way.

Wireframe: N/A (no UI).

## Tier 1 self-check (incl. tests now GREEN)

`cargo test -p holler-adapter-herdr --no-fail-fast`: all 96 pass, all 34 new tests GREEN.
```
tests/adapter_conformance_test.rs  test result: ok. 4 passed; 0 failed
tests/adapter_test.rs              test result: ok. 17 passed; 0 failed
tests/layout_test.rs               test result: ok. 10 passed; 0 failed
tests/plan_splits_test.rs          test result: ok. 19 passed; 0 failed
tests/protocol_test.rs             test result: ok. 33 passed; 0 failed
tests/transport_test.rs            test result: ok. 13 passed; 0 failed; finished in 1.00s
```
The three new targets were run 8 more times in a row, and all were green every time (transport 13/13, adapter 17/17, conformance 4/4).

Gates:
```
cargo fmt --check -p holler-adapter-herdr                    exit 0
cargo clippy -p holler-adapter-herdr --all-targets -D warnings  clean
cargo clippy --workspace --all-targets -- -D warnings          exit 0
RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-herdr --no-deps   clean (not a CI gate)
bash scripts/lint.sh                                          exit 0 (only the >600-line warnings, on part-1/T test files and other crates; src/adapter.rs is 445 lines, src/transport.rs 286)
bash scripts/changelog-check.sh                               changelog-check: ok
cargo machete                                                  no unused dependencies
cargo tree -p holler-adapter-herdr -e normal --depth 1        holler-pane, serde_json only
grep -n "holler-hub" crates/holler-adapter-herdr/Cargo.toml   (nothing)
AC 35-39 greps                                                 each printed nothing
cargo test --workspace --no-fail-fast                          1414 passed, 5 ignored, 4 failed (see Known issues)
```

## Evidence appendix

`docs/handoffs/640/evidence.md`, with 9 entries:

1. `decode_reply`'s `pane_not_found` mapping (`protocol.rs:279-296`).
2. `plan_splits`' empty plan for an occupied cell (`plan.rs:75-86`).
3. `CreateRoot` only on an empty grid (`plan.rs:113-117`, `:132-143`).
4. `grid_of` gives at least one row (`layout.rs:145-149`, `:171-185`).
5. `parse_workspace_created` always sets `grid_tab` (`protocol.rs:486-497`).
6. Duplicate labels in `SessionState::workspace` (`protocol.rs:358-368`).
7. `parse_read`'s cut to `max_lines` (`protocol.rs:506-509`, `:592-595`).
8. The I5 bound on `HerdrPort` (`holler-pane/src/ports.rs:118-123`).
9. The conformance suite filters by label (`holler-pane-testkit/src/conformance/herdr.rs:394-401`).

One fact rests on std rather than on this repo: std refuses a zero socket timeout, and the brief states this in Decision 3. Handling it needs no evidence entry, because the worker never asks for a zero timeout and reports `Timeout` instead.

## Tests that look wrong (for T)

None. One gap T may close at GREEN, optional: no test pins the A-finding-3 refusal. A ninth AC 27 case, `HerdrConfig { timeout: Duration::MAX, .. }`, would expect `Usage` naming `timeout` with no request recorded.

## Known issues

1. **`cargo test --workspace` on this machine: 4 failures in `holler-cli --test logging_test`.** The failing tests are `banner_names_resolved_level_and_format`, `debug_flag_beats_env`, `env_none_loses_to_flag_noisy` and `log_output_stays_off_stdout`.
   - Cause: each one runs the real `holler roster` binary and expects exit 1 because no hub is reachable. A live hub runs on this machine, so the command succeeds ("Unexpected success", code 0).
   - Unrelated to this diff: `holler-cli` does not depend on `holler-adapter-herdr` (`crates/holler-cli/Cargo.toml` has no such line), and nothing in the CLI changed.
   - What it means for GREEN: the suite needs a machine without a reachable hub (CI), or T records the same reason.
2. **Timeout `op` divergence (A finding 1, kept per Decision 3).** Under a real Herdr, a verb test that asserts `timed out: herdr.ensure_pane` against `FakeHerdr` would see `herdr.session.snapshot`, `herdr.layout.export`, `herdr.pane.split` or `herdr.workspace.create`. This is for the operator, next to Decision 20, and for part 3's ADR §9/§10 rows (A finding 7 item 3).
3. **Herdr-sent text in messages is quoted with `{:?}` and not cut to 64 characters** (A finding 2, under the part-1 freeze). Every message stays on one line, but a pathological pane id or label from Herdr would make a long one. Making `protocol::excerpt` `pub(crate)`, a visibility-only edit, would close this. It needs the brief's freeze lifted.

## Files changed

- `crates/holler-adapter-herdr/src/transport.rs`
- `crates/holler-adapter-herdr/src/adapter.rs`
- `CHANGELOG.md`

Handoff artifacts, not production code: `docs/handoffs/640/handoff-F.md`, `docs/handoffs/640/evidence.md`, and an appended entry in `docs/handoffs/640/decisions.md`.
