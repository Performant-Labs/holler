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

## F (Phase 6, implement; this run is 642a) — 2026-10-09T18:02:58-06:00
- **Decided:** `done`. T's 27 hermetic tests are GREEN (from 1 passed / 26 failed), with no test edited. The workspace is clean
  under clippy `-D warnings`, rustfmt, `lint.sh`, `changelog-check.sh` and `cargo machete`, and `cargo test --workspace`
  passes with the hub state dir isolated (see handoff-F.md).
  - `serve` and its boot poll are in `src/server.rs` from the start, by the brief's own split rule, so `lib.rs` is 483 lines.
    `src/exec.rs` holds only the bounded runner and `kill -s KILL -- -<pgid>` (A's W-6).
  - Messages name id routes as `/session/:id`, so no message echoes a caller's id, and a reply-shape message stays near 140
    bytes (AC 11d's bound is 200).
  - A connect failure other than a refusal or a timeout is `HttpError::Garbled`, not `Refused`, so `serve` never starts a
    server on a port it cannot then check. The variant's doc comment is widened to say so; the variants are unchanged.
  - The CHANGELOG entry follows A's W-5: it describes part 1, and says part 2 brings the opt-in real-OpenCode tests.
- **Assumed:**
  - `kill` is on the adapter's own `PATH` on both CI targets, and procps-ng's form `kill -s KILL -- -<pgid>` is the BSD
    `kill`'s too. The 641 brief records the macOS form; I verified Linux (procps-ng 4.0.4) only.
  - OpenCode accepts `POST /session/<id>/abort` with `Content-Length: 0` and no body. The spike's `curl -X POST` sent no body
    at all. Only 642b's real-OpenCode rig can confirm this.
- **Hedged:**
  - The deadline kill and `serve`'s success path have no hermetic test in 642a. I exercised them once from a throwaway
    program against a fake server (python `http.server`, a server that never answers), and the output is in handoff-F.md.
    T may pin them.
  - The 4 `holler-cli` `logging_test` failures in the first workspace run come from a live local hub answering
    `holler roster`. All 11 of those tests pass with `HOLLER_STATE_DIR` isolated, and nothing depends on this crate. CI has
    no hub, so it does not see them.
  - Issue #642 was amended on 2026-10-09 (the OpenCode agent, depending on #700). The amendment is outside the brief and
    642a, so it is flagged for O, not built.
- **Evidence:**
  - Read: the brief (1-1106), handoff-A.md, handoff-T-red.md, this file, issue #642 (`gh issue view`; #695, #696, #700
    titles); `holler-pane/src/{lib,ports,pane}.rs`, `error.rs:380-710`; `holler-pane-testkit/src/harness.rs`,
    `conformance/harness.rs:1-50`; `holler-adapter-herdr/{Cargo.toml,src/lib.rs}`; `holler-hub/src/ws_handshake.rs:80-140`;
    `httparse-1.10.1/src/lib.rs` (`Response::parse`, `parse_chunk_size`, `parse_version`); the workspace `Cargo.toml:1-80`,
    `clippy.toml`, `scripts/lint.sh`, `scripts/changelog-check.sh`, the `CHANGELOG.md` `[Unreleased]` section;
    `scripts/spikes/opencode-{lib,api}.sh` (`oc_http`, the abort calls); `docs/research/opencode-pane-spike.md:50-80,
    185-205`; ADR-0021:95-110, 175-190, 329, 525-540; the 641 brief's `exec.rs` lines.
  - Ran: `cargo test -p holler-adapter-opencode` (27/27, 8 repeats plus `--test-threads=1`);
    `cargo clippy --workspace --all-targets -- -D warnings`; `rustfmt --check`; `cargo doc` with `-D warnings`;
    `bash scripts/lint.sh`; `cargo machete`; `cargo test --workspace` twice (once with the live hub reachable, once isolated);
    `cargo tree -i holler-adapter-opencode`; a `setsid` group kill through `/usr/bin/kill`; and the scratch self-checks quoted
    in handoff-F.md.

## T (Phase 7, verify GREEN + Tier 2) — 2026-10-09

- **Decided:**
  - GREEN is valid. F changed no test, and the 27 RED tests all pass. Five hand mutations of `src/` (M1-M5 in
    handoff-T-green.md) each failed exactly the test that targets them.
  - F's fragility note on the AC 8 "42" check is applied. The check now asks for `42` as a whole number in the message,
    so a scratch path (`hlr642-...`), a pid or the port can no longer satisfy it.
  - One test is added: `serve_kills_its_process_group_when_the_deadline_passes`. It pins the brief's Behaviour rule for
    `serve` (kill the group on the deadline, then answer `timeout`), the one specified branch with a real leak cost that
    had no test. The success path is left to 642b's real rig, since only a child that answers HTTP reaches it.
  - Verdict PASS. Nothing needs F's production code to change.
- **Assumed:**
  - CI's macOS leg has `kill`, `sh` and `sleep` on `PATH` and supports `kill -0`. These are standard on GitHub's macOS
    images.
  - An orphaned background `sleep` is reaped by init (or a subreaper) within 2 s of the group kill.
- **Hedged:**
  - The workspace run was done with `HOLLER_STATE_DIR` isolated only, as CI runs with no live hub. I did not repeat F's
    non-isolated run.
  - Whether BSD `kill` accepts `kill -s KILL -- -<pgid>` is unverified here. The new test will show it on the macOS leg,
    and any fix belongs in `exec.rs`.
- **Evidence:**
  - Read: handoff-F.md, handoff-T-red.md, the brief's AC list (712-887) and its Behaviour section (618-712),
    `evidence.md`, `.github/workflows/ci.yml` (the matrix and the run steps), and `src/{lib,server,exec}.rs` at the
    mutation points.
  - Ran:
    - `cargo test -p holler-adapter-opencode` (27/27, then 28/28; 12 runs at once and one `--test-threads=1` run);
    - the M1-M5 mutations, each restored with `git checkout`;
    - `cargo clippy --workspace --all-targets -- -D warnings`, `rustfmt --check`, and `cargo doc` with `-D warnings`;
    - `bash scripts/lint.sh`, `bash scripts/changelog-check.sh`, `cargo machete` and `bash scripts/test-hooks.sh`;
    - `cargo test -p holler-cli --test wire_selftest` and `--test docs_cli_test`;
    - `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` with `HOLLER_STATE_DIR`
      isolated (126 suites, 1410/0/5);
    - the AC 22 `git diff --name-only`, the `Cargo.lock` diff, the `unsafe` count and the AC 24 grep.

## A (Phase 3, up-front plan review: a re-review on the rework route after the outside diff gate's BLOCK; this run is 642a) — 2026-10-09T18:28:09-06:00
- **Decided:** PASS on docs/handoffs/642-brief.md, unchanged since 5a4d68e, with 0 blocks and 3 new warns. The 8 warns of
  80901fa are carried forward with their status (see handoff-A.md, which replaces the PASS at 80901fa).
  - F's architecture change stays within the brief. The private `server.rs` is the brief's own split, the private `exec.rs`
    holds only the kill path, `HttpError` keeps its variants and the bounds are private. The public API is the brief's.
  - N-1: issue #642 (amended at 17:40 MDT, after the brief and my last PASS) and epic decision 8 make this adapter apply
    `Pane.opencode_agent`. No part of the brief covers it, and it cannot land before #700. Only the serve-time
    `default_agent` route is this crate's layer: a per-prompt agent belongs to the hub's `send_prompt` and `holler-body`'s
    driver, not to `HarnessPort`.
  - N-2: 642b's file plan predates 642a's sizes (`lib.rs` 483, `hermetic_test.rs` 704). 642b's additions would put the test
    file past lint's 900-line failure.
  - N-3: the diff gate's B-1 does not hold, as measured. Pin the cap with one test through the stub's `raw` mode, and add no
    test-only knob.
- **Assumed:** This pass came from the diff-gate BLOCK with F's `archChanged` (`coding-pipeline-logic.mjs:1127`), not from a
  fresh run: there are no round-4 brief-gate files, and the only new artifact is `642-diff-result-r1.md`. #641's brief at
  0f18b80 and #644's at 7195993 are those stories' current plans.
- **Hedged:**
  - N-1 is a warn, not a block. 642a uses no agent field, `AgentKey` is not on `main`, and 642a's resolver shape takes the
    agent additively. It must be planned before 642b.
  - The OpenCode facts in N-1(b) are strings in the installed 1.18.35 bundle, not observed behaviour. I started no OpenCode
    server.
  - N-2's 642b line counts are estimates at 642a's density.
- **Evidence:**
  - Read in full: the brief; the earlier handoff-A.md (80901fa), handoff-F.md and this file; `src/{lib,server,http,exec,tui}.rs`;
    `642-diff-result-r1.md`.
  - Read in part:
    - `tests/support/stub.rs:1-150`, the `hermetic_test.rs` test list and `handoff-T-green.md:150-177`;
    - ADR-0021 §11 (437-453) and the 641 brief (569, 718-736);
    - the playbook driver (`coding-pipeline-logic.mjs:1030-1150`, `coding-pipeline.workflow.mjs:1268-1317, 4596-4726`).
  - Issues read with `gh issue view`: 642, 700, 633 (decision 8), 695 and 696. Also `gh issue list --search FakeHarness`.
  - Ran:
    - `git diff --stat 5a4d68e HEAD -- docs/handoffs/642-brief.md` (empty), `git ls-remote origin` (main 3bdd129) and
      `bash scripts/lint.sh`;
    - a grep of the branch's added lines for personal names (none), and greps of the installed OpenCode bundle
      (`default_agent`, `OPENCODE_CONFIG*`);
    - a scratch program outside the repo (its own workspace and target dir, deleted afterwards). It called `http::request`
      on replies of 200 MiB, 64 MiB + 1 and 64 MiB - 1000 with no framing headers.

## T (Phase 4, author RED: round 2, the rework after the outside diff gate's round 1) — 2026-10-09T18:32:14-06:00
- **Decided:**
  - PASS: the test contract is valid. Two tests are added and two are extended in `hermetic_test.rs` (704 to 775 lines).
    - The 64 MiB cap on the unframed read-to-the-close path is pinned with over and under cases through the stub's `raw`
      mode. This is B-1 and A's N-3. There is no test-only knob and no second stub.
    - A chunked vector with an extension and a trailer (NV-2).
    - A counting `workdir` resolver in the two `serve` tests where the port is in use or frozen (NV-4: the resolve comes
      after the health check).
  - Every new assertion passes on the current code, by design: the behaviour exists, and A measured the cap. Validity rests on
    mutations M1-M4 (handoff-T-red.md), each failing exactly the targeted test. The new tests are reported as answers to
    the gate, not as RED for F.
  - NV-1 is answered with evidence, not a new test. The `httparse` 1.10.1 doc example is in `evidence.md`.
  - NV-3 and the pgid NV-4 are answered by the existing group-kill test.
  - The new tests stay in `hermetic_test.rs`, not in a new `http_test.rs`. A second target would compile `support/stub.rs`
    again with parts unused, which would need a `dead_code` allow. 775 lines is still under the brief's 800 split point.
    N-2's 642b test home is a new target.
- **Assumed:**
  - About 200 MB of transient memory for the 64 MiB case is acceptable on the GitHub Linux and macOS runners.
  - The diff gate's `server.rs` and `http.rs` line numbers are off by the file's real size (`server.rs` is 186 lines). I
    matched its findings to the code by content.
- **Hedged:**
  - The cap test's peak RSS is estimated from the buffer sizes, not measured. A measured 68 MB for the client alone.
  - The trailer test pins the brief's "trailers discarded" only for a reply on a closed connection. That is the only kind
    this client makes (`Connection: close`).
- **Evidence:**
  - Read: `642-diff-result-r1.md` (whole), handoff-A.md (re-review), the brief's API and Behaviour sections (498-669),
    `src/http.rs` (whole), `src/server.rs:30-70`, and `tests/{hermetic_test.rs,support/stub.rs}`.
  - Read in the `httparse` 1.10.1 registry source: `lib.rs:1251-1340`.
  - Ran:
    - `cargo test -p holler-adapter-opencode`: 28/28 before, 30/30 after, and six concurrent runs at 30/30;
    - mutations M1-M4, each restored with `git checkout`;
    - `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings`;
    - `rustfmt --check --edition 2021`;
    - `bash scripts/lint.sh` (exit 0).

## F (Phase 6, implement: round 2, the rework after the outside diff gate's round 1) — 2026-10-09T18:44:50-06:00
- **Decided:** `done`, with archChanged false. All 30 hermetic tests still pass, and no test was edited.
  - B-1: the 64 MiB bound in `http.rs` now counts every byte read off the connection (heads, chunk framing and body). Once
    the total passes the cap, a read is refused before its bytes are kept, so the read buffer never holds more than the cap. A's
    N-3 showed that the old check already stopped within one 8 KiB read. The change makes the bound exact and the doc
    literally true. It stays in `http.rs`, `MAX_REPLY` stays private, and there is no test-only knob (N-3).
  - NV-2, W-1 and W-2 are wording fixes only:
    - a trailer is never read;
    - boot tries start at least 150 ms apart, and a longer try is followed at once;
    - the request sends no other header.
  - A's N-1(e): `tui.rs` no longer calls `OpenCodeConfig` final.
  - NV-3 and NV-4: two evidence entries quote std's `CommandExt::process_group` and `Child::id` (Rust 1.98.1).
  - W-3, NIT-1, NIT-2 and the B-2 the gate withdrew: no change. handoff-F.md gives the reasons.
- **Assumed:**
  - The gate's line numbers do not match the files (T found the same), so I matched its findings to the code by content.
  - Counting a reply's head and framing toward the cap is acceptable, because no OpenCode reply comes near 64 MiB. The cost
    is a largest readable body a little under 64 MiB.
- **Hedged:**
  - The std excerpts come from the local `rust-docs` component's rendered source (1.98.1), outside the repo, so the gate
    cannot attach them. `process_group` and `Child::id` are stable APIs, so a newer stable on CI does not change them.
  - The macOS form of `kill -s KILL -- -<pgid>` stays unverified until CI's macOS leg runs T's group-kill test.
  - I left the early `timeout` in `settled` (less than one poll interval left) as it is. A final poll would have a request
    budget of about zero and would time out anyway.
- **Evidence:**
  - Read:
    - `642-diff-result-r1.md` (whole), handoff-A.md, handoff-T-red.md, handoff-T-green.md, this file, evidence.md, the brief
      (whole), `src/*.rs` (whole), `tests/hermetic_test.rs:1-300` and `tests/support/stub.rs` (whole);
    - the playbook's F block and commit step (`coding-pipeline.workflow.mjs:3048-3121, 4774-4800`) and `dual-review.sh`'s
      excerpt and evidence handling (894-1110);
    - the rendered std source of `os/unix/process.rs` and `process.rs` (rust-docs 1.98.1);
    - issue #642 (last updated 17:40:07 MDT, A's N-1 amendment, nothing newer).
  - Ran:
    - `cargo test -p holler-adapter-opencode` (30/30 before and after);
    - `cargo clippy --workspace --all-targets -- -D warnings`, `rustfmt --check`, and `cargo doc` with `-D warnings`;
    - `bash scripts/lint.sh`, `scripts/changelog-check.sh`, `cargo machete`, the AC 24 grep and the `unsafe` count;
    - `rustc --print cfg --target aarch64-apple-darwin`;
    - `cargo test --workspace` with `HOLLER_STATE_DIR` isolated;
    - a scratch probe of the bound's edges and peak memory, outside the repo and deleted afterwards (output in
      handoff-F.md).

