# Handoff-A: Phase 3 - #647 part 2: `unattached` and `agent-cannot-dispatch`  (up-front plan review)

**Date:** 2026-10-12
**Branch:** issue-647-implementation (worktree `.claude/worktrees/0647-agent-cannot-dispatch`, head `6cdbf1f`, `main` at `6eff9cc` merged in)
**Brief reviewed:** `docs/handoffs/647-part2-brief.md`   **Reuse map:** the brief's D4 ("Reuse, nothing copied"), D14, D15 and E18-E20, E37, E38, E61-E69 (no `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS, with no block. The plan extends the right objects at the right layers. It adds two required read-only methods to the ports that own the facts: the client count on `HostPort` (tmux) and the agent view on `HarnessPort` (the server API). That is #835's rule, and the ADR is amended first, in a commit that is documentation only. The OpenCode adapter turns OpenCode's permission rules into an `Option<bool>`, so no adapter text reaches `holler-pane` (I6, I7). Reconcile reads only through the ports, at the two hooks the chain already has: `check_tmux` for both shapes, and after `check_session_of_record` for a server, where it borrows `&Server` as `check_screen` and `record` do. The test kit gets its support in the shape of the fakes' existing scenario methods, and every wrapper gets a forward and a test. There are eight warns. The most important: the Reuse map's shared message reader cannot be shared inside the Blast radius as it is written (W-1, which becomes a Phase 7 block if F writes a second reader), and the remedy plan differs from #839's in-flight one (W-2) and misleads for a pane in a profile (W-3).

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn (a Phase 7 block if missed) | D4 and E19, "the reader is shared" (`from_messages`); D15, "quotes no reply (the adapter's rule)"; acceptance 9, "quoting nothing"; the Blast radius, which does not list `prompt.rs` | abstraction level; anti-duplication | `from_messages` is not a reader. It reads `GET /session/:id/message`, checks the shape and returns an `Activity`, all in one private function. Sharing its read means splitting it, which edits `prompt.rs`, and the Blast radius does not list that file. Its failure quotes 60 bytes of the body, while acceptance 9 asks for none. Held to the Blast radius, F can only write a second reader. `GET /agent` has the same problem on a smaller scale. | Treat the split as the refactor-to-extend that D4 already specifies. Write one `pub(crate)` reader of a session's message list, called by `from_messages` and `agents.rs`, that quotes no body. Write one `GET /agent` reader in `server.rs`. F records the `prompt.rs` edit in its handoff. |
| W-2 | warn | D8: `remedy_for(self, pane, fix, agent)` and `Finding::with_agent`; the `findings.rs` module doc | pattern consistency across parallel stories; cross-cutting | #839 (F done, not merged) solves the same need, a remedy that carries an observed value, with a builder set after `with_fix`. It also rewrites `remedy`'s doc to say "the table answers by kind and fix alone". With both merged, `findings.rs` has two mechanisms for one concern. `with_fix` recomputes the remedy without the agent. The module doc says no remedy carries adapter text. | Keep `remedy_for` (main has one table). Apply `with_agent` after `with_fix`, and add no field to `Finding`. Amend the module doc to admit a parsed `AgentKey` in the joined form. Whichever of #839 and this story merges second makes the docs agree and files one follow-up to fold the two mechanisms into one. |
| W-3 | warn | D8's remedy, `holler pane relaunch PANE --agent=KEY`, for a pane whose record names a profile | cross-cutting: the spec-editing verbs (ADR-0021 section 3) and `profile-drift` (#665) | Without `--profile`, `relaunch` changes the record and not the profile's spec. So following the remedy on a member trades this finding for `profile-drift` on the next pass, and `profile apply` puts the agent back. Every imported pane is a member of `fleet`. | For a member, the message and the guide row say that the relaunch also needs `--profile` with the pane's profile. A remedy that carries `--profile` would change part 1's rule, so it is a decision for the operator (a follow-up issue). |
| W-4 | warn | D2, D15 and acceptance 7: Claude Code "answers an empty view for the endpoint they are given" | pattern consistency; ADR-0021 section 15 item 5 | A single-process harness answers a port, or a server step, with `server-unsupported`, and `at.process()` is its one conversion. The closest analogue, `list_sessions`, refuses. An empty view for any endpoint would hide a misrouted OpenCode call: the silent default that D3 rejects. | Keep the empty view only for a `Process` endpoint. Both Claude implementations call `at.process()?` first. `FakeClaude` passes its faults with a new `ClaudeOp::AgentView`. One clause in the section 12 text names the difference from section 15 item 5. |
| W-5 | warn | Scope 2 and the Blast radius, "new `crates/holler-pane/src/agent_view.rs` re-exported from `lib.rs`", against the `ports.rs` row, "`mod agent_view;`" | naming and file structure | These two contradict each other: `mod agent_view;` in `ports.rs` names `src/ports/agent_view.rs`. The vocabulary of a harness port lives in `ports/process.rs`. `holler-pane` already has an `unattached` with another meaning (`import::findings::Finding::unattached`). | Put it in `src/ports/agent_view.rs` with `pub use` in `ports.rs`, and add it to `lib.rs`'s `pub use ports::{...}`. The doc of `FindingKind::Unattached` says "no tmux client is attached". |
| W-6 | warn | D13, "the read-only allowlist adds exactly ..."; acceptance 15 and 16; the Blast radius row for `pane_verbs/doctor.rs` ("two `mod` lines") | cross-cutting: the read-only guarantee is pinned in two places | The rig's panes record no agent, so every doctor test pass will call `attached_clients` and `agent_view`. `pane_verbs/doctor.rs:338-342` also asserts that every host call is `HostOp::Ps`. The brief does not name that assertion, and it will fail. | Name both assertions, preferably as one list of host reads that both tests use. Edit both in T's RED commit. |
| W-7 | warn | D15: `agent_view` reads the whole message list of the session of record, for each OpenCode pane with no recorded agent, in every pass | concurrency; I5 | Doctor is the periodic pass (ADR-0021 section 12), and the fleet's sessions are long-lived. A reply over 64 MiB, or one past the request or call bound, gives `observe-failed` on every pass. The brief states no bound for this read. | Measure whether OpenCode 1.19.0's message route takes a limit, in the opt-in real test, and use it if it does. If not, say that the whole list is read and what happens past the cap. Add one clause to the ADR's "Reads and writes" bullet. |
| W-8 | warn | Blast radius sizes: `reconcile/observe.rs` "about 540" and `ports.rs` "about 700"; the agent check placed in `observe.rs` | size and structure; overlap with a parallel story | #839 adds 98 lines to `observe.rs` and edits the same block of `check_pane`, so the file lands near 630. `ports.rs` lands near 745, or 760 with #839. Both stay under 800. | Put the agent check in a child module, `reconcile/observe/agent.rs`, the shape `observe/process.rs` set. `check_pane` gains one call, and the rule stays away from #839's hunks. |

