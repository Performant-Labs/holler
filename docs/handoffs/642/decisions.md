# Decisions — #642 the OpenCode adapter (`HarnessPort`)

## A (Phase 3, up-front plan review) — 2026-10-09T14:52:40-06:00
- **Decided:** BLOCK on docs/handoffs/642-brief.md at 53237d4, with 2 blocks and 9 warns (see handoff-A.md).
  - The plan's choice of objects is otherwise right: `HarnessPort`, the closed codes and the conformance suite are reused unchanged; `FakeHarness`'s `op` strings are mirrored with no test-kit runtime dependency; the hand-rolled blocking HTTP client over `httparse` and the new stub server are justified in writing; the crate depends only on `holler-pane`, `serde_json` and `httparse`.
  - B-1: decision 7 and AC 22 forbid any ADR-0021 edit, although ADR-0021 defers `HarnessPort`'s final form to #642 by name (line 530). The brief's decisions 1, 2, 3 and 6 are contract facts the verb stories need, and the established practice, #639, #676, #692 and the sibling #640 brief, is to amend ADR-0021 in the same change.
  - B-2: `tui_target` is a bare tmux target (fed `Pane.host.tmux`) passed to `respawn-pane -k` and `display-message`. tmux prefix-matches bare targets (#641's tmux 3.7c evidence), so a missing session can resolve to another pane's: the TUI is killed and the title is read from the wrong pane. It also pushes tmux target syntax into holler-cli's wiring.
- **Assumed:** #641's brief (`.claude/worktrees/0641-host-adapter/docs/handoffs/641-brief.md`) and #640's brief are the current plans of those parallel stories. Neither has passed its own A, so their conventions (exact `=NAME` targets, removing `TMUX`/`TMUX_PANE`, ADR and `ports.rs` doc updates, `Cargo.lock` as mechanical) may still change. I used them as evidence of how the same tmux server and the same deferred ADR item are being treated, not as merged code. The issue (#642), the epic (#633), ADR-0021 and the merged test kit are the authority.
- **Hedged:**
  - W-1 (nothing stops the server `serve` starts) is a warn, not a block. The conflict lives in #641's brief, which narrows its issue's "by recorded pid and port". #642's process model already fits a stop by recorded pid. It is still the most consequential finding: with decision 2 (never adopt), #644's relaunch and the spike's wedge recovery cannot work until the MO names an owner.
  - B-1 differs from #508's W-5 (an ADR-not-updated warn) because here the ADR names this story for the decision and the brief explicitly forbids the update. The amend-first channel (a separate docs PR) is offered as an alternative.
  - W-7 (child sessions in `list_sessions`) rests on holler-body's verified reading of `GET /session` (`parentID`, issue #382), not on a run of my own.
- **Evidence:**
  - Read in full: the brief; `holler-pane/src/{ports,error,probe,argv,lib}.rs` and `pane.rs:1-140`; `holler-pane-testkit/src/{harness,lib}.rs`, `conformance/{harness,mod}.rs` and `tests/harness_conformance_test.rs`; `docs/adr/ADR-0021.md`; `docs/research/opencode-pane-spike.md`.
  - Also read: `herdr-api-spike.md:120-180`, `holler-cli/src/pane/wiring.rs:1-80`, `holler-body/src/http_attach_driver.rs:1-112` and `connection.rs:66-90, 300-330`, `holler-hub/src/ws_handshake.rs` (`httparse` use), `scripts/lint.sh`, `clippy.toml`, `scripts/spikes/opencode-lib.sh:1-80`, `.github/workflows/ci.yml` (test steps), and the 641 and 640 briefs.
  - Issues read with `gh issue view`: 642, 633, 641, 644, 649, 639, 683.
  - Git: `git show --stat` of 9d61c9f, 316b8e3, 434a1b5 and 90997a2 (each added a CHANGELOG entry); the ADR-0021 diffs of 316b8e3 and 2a6f349; `git log -- docs/adr/ADR-0021.md`. `Cargo.lock:1127-1128`.

## A (Phase 3, up-front plan review: a re-review of the amended brief; this run is 642a) — 2026-10-09T17:27:08-06:00
- **Decided:** PASS on docs/handoffs/642-brief.md at 5a4d68e, with 0 blocks and 8 warns (see handoff-A.md, which replaces the BLOCK at 4831580).
  - Both earlier blocks are resolved. B-1's ADR-0021 edit moves to 642b, the closing part, as #640's split does (part 3, 640-brief.md:22). B-2's exact targets are 642b's code, pinned by AC 11a-11c and 19a.
  - 642a stays consistent with existing patterns. It implements `HarnessPort` with `NotImplemented` for the three TUI methods, the skeleton's documented answer (error.rs:404-406). It depends only on `holler-pane`, `serde_json` and `httparse`, and it justifies `http.rs`, `exec.rs`, the `tui.rs` types and the stub in writing.
  - W-1 is the warn with the most consequence. This brief's decision 14 says "#644 passes the session name (and directory)", and #644's brief (C-9, at 7195993) rejects that, so #649 gets opposite instructions. `workdir` (642a) can be met by wiring alone, because `ensure_session(name, cwd)` precedes `serve` in #644's act. `tui_session` (642b) has no such source, given #644's C-8.
- **Assumed:** #644's brief (7195993), #641's (95e2260), #663's (ec3a214) and #640's part 2 brief are the current plans of those in-flight stories, not merged code. #644's brief was amended while I reviewed (d2636ba to 7195993); the lines I cite are the new ones, and their substance did not change. The issue (#642), ADR-0021, the merged test kit and #640 part 1 (3bdd129) are the authority.
- **Hedged:**
  - W-1 is a warn, not a block. For 642a, the `Resolver` doc already says meeting the precondition is wiring's job, which matches #644; only decision 14's prose and the Forward-compat row disagree. Whether `tui_session` can be met at all is 642b's question.
  - W-3 (contract facts on main before the ADR note) is a warn, because #640's split defers its ADR edits the same way and ADR-0021.md:533 stays true while #642 is open.
  - W-6 assumes that `cargo machete` (bnjbvr/cargo-machete@main, default flags) checks dev-dependencies. Either way, an unused `tempfile` breaks the manifest rule "declare only what is consumed".
  - The outside-model mismatch (`deepseek-v4-pro` in the brief and the usage files, against `glm-5.3-flash` in CLAUDE.md) is a process note for O, outside A's dimensions.
- **Evidence:**
  - Read in full: the brief at 5a4d68e; the earlier handoff-A.md and decisions.md; `holler-cli/tests/attach_cli_test/fake_server.rs`; `holler-adapter-herdr/{Cargo.toml,src/lib.rs}`.
  - Read in part:
    - `holler-pane/src/{lib,ports,pane,probe}.rs` and `error.rs:398-410`; `holler-pane-testkit/src/harness.rs:1-160` and `287-310`; `conformance/harness.rs:8-22`.
    - `docs/adr/ADR-0021.md:1-200`, `473-536`; `docs/research/opencode-pane-spike.md:224-290`.
    - The workspace `Cargo.toml:1-40` and `150-170`; `ci.yml:305-325`; `holler-cli/src/pane/wiring.rs:1-40`; `holler-cli/tests/support/mod.rs:740-830`.
    - The 644 brief (C-4 to C-9, the launch and relaunch act tables, decisions 6-7, Forward-compat, Follow-ups, Risks); the 641 brief (grep: `TmuxSocket`, `exec.rs`, the #642 row, `stop_owned`); the 663 brief (grep: runner, #696, `kill -s`); the 640 part 2 brief (header, `HerdrConfig`, transport).
  - Issues read with `gh issue view`: 642, 644, 633 (grep), 695, 696. I searched open and closed issues for a `FakeHarness` parity follow-up and found none.
  - Git: `git diff --stat 9d61c9f 3bdd129` over `holler-pane`, the harness fake and suite, and this crate (empty); `git show 3bdd129 -- CHANGELOG.md`; `git ls-files docs/handoffs | grep 642`; the usage.json files of rounds 1-3; and a grep of the brief and handoffs for personal infrastructure names (none).

## T (Phase 4, author / RED; this run is 642a) — 2026-10-09T17:33:29-06:00
- **Decided:** RED is valid (PASS). 27 tests in `tests/hermetic_test.rs` (+ the `tests/support/stub.rs` stub); 26 fail on
  assertions about the missing behaviour, 1 (`ac11_the_adapter_is_send_sync_and_static`) is a compile-time pin that holds by
  the stub's construction (see handoff-T-red.md).
  - Landed the brief's public API as stubs with no behaviour (the brief's Test plan allows it; #640 part 1's T-red did the
    same) so the RED is a runtime assertion failure, not a compile error. `http::request`'s stub returns
    `Ok(Reply { status: 0, .. })`, not `Garbled`, so no AC 1 case passes before F's code.
  - AC 8's "the message holds the exit status" is pinned with `sh` running a scratch `serve` script that exits 42 (the port is
    removed from the message before the check, so an OS-assigned port cannot satisfy it); `false` is kept for the brief's
    "exits at once, returns well before `call`" half. The same script's marker pins "nothing spawned" for the healthy and
    frozen ports and "runs in the pane's project directory" for the exit case.
  - AC 5 also checks the Behaviour rule that a `400` from the existence check reads as `404`; AC 2 also checks a
    `{"healthy":false}` 200. Both are one extra loop iteration or assertion, not separate tests.
