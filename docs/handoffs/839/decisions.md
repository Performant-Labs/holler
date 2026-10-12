# Decision journal: #839, DRIVEN follows each pane's session of record

Run handoff directory `docs/handoffs/839` · branch `issue-839-implementation` · rigor in-session · uiSurface false
(D and U are N/A). Append-only; every phase adds its entry.

## A, up-front plan review: 2026-10-11T19:59-06:00 (MDT)

- **Decided:** BLOCK, with 3 `block` and 10 `warn` findings (`docs/handoffs/839/handoff-A.md`).
  - **The blocks:**
    1. Widening `LiveHandle::query`'s allowlist exposes `session/rebind` through `control/query_remote` and `admin/query`. That second route skips `control/rebind`'s cache update and the brief's own choice to keep the rebind off the admin surface. Acceptance 13 would pin it.
    2. The body's attach id becomes runtime state, but the plan names only `presence_doc` as a reader. The session task re-attaches from its immutable config after a turn that ends in error (`task.rs:493`, `:662-663`, OpenCode's `session.error`) and in its retry loop. The body then silently returns to the pre-rebind session. A body restart reverts the same way, and the brief does not decide it. With servers that share a data directory, the resulting ambiguity refuses the stated remedy.
    3. D1 makes a harness session id an address. That changes the addressing rules of ADR 0005 section 3, ADR 0003 and ADR 0014 (point 5 and Consequences), and the brief amends none of them. It also makes a bare `holler say ses_…` work, with no decision behind it.
  - **The warns:**
    - `Roster::advertise` replaces a token's whole set of rows.
    - `DriverState` duplicates `Activity`.
    - The accept rule, the refusal mapping and the `SayError` mapping must each exist once; three CLI files are missing from the blast radius.
    - The `.send_prompt(` scan would fail on `wiring/bind.rs`'s forward.
    - `FixState` is JSON-visible.
    - The gate misses a body that is behind.
    - The I2 sentence over-claims for launch and relaunch.
    - Queries share one slot per connection.
    - `resolve_target`'s name, and ADR section 2's file list.
    - Blast-radius gaps (`codec_test.rs`, the body's fake server, the testkit's crate).
  - **Confirmed against the code, and left standing:**
    - the shared predicate in `holler-pane`;
    - the gate key set in `send_turn` (the only place `SayOpts` is built);
    - `control_rebind.rs`, `rebind.rs`, `HubPort`, `wiring/hub.rs` and `FakeHub` with its conformance file, each following its precedent;
    - D4's hold keys;
    - D3's step order;
    - `circuit.rs` with no growth.
- **Assumed:**
  - F would read the brief's "swapped state" literally, as a value separate from `Inner.config`. Nothing in the brief or the acceptance asks F to update the task's config.
  - Admin-role connections are remote and less trusted than the local control socket. The brief keeps `control/rebind` off them.
  - Bodies restart (a reboot, a deploy) often enough for the restart case to matter.
  - `origin/main` is still `bda1069`. I read local refs only and did not fetch.
- **Hedged:**
  - Finding 3 is documentation plus one CLI decision. It is a `block` because the repo's review overlay requires an ADR to be amended in the same change, and the brief is being amended anyway.
  - Finding 9 (key the gate on a recorded DRIVEN too) is a design suggestion beyond the brief; the operator may prefer to state the window.
  - For finding 2's restart decision, both options are acceptable architecture. The handoff recommends making it explicit, not one answer.
  - I ran no build or test: no code exists yet.
- **Evidence:**
  - Read the brief in full (3042 lines, E1-E97).
  - **Hub:** `live.rs` (`query`, `resolve_session`, `replace_session`, `find_by_token`, `find_target`), `talk.rs`, `interrupt.rs`, `circuit.rs` (the `Query` and `Say` arms, `static_wire_method`), `circuit/dispatch.rs` (`send_prompt`, presence and response routing), `circuit/admin.rs` (`admin/query`), `control_server.rs` (`dispatch_session_control`, `hub_query_remote`, `say`), `control_hold.rs`, `roster.rs` (`advertise`).
  - **Body:** `session_manager.rs`, `session_manager/{task.rs,driver.rs}`, `http_attach_driver.rs` and `connection.rs`, `query.rs`, `registry.rs`, `connection.rs`.
  - **Pane crate:** `ports.rs`, `tx_switch.rs`, `tx_launch.rs`, `reconcile.rs`, `findings.rs`.
  - **CLI:** `say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs`, `prompt_target.rs`, `pane/wiring/{store.rs,bind.rs}`, `query_cmd.rs`.
  - **Proto:** `lib.rs`, `methods.rs`, `tests/codec_test.rs`, the golden list.
  - **Tests:** `hold_hub_test.rs`, the fake OpenCode servers.
  - **Docs:** ADR-0003, ADR-0005, ADR-0014, ADR-0021 section 2, `docs/protocol/v2.md` sections 4 and 7, `scripts/lint.sh`, the #828 A review on branch `issue-828-implementation`.
  - Commands: `grep -n`, `sed -n`, `wc -l`, `git log origin/main`, `git show`.
