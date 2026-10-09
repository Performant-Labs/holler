# Decisions — #647 the reconcile engine and `holler pane doctor`

## A (Phase 3, up-front plan review) — 2026-10-09T17:23:37-06:00
- **Decided:** PASS on docs/handoffs/647-brief.md at a98258a (sha256 0f98bbc7b52e5c78...), with 0 blocks and 7 warns (see
  handoff-A.md).
  - The objects and layers are right:
    - a pure engine and finding types in `holler-pane` (ADR-0021 §5), working only through `Ports`;
    - a thin verb in its own file;
    - closed codes only, with exit codes from `class_of`;
    - scoping through `ProfileScope::resolve`;
    - record writes only on change (`panes/mod.rs:27-28`).
    No frozen file is edited, and the only copy of something already on `main` is the clock read (W-3).
  - W-1 (the most consequential): reconcile becomes the writer of `harness.health` and defines what `last_observed.at` and
    `shown: None` mean. #643, #645, #646 and #648 read those fields, and #643 reads them differently. The meanings would
    live only in this brief, because AC 28 forbids any other ADR-0021 edit.
  - W-2: the SHOWN-vs-record comparison should be one `pub` pure function in `reconcile.rs`, the only unfrozen place the CLI,
    the hub and the engine can all import.
  - W-3: use `holler_proto::clock::now_millis()`, not a hand-written `SystemTime::now()` read.
  - W-4: make the doctor-command builder `pub`, since #644 and #663 build the same command line in flight. On C-8, never
    print raw OpenCode, Herdr or tmux commands.
  - W-5: three vocabulary problems:
    - `FindingKind` keeps two sources (serde derive and `code()`);
    - `ObservedHealth` spells an observation differently from the finding codes and the record;
    - `observe-failed` has two meanings.
    Also, #650's findings have no Forward-compat line.
  - W-6: test placement and shared test helpers:
    - AC 25 belongs in `holler-pane/tests/`;
    - reuse the rig that merges first;
    - AC 26 should call `try_parse`;
    - one text-escaping helper.
  - W-7: two blind spots of the frozen ports to record as risks: a foreign TUI on a shared data directory, and strays that
    accumulate.
- **Assumed:**
  - The briefs and A handoffs in `.claude/worktrees/{0643-read-verbs,0644-launch-relaunch,0663-profile-scope-probe}` are the
    current plans of those stories. #644's brief was being amended in its working tree while I read it, so I cite its
    content rather than its line numbers.
  - None of these plans is merged. I used them as evidence of what is being planned, not as patterns.
  - The issue (#647), the epic (#633), ADR-0021, ADR 0003 and the merged code are the authority.
- **Hedged:**
  - W-1 is a warn, not a block. The ADR is silent on these meanings rather than contradicting the plan, and the frozen docs
    are themselves ambiguous: `pane.rs:188` says "When it was observed", `ports.rs:198` says "if it can tell", and the fake
    says the home screen (`harness.rs:135-136`).
  - This also differs from #642's B-1. That block was an item the ADR deferred to the story by name, while the brief
    forbade the edit. Here the only item deferred by name (the hub timer) is handled.
  - #643's A rated the same `shown: None` question a warn, so the rating is consistent.
  - W-2, W-4 and W-6(b, d) are forward duplication between unmerged plans. There is no dominant pattern on `main`, so per
    the role rule they are warns. At Phase 7 I will reject a second rig or text helper if one is on `main` by then.
  - W-3 is the only finding about an object already on `main`. It stays a warn because the fix is one call and F reads this
    handoff. #644's A rated the same point a warn.
  - W-7(a) rests on the documented fake (one shared data directory) and on `shown_session`'s signature, not on a run of the
    real adapter.
- **Evidence:**
  - Read in full:
    - the brief and its two outside-review results (r1, r2);
    - `holler-pane/src/{lib,ports,pane,error,profile,grid,reply,probe,generation}.rs` and the stubs;
    - `holler-cli/src/output.rs`, `pane/{mod,args,wiring,doctor,relaunch,reset,switch,list,get,launch,close}.rs`;
    - `tests/verb_harness/{mod,parse}.rs` and `tests/pane_verbs/main.rs`;
    - `holler-pane-testkit/src/{lib,fixture}.rs`;
    - `docs/adr/ADR-0021.md` and `docs/adr/ADR-0003.md`.
  - Also read:
    - `holler-hub/src/panes/mod.rs:1-60` and `holler-proto/src/{clock,error}.rs` (excerpts);
    - the sanitizers in `hub_cmd.rs`, `acp_driver/auth.rs`, `log.rs`, `holds.rs` and `adapter-herdr/src/protocol.rs`;
    - `holler-pane/tests/{ports_test,error_test}.rs` (headers and the code-table test);
    - `tests/pane_verbs/process/{docs_rows,stub}.rs` (excerpts);
    - the test kit's public API and op names;
    - `scripts/lint.sh` and `clippy.toml`;
    - `crates/holler-cli/Cargo.toml` and `crates/holler-pane/Cargo.toml`.
  - Sibling plans: `643-brief.md` (Decisions 10-12) and `643/handoff-A.md`; `644-brief.md` (excerpts) and
    `644/handoff-A.md`; `663-brief.md` (excerpts); `642/decisions.md` (for the journal format).
  - Issues read with `gh issue view`: 647, 633 (with comments), 644, 645, 646, 648, 649, 650, 665, 694.
