# Handoff-A-dup: Phase 7 - #642a the OpenCode adapter, server side  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-642-implementation
**Diff base:** 3bdd129 (`origin/main`, the merge base)   **Diff head:** 884d772 (T-green round 2)
**Reuse map:** docs/handoffs/642-brief.md, "Reuse map (extend, do not duplicate)" (lines 935-968)
**Verdict:** PASS

## Summary

PASS. The diff extends what the Reuse map says to reuse, and every new object it builds is one the map justifies in
writing. `HarnessPort`, `PaneError` and its codes, `PaneName`, `PaneId` and the conformance suite are used unchanged; no
file of `holler-pane` or the test kit is touched. The new objects are `OpenCodeHarness`, `OpenCodeConfig`, the blocking
`http` client over the workspace's `httparse`, the private `exec.rs` with only the kill path, the `tui.rs` types mirroring
#641, and the test stub. Each is the map's "new, justified". `server.rs` is the brief's own named split of `lib.rs`.

I searched for the near-copies the map did not name. I found no parallel path, only per-crate private helpers where no
shared one can be reached.

There are two warns:

- **D-1:** two of the written justifications are inaccurate. Their conclusions still hold.
- **D-2:** the kill runner differs from #641's in two ways, which #696 should record.

Round 2's rework added no new object and no drift.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| D-1 | warn | `src/http.rs:9-10`; `tests/support/stub.rs:5-6` | Two written justifications are inaccurate, but the conclusions hold. **(a) The HTTP client.** `http.rs` says "the only OpenCode clients in the repo are async (`reqwest` on `tokio`, in `holler-body`)", as the brief's map does (brief:948-949). That is not quite true. `holler-body/src/query.rs:239-269` (`probe_one`) is a blocking client over `std::net::TcpStream`. It is private and makes one read of at most 128 bytes (line 260). It returns only whether the status is 2xx, and it answers `false` for a refusal and for a timeout alike. It could not have been reused: `holler-body` is a layer this adapter must not depend on (ADR-0021 section 5), and the probe reads no body. **(b) The stub.** `stub.rs` says its cousin "lives under another crate's `tests/`, so it cannot be imported from here". In fact, three targets include `holler-body/tests/http_attach_driver_test/fake_server.rs` across crates through `#[path]`: `holler-cli/tests/debug_trace_test.rs:39`, `holler-cli/tests/attach_mode_test.rs:30` and `holler-load-test/src/main.rs:55`. Neither cousin fits, though. holler-body's fake runs on `tokio`, which would be a new dev-dependency here. It answers `{}` to `GET /session/:id` and `POST /session`, where the adapter requires the id (lines 286-306), and it has no frozen, raw or chunked mode. holler-cli's fake serves two fixed routes and panics on any other status (line 149). An adapter's tests that pulled in the CLI's test tree would also invert the layering. | Docs only, at the next edit of these files (642b's F), not in this cycle. In `http.rs`, name `query.rs`'s status probe and why it does not fit. In `stub.rs`, change "cannot be imported" to "could be included by `#[path]`, but neither cousin fits", with the reasons above. |
| D-2 | warn | `src/exec.rs:17`, `51-53` | The kill runner will not consolidate with #641's "as a move", as `exec.rs:7-8` and brief decision 11 claim. #641's `TmuxHost` (its branch at a5ea942, `lib.rs:38` and `67-71`) takes the `kill` binary as configuration (`with_kill_binary`), so that its tests can use a fake `kill` that records its arguments (641-brief.md:377). It also runs `kill` under `LC_ALL=C` and reads its stderr: the `-s 0` probes read `No such process` (641-brief.md:261, 303-313, 435). This crate's `exec.rs` hardcodes `kill` on the adapter's own `PATH` and discards stdout and stderr (the outside diff gate's round 2, W-4). That is correct for 642a, which reads only the exit status. | No change in 642a. Record both differences on #696, so that the shared runner takes the binary as configuration and captures stderr. 642b's tmux calls classify stderr (brief:612-616), so 642b's `exec.rs` will need the capture anyway. |

No duplication otherwise; the extension is clean. Round 2's rework is confined to one private counter field in
`http.rs`'s `Reader` and to doc comments in `server.rs` and `tui.rs`. The cap test reuses the stub's `raw` mode: there is
no second stub and no test-only knob, as Phase 3's N-3 asked.

### Carried forward from Phase 3 (handoff-A.md at c052d02)

| # | Status now |
|---|---|
| N-2 | Still open for 642b. This cycle's rework took `hermetic_test.rs` from 704 to 775 lines with T's two gate tests. That is still under the brief's 800-line split point and lint's 900-line failure, so N-2's plan stands: 642b's builder and parser tests go in a new test target. `lib.rs` is unchanged at 483 lines. |
| N-1, W-1 | Unchanged, for the 642b amendment. |
| W-3(2), W-4(c) | Unchanged, for O before the PR. For W-4(c), there is still no `FakeHarness` parity issue: `gh issue list --search FakeHarness` finds only #638 and #684, both closed. |

### Checked, consistent with existing patterns (no finding)

- **Reused as is.** `git diff --name-only 3bdd129 HEAD` touches nothing in `holler-pane` or `holler-pane-testkit`. The
  adapter returns only existing `PaneError` variants (`Unavailable`, `Timeout`, `SessionNotFound`, `NotImplemented`).
- **`http.rs`.**
  - The workspace has no blocking HTTP crate: `reqwest` has no `blocking` feature, and there is no `ureq`.
  - `httparse` is reused.
  - The hub's `ws_handshake.rs` parses a request, while this parses a response. The hub is also a layer the adapter must
    not depend on.
  - `Cargo.lock` adds only this crate's dependency list. The manifest mirrors `holler-adapter-herdr`'s.
  - The tests reuse `http::request` and write no second client.
- **`excerpt` and `one_line`.**
  - This is the third private `excerpt`. `holler-pane`'s (`error.rs:688`) is `pub(crate)` in a frozen crate, and the herdr
    adapter keeps its own (`protocol.rs:580`). So per-crate private copies are the established pattern.
  - The form differs on purpose. This one cuts at 60 bytes and turns control characters into spaces, so AC 11d's 200-byte
    bound holds. The other two quote with `{:?}`, which can expand one control byte to six characters (`\u{1b}`).
- **The other new helpers.** `deadline_after`, `budget`, `session_path`, `kill_group`, `Resolver` and `ProcessEnv` have
  no equivalent on `main`. #640 part 2's branch (e0254e3) keeps its own `deadline_after` (`adapter.rs:434`) and reply cap
  (`transport.rs:38`), the same per-adapter pattern.
- **`TmuxSocket`** matches #641's variants and derives exactly (its branch at a5ea942, `lib.rs:21-30`).
- **Within the crate.**
  - `health` and `serve`'s boot poll share `healthy` and `is_healthy`.
  - `exec.rs` holds only the kill path.
  - Every child module uses the crate root's deadline and message helpers instead of a copy.
  - `serve` does not use `Call`, because its first GET treats a refusal as success, which `Call::send` maps to
    `unavailable`. The brief's Behaviour section gives the two mappings separately.
- **The test stub** is the map's "new, justified" (see D-1 for the wording). `closed_port` mirrors holler-cli's
  `unreachable_endpoint` by the stub's stated convention.
- **The hub and CLI harness candidates** (`StateDir`, `wait_for`, `Hub`, `Body`, `mint_token`, `join`) cannot be reached
  from this crate:
  - `holler-cli/tests/support/mod.rs` resolves `env!("CARGO_BIN_EXE_holler")` at compile time (line 142), so it cannot be
    included by `#[path]` outside holler-cli.
  - The test kit exports no such helper.
  - `Scratch` and the inline wait in the group-kill test are test-local.
  - Scratch dirs are split across the repo (`tempfile` in 10 files, `env::temp_dir()` in 14), so there is no dominant
    pattern to drift from.
- **The `op` strings** are mirrored constants, because production code cannot depend on the test kit. The AC 5, 7 and 8
  tests pin them to `HarnessOp::as_str`.
- **Public repository.** The added lines outside `docs/handoffs/` hold no personal infrastructure names. Nothing depends
  on the crate: only doc comments in `holler-pane` and the test kit name it.

## Notes for F

None (PASS). D-1 is for 642b's F, at the next edit of `http.rs` and `stub.rs`.

## Notes for O

1. D-2: add the two runner differences (a configurable `kill` binary, and captured stderr under `LC_ALL=C`) to #696.
2. Still open from Phase 3, before the PR:
   - file the `FakeHarness` parity follow-up (W-4(c));
   - the PR-body lines for N-1 (the agent part waits on #700) and for W-3(2) (divergence 15(a), #695, and the ADR-0021
     note landing with 642b);
   - the AI disclosure.

## Patterns referenced

- `crates/holler-adapter-opencode/{Cargo.toml,src,tests}` at 884d772.
- `crates/holler-body/src/query.rs:197-284`; `crates/holler-body/tests/http_attach_driver_test/fake_server.rs`;
  `crates/holler-cli/tests/attach_cli_test/fake_server.rs`; `crates/holler-cli/tests/support/mod.rs:62-165`.
- `crates/holler-pane/src/error.rs:686-694`; `crates/holler-adapter-herdr/src/protocol.rs:575-588` and its `Cargo.toml`.
- The #641 branch at a5ea942 (`holler-adapter-host/src/lib.rs:21-90`; `641-brief.md:261, 303-313, 377, 435`) and the
  #640 branch at e0254e3 (`adapter.rs`, `transport.rs`).
- `docs/handoffs/642-diff-result-r2.md` (W-4); issues #695, #696 and #700.