## T (Phase 7, verify GREEN: round 2, after F's rework for the outside diff gate's round 1) — 2026-10-09T18:51:32-06:00
- **Decided:**
  - PASS. The suite is 30/30 GREEN on 9fc44f6, and Tier 1 and Tier 2 are clean.
  - F's round-2 `taken` count in `http.rs` is pinned from both sides. Disabling it (M6) or counting 1 MiB low (M7) each
    fails only `ac1_an_unframed_reply_past_64_mib_is_garbled_and_one_under_it_is_read`.
  - No test was changed in this round. F's "42" fragility note was already addressed in round 1.
- **Assumed:**
  - The isolated workspace run (`HOLLER_STATE_DIR` set to scratch) is the faithful form of CI's run, because this machine
    runs a live hub.
- **Hedged:**
  - The macOS group-kill form remains unverified until CI's macOS leg runs.
- **Evidence:**
  - Read: handoff-F.md (round 2), handoff-T-red.md, handoff-T-green.md (round 1), and
    `git diff 4c9b710 9fc44f6 -- crates/`.
  - Ran:
    - `cargo test -p holler-adapter-opencode` (30/30), 8 concurrent runs plus one `--test-threads=1` run;
    - mutations M6 and M7, each restored with `git checkout`;
    - lint, changelog-check, `cargo clippy --workspace --all-targets -- -D warnings`, rustfmt, rustdoc `-D warnings`,
      machete and test-hooks;
    - the isolated `cargo test --workspace` (1413/0/5), `wire_selftest` and `docs_cli_test`;
    - the AC 24 grep and the `unsafe` count.

