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

## A, up-front plan review, second pass: 2026-10-11T20:43-06:00 (MDT)

- **Decided:** BLOCK, with 1 `block` and 9 `warn` findings (`docs/handoffs/839/handoff-A.md`, which replaces the first pass's file; that version is at `6211c86`).
  - **The first pass's 13 findings are all answered.**
    - The three blocks are resolved: P1, P2 and P3.
    - Of the warns, 4, 5, 7, 9, 12 and 13 are resolved.
    - 6, 8 and 11 are resolved in substance, with follow-on points (findings 2, 5 and 6 of this pass).
    - 10 is narrowed correctly, but the relaunch sentence it added is false (block 1).
  - **The block:** `driver_of` takes "the pane's own server lists the advertised id" as proof that a body belongs to the pane.
    - **Too wide:** with servers that list each other's sessions (the case `check_unclaimed` documents), another pane's behind body is "the only candidate" for a pane with no connected body. `doctor`, `pane switch` and `pane reset` then claim it and re-point it silently.
    - **Too narrow:** after a relaunch onto a new session, the body's session is no longer listed. It is never found, `doctor` erases the stored DRIVEN, and `pane switch` succeeds without re-pointing it. The verbatim section 11 text says the opposite.
    - P2 makes behind bodies routine (every restart), so this rule is now the main recovery path.
  - **The warns:**
    - the routed verbs' hold exit (3 or 4) and the shape of `hub_refusal`;
    - two bodies on one session after a switch;
    - `talk::admit` must be `say`'s extracted prelude;
    - the slot outside `query`'s timeout, one send and wait, and the 10 s tie between the hub and the CLI;
    - the remedy set by hand against `with_fix` and the remedy table;
    - the presence mirror before the task runs;
    - reuse of `store.rs`'s failure mapping and of `FaultSwitch`;
    - section 11's "ADR 0014 stays in force, unchanged.";
    - the clashes table (#667 now has code in `scenario.rs`).
- **Confirmed against the code, and left standing:**
  - `LiveCommand` is `pub`, and `live/target.rs` reaches `LiveHandle`'s private `tx` as a child module.
  - `Inner.config` is an owned `SessionConfig`, read by `run`, `attempt_initial_attach` and `start_turn`.
  - The body reads `session_id` in exactly `presence_doc`, `query.rs`, `registry.rs` and `HttpAttachDriver::attach`.
  - `SayOpts` is built only in `send_turn`.
  - The CLI's bare-id refusal equals the hub's existing `RoutableName::parse` refusal.
  - `advertise` upserts a changed `harness_session_id`.
  - The four ADR passages exist as quoted.
  - The 15 `Ports` literals are in 13 files.
  - `codec_test.rs` takes a one-line arm.
  - The existing forward scan still passes with P1.
- **Assumed:**
  - OpenCode servers that share storage answer `GET /session/{id}` for each other's sessions, so a rebind through another pane's server succeeds. The brief assumes the same ("every rebind would succeed").
  - A body can be disconnected while another pane's body is up, e.g. during the cutover or while a fleet boots.
  - Local refs are current. I did not fetch.
- **Hedged:**
  - The "too wide" half needs listings that overlap. Production may not have them: question 2 confirmed one attach body per pane, not separate data directories. It is still a `block`, because the plan designs for overlapping listings (Acceptance 20, `check_unclaimed`) and its own rules reject a silent wrong pairing.
  - The "too narrow" half does not depend on overlap.
  - Two fixes are offered. I recommend none over the other: (a) rules over the record and the hub, with a fallback guarded by the shared-listing signal; (b) a join on the endpoint from the body's `query/status`.
  - I ran no build or test: no code exists yet.
- **Evidence:**
  - Read the amended brief: the decisions, the ADR text, where the code goes, Reuse, acceptance, blast radius, clashes, and E98 to E136.
  - **Hub:** `live.rs` (in full), `talk.rs`, `interrupt.rs`, `circuit.rs` (the command arms, `PendingQuery`), `circuit/dispatch.rs` (`handle_response`, the presence handler), `circuit/admin.rs`, `control_server.rs`, `control_hold.rs`, `control.rs`, `roster.rs` (`advertise`, `apply_ad`, `set_last_turn`), `panes/mod.rs`, `lib.rs`.
  - **Body:** `session_manager.rs`, `session_manager/task.rs` (in full), `session_manager/driver.rs`, `http_attach_driver.rs`, `http_attach_driver/connection.rs`, `connection.rs`, `connection/session_dispatch.rs`, `connection/handshake.rs`, `query.rs`, `registry.rs`, `dispatch.rs`.
  - **Pane crate and kit:** `ports.rs`, `lib.rs`, `tx_switch.rs` (in full), `tx_launch.rs`, `reconcile.rs`, `reconcile/observe.rs`, `findings.rs`; the testkit's `lib.rs`, `fault.rs` and manifest.
  - **CLI:** `say_cmd.rs`, `prompt_target.rs`, `interrupt_cmd.rs`, `answer_cmd.rs`, `hold_cmd.rs`, `pane/wiring.rs`, `pane/wiring/store.rs`; the tests `hold_single_path_test.rs`, `hold_hub_test.rs`, `pane_verbs/target_flags.rs`, `pane_verbs/switch.rs` (`SeamHarness`) and `docs_pane_control_test.rs`.
  - **Proto:** `methods.rs`, `lib.rs`, `hold.rs`, `codec_test.rs`.
  - **Docs:** ADR-0003, ADR-0005, ADR-0014 and ADR-0021 (sections 2 and 11, the header).
  - **Issue and siblings:** issue #839 (`gh issue view`); the sibling branches #667, #835, #648, #665 and #831 (`git log`, `git diff --stat` from the merge base, `git grep`).
  - Commands: `grep -n`, `sed -n`, `wc -l`, `git show`, `git grep`, `git diff --stat`, `gh issue view`.
