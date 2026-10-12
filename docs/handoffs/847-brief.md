# Brief: #847 remote-admin-last-seen-flake

Repo: Performant-Labs/holler. Issue: #847. Rigor: in-session. UI surface: no. Kind: test-only (root cause is in the test, not production code).

## Problem
`remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen` (#508, AC 4 second half) failed once in the full workspace run of #649 (`docs/handoffs/649/handoff-T-green.md`: `last_seen` 1791759181 -> 1791759182) and passed on re-runs. The test measures "admin traffic did not move `last_seen`" by sleeping 2 s and comparing two whole-second readings, while a real `holler body run` is alive on the same token and legitimately moves `last_seen` itself. The assertion is a wall-clock window over an uncontrolled writer.

## Root cause (reproduced, not a hypothesis)
**The hub is correct; the test is wrong.** `Roster::touch` runs for every inbound text frame on a body's own socket, and the real body sends frames on its own schedule:

```
crates/holler-hub/src/circuit.rs:831-835
        if let Some(token_id) = self.registry.token_id_for_client(self.client_id).await {
            self.roster.touch(&token_id, env.method().unwrap_or(""));
        }
```
```
crates/holler-hub/src/roster.rs:390-399
    pub fn touch(&self, token_id: &str, _method: &str) {
        ...
            row.last_seen = now;
```
```
crates/holler-body/src/connection.rs:612-613  (the live loop's select!)
            _ = self.heartbeat.tick() => Beat::Presence(send_presence_or_drop(...)),
            changed = self.presence_changed.recv() => presence_changed_beat(...)
```
The body sends `session/presence` every `heartbeat_interval()` (15 s default, `HOLLER_HEARTBEAT_INTERVAL_MS`) AND immediately on any session state change (`connection.rs:543-550`). Either lands in the test's window, whose real length is the 2 s sleep plus two `roster --server` and `hub status --server` CLI spawns plus the `before`/`after` `roster` spawns; under a loaded workspace run those spawns take seconds, so the window grows and a presence frame (a late state-change push after the rig's "idle" wait, or the 15 s tick) can fall inside it. Because `last_seen` is whole seconds, one frame is enough to flip the value.

The admin path is verified innocent: `crates/holler-hub/src/circuit/admin.rs:5-8` states and the code keeps "never touches Registry body-slot bookkeeping or Roster"; `grep touch` in `admin.rs` is empty. WS Ping/Pong on the body socket do not touch (`circuit.rs:663`, only `last_frame_at`).

**Deterministic reproduction (run in this worktree, 3 of 3 RED):**
`HOLLER_HEARTBEAT_INTERVAL_MS=300 cargo test -p holler-cli --test remote_admin_test remote_admin_traffic_never` fails every time at `remote_admin_test.rs:380` ("admin/roster and admin/status traffic must never touch the body's last_seen"), against production code that is unchanged. Without the env var it passes (2.7 s). That is the whole bug: the test only passes when the real body happens to be quiet.

## Evidence (verbatim)
```
crates/holler-cli/tests/remote_admin_test.rs:364-387
#[test]
fn remote_admin_traffic_never_moves_the_bodys_last_seen() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);

    let before = roster_json(&hub_state);
    let last_seen_before = before["rows"][0]["last_seen"].as_u64().expect("last_seen is a number");

    std::thread::sleep(Duration::from_secs(2)); // last_seen is whole seconds; make a moved value observable.

    let roster_out = run(&body_state, &["roster", "--server", &ws_url, "--json"]);
    ...
    let after = roster_json(&hub_state);
    let last_seen_after = after["rows"][0]["last_seen"].as_u64().expect("last_seen is a number");
    assert_eq!(last_seen_before, last_seen_after, "AC 4: ...");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}
```
Seams the fix reuses (all in `crates/holler-cli/tests/`):
- `support/raw_ws.rs:254` `pub async fn live_socket(state, hub, token_id, hostname, role) -> WsClient`: authenticates with the joined credential and completes the hello with the given `role`; `role: "body"` makes a hand-rolled socket a body that sends nothing unless told to.
- `remote_admin_test.rs:175-185`: the exact `session/presence` JSON and the `futures_util::SinkExt::send(&mut ws, Message::text(..))` call used to put a row in the roster.
- `support/remote_admin_rig.rs:26-43` `rig_with_env` (what `two_dir_rig` wraps: `Hub::start_with_env`, `mint_token`, `join`, `write_sessions_toml`, `Body::start`, wait idle). The new test replaces the `Body::start` step with a `live_socket` body.
- `support/mod.rs:159` `wait_for`; `support/mod.rs:681` `Body::start_with_env` (not needed by the chosen fix).