## A (Phase 7, anti-duplication gate; this run is 642a) — 2026-10-09T19:00:00-06:00
- **Decided:** PASS on 3bdd129..884d772, with 0 blocks and 2 warns (see handoff-A-dup.md).
  - Every object the Reuse map says to reuse is used unchanged. No file of `holler-pane` or the test kit is touched.
    Every new object is the map's "new, justified", and `server.rs` is the brief's own split.
  - D-1: two written justifications are inaccurate, though the conclusions hold.
    - `holler-body/src/query.rs:239` is a blocking OpenCode probe, so "the only OpenCode clients are async" is not quite
      true. The probe is private, reads 128 bytes, returns a bool, and sits in a layer the adapter must not depend on.
    - holler-body's fake server is included across crates by `#[path]` in three targets, so "cannot be imported" is not
      true either. Neither cousin fits, though.
  - D-2: `exec.rs` hardcodes `kill` and discards its stderr. #641's `TmuxHost` takes the binary as configuration and
    reads stderr under `LC_ALL=C`, so #696 is not quite "a move". Both differences should be recorded on #696.
  - Round 2's rework added no new object and no drift: one private counter in `Reader`, plus doc comments.
- **Assumed:**
  - #641's branch at a5ea942 (T-red stubs) and #640's at e0254e3 (A-dup and S PASS, unmerged) are those stories' current
    shapes. Neither is on `main`, so neither could have been extended here.
  - Including holler-cli's `tests/support/mod.rs` by `#[path]` outside holler-cli fails to compile, because it calls
    `env!("CARGO_BIN_EXE_holler")` (line 142). I read this from the code and did not compile it.
