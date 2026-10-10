# Handoff-A: Phase 3 - #642a the OpenCode adapter, server side: re-review on the rework route after the outside diff gate's BLOCK  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-642-implementation at 1c3ae10 (642a as built: F 999d7e3, T-green 1c3ae10; `origin/main` still 3bdd129)
**Brief reviewed:** docs/handoffs/642-brief.md, byte-identical to 5a4d68e (sha256 `f80295d3...`)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)" (lines 935-968)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This pass replaces the PASS at 80901fa. Code is already on the branch because of how this pass was reached. The outside diff
gate's round 1 (`docs/handoffs/642-diff-result-r1.md`, untracked) returned BLOCK, and F had reported an architecture change,
so the BLOCK routed back to this phase (playbook `coding-pipeline-logic.mjs:1124-1127`) before T, F and T-green run again.
The brief has not changed, so I re-checked it against the code F wrote and against what changed upstream since my 17:27
review.

## Summary

PASS: no finding is a block. F's architecture change stays within the brief's own rules. The private `server.rs` is the
split the brief names, the private `exec.rs` holds only the kill path, `HttpError` keeps its variants, and the read bounds
are private. Two things changed upstream:

- Issue #642 and epic decision 8 now require this adapter to apply the pane's OpenCode agent, and no part of the brief covers
  it. This is N-1, the most consequential warn. It cannot land before #700.