No finding blocks. The plan does not read files to observe state (I6). It amends the ports under the operator's carve-out of 2026-10-11, ADR first. It does not re-declare or stub a merged item. And it builds every remedy in the one table and names no non-Holler command, so it trips none of the earlier plan check's re-review conditions.

### Evidence and detail

**W-1.** `crates/holler-adapter-opencode/src/prompt.rs:118-130`: `from_messages(call, session) -> Result<Activity, PaneError>` holds the request, the shape check and the activity rule in one private function, and fails with `call.unexpected(GET_MESSAGES, &reply, "a list of messages")`. That message quotes up to 60 bytes of the body. The crate's documented rule (`lib.rs:30-35`) is "quotes at most 60 bytes of the body; the reply to a prompt is quoted not at all". So D15's "quotes no reply (the adapter's rule)" misstates it, even though "quote nothing" is the right rule for this route: the list is conversation text, which is the reason a prompt's reply is never quoted. The Phase 7 rule blocks a parallel path where the map called for an extension, so a second reader of the same route in `agents.rs` would be blocked.

What to do: put the shared reader in `prompt.rs`, or in `lib.rs` beside `known` and `status_object` (the crate's "one reading of ..." helpers), and give it `Call::unexpected_status`. Acceptance 9 then holds. `session_activity`'s fallback also stops quoting, which no test pins: a search for `a list of messages` under `tests/` finds nothing.

The same applies to `GET /agent`. The request and the array check live in `verify_serving` (`server.rs:100-105`), not in `usable`. `usable` (`server.rs:114-129`) returns `Result<(), PaneError>`, so using it as a filter means telling `agent-unusable` from `unavailable` by matching the error. Either split out a per-element predicate that `usable` wraps with its messages, or write the error match once with a comment that says why. Either way the rule stays written once.

**W-2.** Branch `issue-839-implementation` (F done at `d480424`, 09:16 MDT today, not merged) adds `switch_command`, `parse_session_id`, `Finding::with_switch_remedy` and `Finding::without_remedy` to `findings.rs`. The doc of `with_switch_remedy` says "Set after `Finding::with_fix`", and `remedy`'s doc becomes "The table answers by kind and fix alone; reconcile sets two remedies itself" (`findings.rs:143` on that branch). The brief's "Clashes" row calls this overlap mechanical, but it is a choice between two patterns. On `main`, `Finding::with_fix` (`findings.rs:321-328`) sets `remedy: self.kind.remedy(self.pane.as_ref(), fix)`, so it would drop an agent remedy applied before it. The module doc (`findings.rs:12-18`) says that no remedy carries "any adapter text". A key from the server's agent list is adapter text, even after `AgentKey::parse`. #839 amends the same paragraph for its session id, which is built only after `parse_session_id`.

**W-3.** `crates/holler-cli/src/pane/relaunch.rs:75-80`: without `--profile`, the base that the flags overlay is `spec_from_pane(&record)`. ADR-0021 section 3 (line 255): "Without `--profile` these verbs never touch a profile." `opencode_agent` is compared by `profile_diff` (#644, section 3), and `profile-drift` (#665) reports it. `pane import` writes every imported pane into the profile `fleet` (guide section 10), so on the live fleet the remedy leads to drift in the common case. Part 1 kept a profile out of every remedy (`findings.rs:15`), and D8 does not consider profiles. The finding's rule (D6) is unaffected. Only the advice is.

**W-4.** ADR-0021 lines 1247-1249: "`HarnessEndpoint::port()` and `process()` are the one conversion each shape makes: ... a single-process harness answers a port, or a server's or a TUI's step, `server-unsupported`." The Claude adapter's module doc (`holler-adapter-claude/src/lib.rs:7-8`) says the same. In both Claude implementations, every method keyed by an endpoint either converts the endpoint first (`health`, `abort`, `shown_session`, `send_prompt`: `at.process()?`) or refuses outright as a server's step (`verify_serving`, `create_session`, `list_sessions`, `attach_tui`: `no_server`, `lib.rs:367`). With the conversion, both misroutes are loud: a `Port` endpoint reaching Claude is `server-unsupported`, and a `Process` endpoint reaching OpenCode is `process-unsupported`. The empty view is then only the "cannot tell" answer to a call that was routed correctly, which is coherent with D5. Answering `server-unsupported`, as `list_sessions` does, is the other consistent choice.

**W-5.** `ports.rs:43-44` declares `mod process; mod router;`, and `ports.rs:58-62` re-exports from both. `lib.rs:72-75` re-exports flat from `ports`. `lib.rs`'s module list is also the crate's hot spot: #839 adds three modules to it. `import/findings.rs:217`: `pub fn unattached(kind: ImportFinding, message: String)` builds an import finding that is about no pane.

**W-6.** `holler-pane-testkit/src/fixture.rs:79` sets `opencode_agent: None`, and `doctor/rig.rs`'s `record_of` builds every record from `sample_pane`. So D6's rules (1) to (3) hold for every healthy rig pane. `doctor/read_only.rs:88` and `pane_verbs/doctor.rs:339` assert the same host-reads-only rule. The harness assertions of `doctor.rs:316-334` look only for `SelectSession` and the writers, so `AgentView` does not break them.

**W-7.** `holler-adapter-opencode/src/http.rs:40` sets `MAX_REPLY` to 64 MiB, and `Timeouts` has `request` 5 s and `call` 10 s (`lib.rs:263-271`). One `agent_view` makes at least two requests inside one call's bound. `from_messages` has the same property today, but it is used only as the fallback for an old server. `agent_view` makes it the regular path.

**W-8.** `git diff 6eff9cc issue-839-implementation --stat -- crates/holler-pane/src/reconcile/observe.rs` shows 98 lines added. That branch's `check_pane` calls `check_driven` after `check_screen` and takes a `bodies` parameter. A child module may use the parent's private items, so it can call `Server::has`, as `observe/process.rs` uses `Server`, `Screen` and the parent's helpers (`process.rs:27-30`).

**Also checked and consistent:**

- The borrow plan of D13 compiles as stated: `check_screen` and `record` take `&Server` (`observe.rs:266-273, 412-418`), and `server.into_sessions()` consumes `server` last (`observe.rs:136`). This settles brief-gate NV-2.
- `check_tmux` runs before the shape branch (`observe.rs:113, 125-126`), so one call covers both chains (NV-1).
- The router's `by` (`router.rs:53-58`) makes the `agent_view` arm one line, and the module table's endpoint row names it (NV-3).
- All six ADR anchors of the amendment exist verbatim at `6eff9cc`.
- The implementations listed in "Wrapper forwarding" match a workspace search for `impl … HostPort for` and `impl … HarnessPort for`: 13 host implementations (one of them the `PidsOnly` doc test) and 20 harness implementations. None is missing. `pane_integration/observe.rs` holds only a `HerdrPort` double, so it needs no forward.
- No hub, proto or persisted state changes. No new error code.
- Every touched file stays under 900 lines, and the two that cross 800 are named: `switch.rs` (split) and `launch/rig.rs`.

## Notes for O

PASS, so the brief needs no amendment before T. The automated path has no O, so this is what T and F take from the warns.

- **T, in the RED commit:**
  - Admit `HostOp::AttachedClients` in both host-reads assertions (W-6).
  - Write acceptance 7's Claude case for a `Process` endpoint, and add one check that a `Port` endpoint is `server-unsupported` (W-4).
  - Acceptance 9's "quoting nothing" stands (W-1).
- **F:**
  - Share the message reader and the agent-list reader instead of copying them, and record the `prompt.rs` edit (W-1).
  - Apply `with_agent` after `with_fix`, add no field to `Finding`, and amend the module doc's remedy rule (W-2).
  - Add the profile clause to the message for a profile member (W-3).
  - Use `src/ports/agent_view.rs` (W-5).
  - Put the rule in `reconcile/observe/agent.rs` (W-8).
  - State the bound of the message read in the handoff (W-7).
- **For the operator, as follow-ups if not decided in this run:**
  - Whether `agent-cannot-dispatch`'s remedy should carry `--profile` for a profile member (W-3).
  - One remedy mechanism in `findings.rs` once #839 and this story have both merged (W-2).
  - Phase 7 will check W-1 and W-2 against the diff.

## Patterns referenced

- `crates/holler-pane/src/ports.rs`, `ports/process.rs`, `ports/router.rs`: the required-method rule (#835), where a harness port's vocabulary lives, the one conversion each shape makes, and the routing table.
- `crates/holler-pane/src/reconcile/observe.rs` and `observe/process.rs`: the chain's shape, the trust rule (`Server::has`), and the child-module precedent.
- `crates/holler-pane/src/findings.rs`: the one remedy table, `with_fix`, and the remedy rule of the module doc. #839's `findings.rs` at `d480424` is the parallel pattern.
- `crates/holler-adapter-opencode/src/{lib.rs,prompt.rs,server.rs}`: one submodule per slice of the port, the shared readers in `lib.rs`, and the quoting rule. `crates/holler-adapter-claude/src/lib.rs`: the refusals for the other shape.
- `docs/adr/ADR-0021.md`: section 2 (ports), section 3 (spec-editing verbs), section 12 (the periodic pass) and section 15 item 5 (the conversion and the refusals of each shape).