- **Hedged:**
  - D-1 is a warn, not a block. The brief justified both new objects in writing, and the missed analogs could not have
    been reused.
  - D-2 is a warn. #641's runner is not written yet (T-red stubs only), so its final shape may change.
  - The third private `excerpt` is not a finding. Per-crate copies are the pattern (`holler-pane`'s is `pub(crate)`, and
    the herdr adapter has its own), and this one's byte cut is what keeps AC 11d's 200-byte bound.
- **Evidence:**
  - Read in full: the brief; handoff-A.md, handoff-F.md and this file; `src/{lib,server,http,exec,tui}.rs`, `Cargo.toml`,
    `tests/hermetic_test.rs` and `tests/support/stub.rs`; `642-diff-result-r2.md`; both cousin fake servers; holler-pane's
    `lib.rs` and `probe.rs`; and the herdr adapter's `lib.rs` and `Cargo.toml`.
  - Read in part: `holler-body/src/query.rs:190-290`; `holler-cli/tests/support/mod.rs:36-165`;
    `holler-pane/src/error.rs:660-710`; `holler-adapter-herdr/src/protocol.rs:1-80` and `560-595`; and the #641 branch's
    `lib.rs:1-95` and brief (a grep for `kill`, lines 375-379 and 433-436).
  - Ran:
    - `git diff --stat` / `--name-only` from 3bdd129, and `git diff 1c3ae10 HEAD -- crates`;
    - the diffs of `Cargo.lock`, `Cargo.toml` and `CHANGELOG.md`;
    - repo-wide greps for `one_line`/`excerpt`/sanitizers, deadline helpers, percent-encoding, process-group kills,
      `#[path]`, blocking `TcpStream` in `src`, `Arc<dyn Fn`, `env_clear`, and `tempfile` against `env::temp_dir()`;
    - the workspace dependency table;
    - the 640, 641 and 663 branches' crate diffs and new helpers;
    - `gh issue view` 695, 696 and 700, and `gh issue list --search FakeHarness`;
    - a grep of the added lines outside `docs/handoffs/` for personal infrastructure names (none).