- **Assumed:**
  - The brief's AC 24 port rule ("no test reads a port outside 48100-48199") does not bind the hermetic stub, which the brief
    itself puts on `127.0.0.1:0`; A's W-5 says the same. No fixed port appears in the tests.
  - `std::env::temp_dir()` for scratch dirs is not "a real HOME" (AC 24). `tempfile` is not declared (A's W-6).
  - `sh`, `false` and `touch` are on `PATH` on both CI runners (ubuntu-latest, macos-latest).
- **Hedged:**
  - The timing bounds are upper bounds only, with 300 ms of slack (ACs 1, 2, 5, 7, 8). They do not assert that a frozen call
    waits its full timeout, so a loaded runner can only make them slower, never flakier the other way.
  - The `create_session` request sequence is pinned exactly (POST then PATCH). A later change that adds a request on the
    success path would have to update AC 3's test; that is the brief's stated sequence.
- **Evidence:**
  - Read: the brief in full (1-1106), handoff-A.md, decisions.md; `holler-pane/src/{error.rs:380-520,lib.rs,pane.rs}`;
    `holler-pane-testkit/src/harness.rs:30-70` (`HarnessOp`), `fault.rs:19` (`PortOp`);
    `holler-cli/tests/attach_cli_test/fake_server.rs` (the stub's cousin); `holler-adapter-herdr/Cargo.toml` and
    `tests/protocol_test.rs:1-20`; the workspace `Cargo.toml:1-40`, `clippy.toml`, `scripts/lint.sh`; #640's T-red
    (`git show 09ada91`) and #643's for the stub precedent.
  - Ran: `cargo test -p holler-adapter-opencode` (1 passed, 26 failed, all on assertions);
    `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings` (clean); `rustfmt --check --edition 2021` (clean);
    `bash scripts/lint.sh` (exit 0); `cargo machete` (clean); a throwaway raw-socket self-check of the stub (deleted) and a
    shell check that `sh serve ...` exits 42 in its cwd and `false serve ...` exits 1.