## Acceptance criteria
1. `remote_admin_traffic_never_moves_the_bodys_last_seen` no longer contains `std::thread::sleep` and no longer runs a real `holler body run`: the "body" is a hand-rolled `live_socket(.., "body")` that sends exactly one `session/presence` and then nothing, so the only writer of that row's `last_seen` after the baseline read is whatever admin traffic does. Check: `grep -n "thread::sleep" crates/holler-cli/tests/remote_admin_test.rs` shows no hit inside that test, and the test does not call `Body::start`.
2. The assertion states the invariant: after one `roster --server` and one `hub status --server`, plus a second of each, the row's `last_seen` equals the baseline AND its `conn_state` is still `connected` and the socket was not superseded (reuse the existing AC 3 style check). It does not depend on elapsed time to make a move observable: a regression is made observable by the mutant in AC 4, not by sleeping.
3. The test passes N=30 consecutive runs under load (see Test plan), with zero failures.
4. Mutation proof (T runs it, restores it, and records it in handoff-T-red): adding `self.roster.touch(...)` (or `roster.advertise`) to the admin request path in `crates/holler-hub/src/circuit/admin.rs` makes the new test fail. To make that mutant observable without a sleep, the new test must make time pass on the hub's own clock between the baseline and the check: it waits on an observable second boundary as D2 describes.
5. No production file changes: `git diff origin/main --stat` lists only `crates/holler-cli/tests/remote_admin_test.rs` (and, if D3 is taken, `crates/holler-cli/tests/support/remote_admin_rig.rs`) plus `docs/handoffs/`.
6. `cargo test -p holler-cli --test remote_admin_test` is 21/21 and `cargo clippy -p holler-cli --all-targets -- -D warnings` is clean.

## Files
Production: none.
Tests: `crates/holler-cli/tests/remote_admin_test.rs` (rewrite the one test); optionally `crates/holler-cli/tests/support/remote_admin_rig.rs` (D3).
Reuse map (extend, do not duplicate): `live_socket`, the existing presence JSON literal at `remote_admin_test.rs:175`, `roster_json`, `run`, `hub_status_json`, `rig_with_env` internals. Do not add a second presence-JSON builder if one extraction serves both tests (the AC 4 first-half test already builds the same literal).

## Decisions already made (MO)
- D1. "Assert the invariant, not a duration": the invariant is "admin frames do not write the body's roster row". Make the body the controlled variable (silent), not the clock.
- D2. Observability without sleeping: sample `last_seen` after the admin calls and compare to the baseline, but get the "time has passed" half from the real calls, not a sleep: the baseline is read, then each admin CLI run (process spawn, tens to hundreds of ms) happens, and a mutant that touches the row can only produce an equal value if it lands in the same wall second. To remove that gap, loop the admin calls until the hub's clock has provably crossed a second boundary (`wait_for` over `hub_status_json`'s `now`/uptime if it exposes whole seconds, else poll `Roster`-independent `date`-equivalent `SystemTime` seconds in the test process, which shares the hub's wall clock since `Clock::System` is `token::now_secs`). Then assert equality. This is a condition wait on an observable, not a fixed duration. T picks the exact observable; if neither exists, fall back to polling the test process's own `SystemTime` seconds.
- D3. No production hook (no injectable clock in the hub binary) and no new env var. The roster's `Clock::Manual` stays unit-test only.
- D4. Keep the test at process level (real hub, real CLI, real admin socket): that is what AC 4 pinned. The unit-level "admin never touches Roster" is already the module contract in `admin.rs:5-8`; not adding a second unit test here.
- D5. Leave `two_dir_rig` and the other 20 tests alone. They do not assert on `last_seen`; verified by `grep -n last_seen crates/holler-cli/tests/remote_admin_test.rs`. T must re-run that grep and record any other time-window assertion on a live body as a follow-up, not fix it here.