- The diff gate's B-1 (an unbounded read to EOF) does not hold. I measured the cap: it holds to within one 8 KiB read (N-3).

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| N-1 | warn | The whole brief: the header, the split table (brief:67-70), Decisions, Forward-compat and Out of scope. Also `tui.rs:3-4` ("so that `crate::OpenCodeConfig` is final"). Both are set against issue #642 as amended on 2026-10-09 at 17:40 MDT, after the brief's last edit (17:11) and my PASS (17:27), and against epic #633 decision 8. | contract shape (forward-compat); layering | The issue now says the pane's OpenCode agent (`Pane.opencode_agent`) "reaches the harness through this adapter: applied as the server's `default_agent` when it is started, or per prompt if the OpenCode API takes an agent per message (the #635 spike's result says which)". It adds "one extra conformance case" and depends on #700, which is open and says "#642, #644 and #647 start when it has merged". The brief never mentions it: a grep finds no `opencode_agent`, `default_agent`, `AgentKey` or #700. For the amendment: **(a)** Only the `default_agent` route is this crate's layer. Prompts reach OpenCode through the hub's `send_prompt` (`holler-hub/src/circuit/dispatch.rs:81`) and `holler-body`'s `prompt_async` driver (`http_attach_driver.rs:38-40`). That is the DRIVEN side, which ADR-0021 §11 leaves to #649 and #654 (ADR-0021.md:437-453). `HarnessPort` has no prompt method, so a per-prompt agent falls outside this crate's blast radius. **(b)** The spike does not say which route applies. `opencode-pane-spike.md` has no `default_agent` and no per-message agent; its one agent field is the `shell` call's `"agent":"build"` (line 167). The installed OpenCode 1.18.35 bundle (`opencode-ai/bin/opencode.exe`) holds a `default_agent` config key, which refuses a hidden agent or a subagent, and an `OPENCODE_CONFIG_CONTENT` overlay variable. So the serve route is plausible, but it is unverified behaviour. **(c)** 642a takes the agent additively. `serve` resolves `workdir(name)` after its health check (`server.rs:36-37`), so the agent is one more `Resolver<PaneName, Option<AgentKey>>` beside it (the shape of my earlier W-2), and `start` (`server.rs:88-115`) sets one more variable. That resolver inherits decision 14's precondition: during a launch, the agent comes only from `--agent`, before any record exists. Under `ProcessEnv::Inherit`, an overlay variable would replace one the operator already sets. **(d)** The "extra conformance case" cannot be a `run_harness_conformance` case. No `HarnessPort` method observes a message's agent, and the shared suite is the test kit's file, which is out of scope. It has to be a real-OpenCode test in this crate, and that test sends a prompt, which the rig forbids today ("No prompt is ever sent", brief:821). **(e)** `tui.rs:3-4` calls `OpenCodeConfig` final, but it will gain a field. Nothing outside the crate constructs it yet (#649 does not exist), so only the claim is wrong. | **Before 642b's run**, amend the brief to place the agent part. It cannot go in 642a, because `AgentKey` is not on `main`. Put it in 642b, or in a 642c that starts after #700 merges. For this crate, either choose the serve-time `default_agent` route after a probe on scratch OpenCode (as O probed tmux), or give the per-prompt route to the hub and body path with its own owner. The amendment must state: the resolver and its precondition; the rule under `Inherit` (merge with, or refuse, an operator's own `OPENCODE_CONFIG_CONTENT`); and the crate-local test that stands for "one extra conformance case", with the rig's no-prompt rule restated for it (the dead-end provider still reaches no model). **In this cycle**, change no code. If F edits docs anyway, reword `tui.rs:3-4`. The 642a PR body names the agent part as #642's open remainder, waiting on #700. |
| N-2 | warn | 642b's file plan (Size check, brief:38-80; the split table's 642b row, brief:70), against 642a as built. `lib.rs` is 483 lines (estimated ~420 for all of #642). `hermetic_test.rs` is 704 (estimated ~650 for all of it), and `scripts/lint.sh` already warns on it. `tests/support/stub.rs` is 243. | size and structure | 642b adds `attach_tui` (with its poll and its death branch), `select_session` and `shown_session` to `lib.rs`. It adds AC 9, 10, 11a-11c and 11f, plus two `attach_tui` clauses, to `hermetic_test.rs`. At 642a's density that is about +200 lines in `lib.rs` (to ~680) and +250 in `hermetic_test.rs` (to ~950), past the 900-line failure of lint check 4. The brief's own split rules are used up: `server.rs` exists and the stub has moved (brief:77-80). | The 642b amendment names new homes. The TUI method bodies go in a private module (`tui.rs` beside its builders, or a new `src/attach.rs`), and `lib.rs`'s impl delegates to them in one line, as `serve` does (`lib.rs:193-195`). 642b's pure builder and parser tests need no stub (AC 9, 10, 11a-11c, 11f), so they go in a new test target, such as `tests/tui_test.rs`. Optionally, the helpers `lib.rs` hosts for its child modules (`json_of`, `excerpt`, `one_line`, `deadline_after`, `budget`) move to a private module. In this cycle, a cap test (N-3) can go in `hermetic_test.rs` (to ~730 lines) or start a `tests/http_test.rs`; either is consistent. |
| N-3 | warn | `http::request`'s reading rules (brief:558-563), which set no size bound. F's private `MAX_REPLY` of 64 MiB (`http.rs:35-37`; handoff-F, design decision 9), which no test pins. The outside diff gate's B-1 (`642-diff-result-r1.md:5`), which asks for a fix and "a lowered test-only limit". | cross-cutting concerns (bounds on input the adapter does not author); the public surface | B-1's premise does not hold. `fill` refuses once `buf.len() >= MAX_REPLY`, before every read, and one read adds at most 8 KiB (`http.rs:234-252`). The read-to-EOF path (`294-297`) therefore stops by `MAX_REPLY` + 8191 bytes. I measured it with a scratch program outside the repo, which called the crate's public `http::request` against a local server answering `200` with no `Content-Length` and no `Transfer-Encoding`. 200 MiB gave `Garbled("the reply is longer than 67108864 bytes")` in 29 ms, with a peak RSS of 68 MB. 64 MiB + 1 gave the same error. 64 MiB - 1000 gave `Ok` with the whole body. Two things in B-1's area are real. The bound is recorded only in F's handoff and the module doc, and no test pins it. And the doc's "head and body together" (`http.rs:35`) is looser than the code, which drains the head before it counts the body (`head()`, line 270). A "lowered test-only limit" would need a parameter on the public `http::request`, a `cfg` or a cargo feature. Each puts test-only surface in production code, against W-8's "not an interface". | **In this cycle**, pin the cap with one hermetic test over loopback (about 30 ms). Use the stub's existing `raw` mode (`stub.rs:127-130`): a head with no framing, then 64 MiB + 1 bytes. Do not write a second stub. Keep `MAX_REPLY` private, with no knob and no feature. If F changes code at all, the change stays in `http.rs` (the doc's wording). Because the cap already holds, a test that pins it passes on the current code. That pass is the answer to B-1, and T and F can record it that way for diff round 2. Record the bound in the brief's `http.rs` section at its next amendment. |

### Carried forward from the PASS at 80901fa

The brief is unchanged, so these still stand at the brief level. The code already acts on most of them.

| # | Status now |
|---|---|
| W-1 | Open, unchanged. Decision 14 and the #644 row (brief:974, 1036-1039) still say "#644 passes the session name (and directory)". #644's brief is still at 7195993, and its C-9 says there is no channel to pass them. 642a is unaffected, because wiring alone can meet `workdir`. The MO must decide before 642b, for `tui_session` and now also for N-1's agent resolver. |
| W-2 | Folded into N-1 for the agent. Env names, model and effort remain unowned (#644 decision 7). `lib.rs:60-62` states the position as built. |
| W-3 | (1) Done in code. The crate docs state decisions 1, 2, 13, 14 and 15(a), and that the three TUI methods answer `not-implemented` (`lib.rs:7-11, 37-62`). (2) Pending: the 642a PR body (divergence 15(a), #695, and the ADR note landing with 642b). |
| W-4 | (a) and (b) are cited in code: #695 at `lib.rs:47` and #696 at `exec.rs:7-8`, although the brief still says "no number yet". (c) Still open: no `FakeHarness` parity issue exists (I searched again). (d) Resolved: one form, `kill -s KILL -- -<pgid>` (`exec.rs:22-34`). (e) Optional and unchanged. |
| W-5 | Followed. The CHANGELOG entry is a part-1 entry, and no test fixes a port. S reads AC 23 and AC 24 as W-5 says. |
| W-6 | Followed. `exec.rs` holds only the kill path, and there is no `tempfile`. |
| W-7 | Followed. `stub.rs:1-9` names its cousin and keeps its conventions. |
| W-8 | Followed. `lib.rs:64-65` and `http.rs:4-7` say `http` is not an interface. |

### Checked and consistent with existing patterns (no finding)

- **F's architecture change, the reason for this pass:**
  - `server.rs` is the split the brief names (brief:77-78). It is private, `lib.rs` delegates `serve` to it in one line, and
    `health` shares its check.
  - `exec.rs` is private and holds only the kill path, with a refusal for a pgid of 0 or 1.
  - `HttpError` keeps its three variants. `Garbled` now also covers a connect failure other than a refusal or a timeout. The
    brief's mapping already sends that to `unavailable` naming the route, and it keeps `serve` from starting a server on a
    port it cannot check.
  - `MAX_HEADERS`, `MAX_REPLY` and `LONGEST` are private, and the public surface is exactly the brief's API.
- **Dependencies.** The crate depends on `holler-pane`, `serde_json` and `httparse`, and on `holler-pane-testkit` as a
  dev-dependency only. `Cargo.lock` adds only this crate's dependency list, and the manifest has no feature lists.
- **The mirror of #641 and the stop owner.** #641's amended brief (0f18b80) keeps `TmuxSocket { Default, Name(String),
  Path(PathBuf) }` (641-brief.md:569). It still does not stop the harness server (641-brief.md:726, 731-732). So decision 2
  and #695 stand.
- **The baseline.** `origin/main` is still 3bdd129 (`git ls-remote`), so the evidence the brief cites is current.
- **Public repository.** The branch's added lines hold no personal infrastructure names, and the gate's result and prompt
  files are untracked.
- **A process note closed.** CLAUDE.md now names `deepseek-v4-pro` as the outside model, matching the brief, so my earlier
  note on the model mismatch is closed.

## Notes for O

Not required for a PASS. They are needed here because the driver passes no diff-gate note on this route.

1. **What this cycle has to answer.** The outside diff gate's round 1 is `docs/handoffs/642-diff-result-r1.md`, which is
   untracked. It holds 1 BLOCK, 5 needs-verification items, 3 warns and 2 nits. On this route (a diff-gate BLOCK with
   `archChanged`), the driver neither classifies the findings nor sets a T or F note (`coding-pipeline.workflow.mjs:4642-4656`).
   So T-red and F get the findings only from that file and from this handoff. N-3 is my reading of B-1. The
   needs-verification items and the warns are F's and T's to weigh. The driver gap itself is worth filing in Aftersight.
2. **Before 642b:** N-1 (place and plan the agent part, after #700 merges), N-2 (homes for 642b's code and tests) and W-1.
3. **Before 642a's PR:** W-4(c) (file the `FakeHarness` parity issue), W-3(2), and the PR-body line from N-1.

## Patterns referenced

- `crates/holler-adapter-opencode/src/{lib,server,http,exec,tui}.rs` and `tests/support/stub.rs` at 1c3ae10 (642a as built).
- `docs/adr/ADR-0021.md` §5 and §11 (437-453); `crates/holler-hub/src/circuit/dispatch.rs:81` (`send_prompt`);
  `crates/holler-body/src/http_attach_driver.rs:38-40` (`prompt_async`).
- Issues #642 (amended 2026-10-09, 17:40 MDT), #633 (decision 8), #700, #695 and #696; `docs/research/opencode-pane-spike.md`
  (line 167, and no agent result anywhere).
- `.claude/worktrees/0641-host-adapter/docs/handoffs/641-brief.md` at 0f18b80, and
  `.claude/worktrees/0644-launch-relaunch/docs/handoffs/644-brief.md` at 7195993.
- `scripts/lint.sh` (check 4); the playbook's `coding-pipeline-logic.mjs:1110-1131` and `coding-pipeline.workflow.mjs:4642-4656`
  (the route that brought this pass).
