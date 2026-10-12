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

## A, up-front plan review, third pass: 2026-10-11T21:25-06:00 (MDT)

- **Decided:** BLOCK, with 2 `block` and 6 `warn` findings (`docs/handoffs/839/handoff-A.md`, which replaces the second pass's file; that version is at `8b021d5`).
  - **The second pass's ten findings are all answered.**
    - The block is resolved by P4: the relaunch case is found by rule (ii), and the one-body case is `Unproven`.
    - Warns 2 to 9 are resolved, each checked against the code.
    - 10 was re-read at the amendment, but is stale again.
  - **The blocks:**
    1. **P4's exclusion, stated two ways.** D3, P4 and section 11 remove only a body that advertises another pane's session of record. Acceptance 20(b)'s last case, D2's restart text and question 5 assume that a body proven by another pane's recorded DRIVEN is removed too.
       - Under the rule as written, that body is `Unproven`.
       - The `Unproven` remedy then pairs it with the wrong pane: `check_unclaimed` checks records only, and rule (iii) claims the body.
       - T's test and F's verbatim rule cannot both hold.
    2. **`HubRefusal::NotDriven` by code.** The hub sends `-32004 not_connected`, with no `data`, both at resolve time and after delivery (`say`'s `Timeout` and `Cancelled`; `interrupt`'s `AckTimeout` and `PromptFailed`).
       - A `say --pane` past `--timeout` would print `pane-not-driven` with exit 3 while the turn runs on.
       - `reset --first`'s `Send` built with the 2 s wait as its `timeout_ms` would race the hub's own turn timeout.
  - **The warns:**
    - #831's `ports/router.rs:118` forward breaks D5's `.send_prompt(` scan, whose strip rule is also ambiguous.
    - The `send_prompt`-driven `hold_tests` cannot move to `panes/` (they are private to `circuit`), and each `#[cfg(test)] mod …;` line must be last in its file, for `code_of`.
    - `FakeHub`'s selection (record only, one pane) is a second, different copy of the gate's selection.
    - The brief's line 444 fails the wizard's rename scan, so `cargo test --workspace` fails.
    - The clashes table: #667 merged, #831 built, #835 reworked again, #648 S PASS, #803's brief exists and resolves the deferred dependency.
    - `SESSION_DRIVEN_ELSEWHERE` belongs in `tx_switch.rs`; `token_id`, not `token`; advertised ids are untrusted text.
- **Confirmed against the code, and left standing:**
  - The circuit matches replies to the pending query by correlation id, so the slot design holds.
  - `talk::say`'s prelude moves cleanly into `admit`, and `would_refuse` consumes no grant.
  - `with_fix` recomputes `remedy`, so setting it afterwards is right.
  - `SessionManager::start` builds the presence before the spawn.
  - `store.rs::call` decodes a `PaneReply`, so a separate send over the three helpers is right.
  - `tx_switch` has no early return for the current record, so the switch remedy runs the whole path.
  - The session task `select!`s over its mailbox during a turn, so a busy rebind can answer `session_busy`.
  - `circuit.rs`'s `Say` destructure and `HoldGate` literal are single lines, so +0 lines is feasible.
  - ADR-0021 has one "stays in force, unchanged" (:868).
  - #667's landed `scenario.rs` matches E155, and its new scenario files touch none of `--first`, `send_prompt`, `Ports` or DRIVEN.
  - #803's D13 keeps one attach row per OpenCode pane, with `session_id` from the registry.
- **Assumed:**
  - Phase 3 warns reach no later phase: the script stops only on BLOCK, and `feature.md` and `tester.md` do not read `handoff-A.md`. So the warns are written for this amendment.
  - Local refs are current. I did not fetch.
  - On servers that share a data directory, P2's recorded DRIVEN differing from its record, with a body on it, is rare in production; block 1's certain cost is the T/F contradiction.
- **Hedged:**
  - Block 1 could be read as a `warn` on production reach alone. It is a `block` because the brief contradicts itself on the rule this story exists for, and the contradiction reaches GREEN with no one to resolve it.
  - Both blocks offer two fixes, and I recommend one of each: (a) the exclusion extended, with this pane's evidence first; (a) an additive `data.reason` on the resolve-time `-32004`.
  - I ran no build or test (no code exists yet). The rename scan was reproduced with `git ls-files` and `grep`.
- **Evidence:**
  - Read the brief's decision sections in full (lines 1 to 515).
  - **Hub:** `live.rs` (`query`, `insert`, `replace_session`, `resolve_session`, the inline tests), `circuit.rs` (the `Query` and `Say` arms), `circuit/dispatch.rs` (`pane_gate`, `send_prompt`, `handle_presence_notification`, `handle_response`, `handle_error_response`, `hold_tests`), `talk.rs` (`answer`, `say`, `send_turn`, `not_connected_detail`), `holds.rs` (`would_refuse`), `control_server.rs` (`say`, `interrupt` and `answer` mappings, `encode_error`, `test_drop`), `control.rs` (`say_with`, `token_ping`, `revoke_live`, `CLIENT_TIMEOUT`), `control_hold.rs`, `roster.rs` (`advertise`, `Row`), `panes/mod.rs`.
  - **Body:** `session_manager.rs` (`start`, `presence_doc`), `session_manager/task.rs` (`SessionPresence`, the run loop).
  - **Pane crate and kit:** `tx_switch.rs` (`check_unclaimed`, `check_listed`, `check_idle`, the codes, `parse_session_id`), `tx_launch.rs` (`session_of_record`), `error.rs`, `findings.rs`, `ports.rs`; the testkit's `fault.rs` and `lib.rs`.
  - **CLI:** `say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs`, `prompt_target.rs`, `pane/wiring/store.rs`, `cli.rs`; the tests `hold_single_path_test.rs` and `wizard/skilltext_w5_rename.rs`.
  - **Proto:** `docs.rs` (`Presence`, `SessionAd`).
  - **Workflow:** `coding-pipeline.workflow.mjs`, `feature.md`, `tester.md`, `.aftersight/pipeline.config.json`.
  - **Siblings (local refs):** `origin/main` (`0f5835e`, #667's diff and its `scenario.rs`); #831 at `d137a1d` (`ports/router.rs`, `ports.rs`, the `Ports` literals, the `.send_prompt(` calls); #835 at `d71b091`; #648 at `823e0a3`; #803's brief at `9af2ad7`.
  - Commands: `grep -n`, `sed -n`, `wc -l`, `git show`, `git grep`, `git log`, `git diff --stat`, `git merge-base`, `git ls-files`.