## Out of scope
- Any change to the body's heartbeat or presence cadence, or to `Roster::touch`.
- A hub clock seam (D3).
- The other 20 tests in the target; other flakes.
- Sibling issues (see Clashes).

## Test plan
RED first (T-red), two kinds, both required:
1. **Reproduction of the defect in the old test.** Record, as the RED evidence for the old test shape, `HOLLER_HEARTBEAT_INTERVAL_MS=300 cargo test -p holler-cli --test remote_admin_test remote_admin_traffic_never` failing 3/3 at the `assert_eq!` (this brief already ran it; T re-runs it on the unmodified test to confirm RED before F).
2. **New test is red against the right mutant.** T authors the new test, shows it green on the unmodified hub, then applies the AC 4 mutant (admin path touches the roster row), shows it red, and restores it (`git diff -- crates/holler-hub` empty). The same test must also stay green with `HOLLER_HEARTBEAT_INTERVAL_MS=300` in the environment, because the hand-rolled body sends no heartbeat: this is the direct proof the flake's cause is removed. Record both runs.

GREEN (T-green):
- 30 consecutive runs of the single test under load: e.g. `for i in $(seq 30); do cargo test -p holler-cli --test remote_admin_test remote_admin_traffic_never || break; done` while a second shell runs the whole `remote_admin_test` target with `--test-threads=16` in a loop (or `stress-ng --cpu $(nproc)` if installed). Zero failures. Use a target dir inside the worktree.
- Then the full `remote_admin_test` target and clippy.
- No fmt run workspace-wide; format only the touched file with `rustfmt --check` on that file and report pre-existing hunks separately.

## Risks
- A hand-rolled body that sends one presence and then nothing is silent for the test's life; the hub's liveness timeout is 45 s (`circuit.rs:100-102`), far above the test's length, but a very slow box must not push the test past it. T notes the longest observed run.
- The admin CLI authenticates with the same token as the hand-rolled body (as the real one does). It must not supersede the hand-rolled body (AC 3 already pins that); if the hub ever changes that, this test fails for a real reason.
- `roster_json` and the CLI read the hub through its control socket, which may itself be slow under load; that only lengthens the window, which the silent body makes harmless. That is the point of the fix.
- Honest limit: the original flake is a single-observation failure, so "30 green loaded runs" is evidence, not proof; the proof is the 300 ms heartbeat run (stays green) plus the mutant run (goes red).

## Clashes (siblings)
- #844 (OpenCode adapter locale), #845 (pane switch after relaunch), #846 (fold LinkedHost fakes in pane rigs), #696 (shared bounded subprocess runner for pane adapters), #848 (wizard rename scan skips `docs/handoffs/`): all concern pane code or the wizard, so none touch `remote_admin_test.rs`, `support/remote_admin_rig.rs`, `support/raw_ws.rs` or `holler-hub`. Possible touch points only: #846 may reshape `crates/holler-cli/tests/support/` (a new module or `mod.rs` edits); this story edits no shared support file unless D3-style extraction is taken, in which case only `remote_admin_rig.rs` (not part of the pane rigs). #848 scans `docs/handoffs/` and this story adds `docs/handoffs/847*` files that must obey the rename scan as it stands. Merge order is free; the later one rebases without conflict in the expected case.
