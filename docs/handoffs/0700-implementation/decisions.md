# Decision journal — #700 (in-session)

## 2026-10-10 9:43 AM MDT — Phase 1 (pre-flight → survey-brief), O

- **Run opened.** Issue #700 (`feat(pane): Pane.opencode_agent, its guard and the --agent spec flag`,
  an amendment to #637's contract; epic #633 decision 8). Rigor `in-session` (issue text), no UI
  surface → D and U are N/A for this run. Pre-flight PASS (0 failing of 15). Run worktree
  `<run-worktree>` on branch `issue-0700-implementation` (the plugin's `issue-NNNN-<slug>` naming; the
  0660 precedent records this owns the naming over the older by-hand rule), port 9000.
- **Survey decision 1 — the issue's file names are approximate; the code's layout wins.** The blast
  radius names `holler-cli/src/cli.rs` and `holler-cli/cli-surface.txt`; the shared spec flags
  (`--model`, `--effort`, `--role`) actually live in `crates/holler-cli/src/pane/args.rs` (#670's
  `SpecFlags`/`SpecValues`), and the surface fixture is
  `crates/holler-cli/tests/fixtures/cli-surface.txt`. "Beside `--model`, `--effort` and `--role`"
  controls; the corrected paths are recorded in the brief.
- **Survey decision 2 — `agent-key-invalid` is an open code, not a closed one.** The closed set of 22
  is frozen (ruling 3, ADR-0021 §9); the guard declares its code beside itself in `argv.rs` via
  `RefusalCode::from_static`, carried by `PaneError::Refused`; `class_of` already classes any
  well-formed open code as a refusal (exit 3). No edit to `error.rs`.
- **Survey decision 3 — the record write in `tx_launch` is the one open question for A.** The issue's
  purpose line ("merges first so no later story edits `holler-pane/**`") asks the field to be plumbed
  spec → record now; ADR-0021's deferred list words "recording `Pane.opencode_agent` in the record
  write" as #644's last part. Brief recommends plumbing it (one line, `spec.opencode_agent.clone()`)
  and puts the question to A; the acceptance tests do not pin either answer.
- **Boundaries recorded (from the issue's Rules).** Edit only the blast-radius files (corrected paths
  in the brief); `cargo fmt`, clippy clean, no new `unsafe`, Conventional Commits, no live fleet,
  running pane or real Herdr session; never print or commit a credential. No mutation testing
  (operator, this run). `CHANGELOG.md` is outside the issue's blast radius and stays untouched; the
  gap is journalled for the operator.
- **Non-goals journalled.** `profile_diff`/`profile show` does not compare the new field (that
  vocabulary is #647's, with the doctor code `agent-cannot-dispatch`); `HarnessPort` carries no agent
  parameter (#642 applies it); the project-defined-key validation is #644's last part.

<!-- reported: 0700-implementation/run 0660 journal consulted for format; placeholders per the public-repo rule -->

## 2026-10-10 ~9:58 AM MDT — Phase 3 (A: plan review), O recording

- **A PASS** (`handoff-A.md`, verdict line at end): the plan is faithful to #700 and ADR-0021,
  extend-not-new at every surveyed analogue; boundaries complete as written.
- **A's DECISION 1 (binding for F) — survey decision 3 resolved: plumb the record write now.**
  `Plan::record` (`tx_launch.rs:314`, the brief's "Engine::record" name was off, file/line right)
  gains `opencode_agent: spec.opencode_agent.clone()` after the `model` line (:349). Reasons: the
  epic's hot-spot exception (`holler-pane/**` for this field is #700 only) makes `None` a dead end no
  later story could fix; the deferred bullet bundles flag+validation+recording and the amendment (the
  later, more specific text) splits them — #644's remainder is the project-defined-key refusal; the
  same literal already copies model/env/context mechanically; and a flag-less relaunch (base from
  `spec_from_pane`) would silently reset a stored key to `None`. T pins the positive path (a launch
  with `--agent` stores the key) in the `pane_verbs` rig.
- **W1 (correction adopted): the definitive full-literal set** is 5 `Pane` (tx_launch record;
  testkit sample_pane; pane_verbs launch.rs:57, launch/rig.rs ~330, list.rs:121) + 5 `ProfileSpec`
  (profile_snapshot spec_from_pane; testkit sample_spec; launch.rs:198 effective_spec;
  profile_snapshot_test.rs:51; pane_verbs launch/rig.rs ~555) — all inside F ∪ T boundaries.
  `profile_scope.rs` (mutation/spread) and `conformance/pane_store.rs` (spreads) need no edit;
  the brief's AC 9 parenthetical is corrected by this entry (boundaries unchanged).
- **W3 resolved by O:** AC 8 (the one dated ADR row) is inspection-checked (S verifies it at
  spec-audit), not a T test — no boundary widening.
- **W4/W5 folded into F's instructions:** the one dated §1 row names the whole delta (field on both
  records, the `--agent` flag, the `--from-current` copy); `profile_snapshot.rs`'s copy-table doc row
  lands with F's edit. W6 optional for T (docs_rows.rs static flag list).

## 2026-10-10 10:09 AM MDT — entering t-red (gate behaviour, baseline)

- **The t-red entry gate runs the suite and refuses on GREEN (fail-closed).** O's first
  `advance --target t-red` was premature: the gate ran the configured suite over the untouched tree —
  GREEN, exit 0, the whole workspace compiling and passing at baseline (warm `target/` link) — and
  refused to advance ("T-red requires a RED contract"). Correct flow confirmed: T authors the failing
  suite first, `stage run --phase t-red` records the RED verdict, then the advance crosses. Two
  earlier shell timeouts (120 s) killed the same gate mid-suite; the per-stage lock's stale rule
  (pid gone → stale → takeover) self-healed, no residue.

## 2026-10-10 ~10:35 AM MDT — Phase 4 (T-red: suite authored, RED confirmed), T

- **Decided — the RED contract is split compile/runtime, both legitimate.** Three targets are
  compile-RED against the missing API (`argv_env_test` E0432 `AgentKey`; `profile_snapshot_test`
  E0560/E0609 `opencode_agent`; `pane_verbs` 8 errors all naming `AgentKey`/`opencode_agent`); two
  targets compile today and fail at the right assertion (`records_test` 3/14 on the
  `unknown field \`opencode_agent\`` refusal — JSON-driven on purpose, per the brief's own wire-form
  framing; `cli_surface_test` 1/3 on `unexpected argument '--agent' found`). Ten new tests, five
  extensions, all inside T's boundary; staged by explicit path.
- **Decided — DECISION 1 is pinned from both ends in the rig.** Positive: a launch with
  `--agent feature-implementor` must store `Some(...)` in the record
  (`launch_with_agent_stores_the_key_in_the_record`); None-half: the flagship expected record gains
  `opencode_agent: None` (no flag, no base); landmine: `relaunch_without_agent_keeps_the_stored_key`
  (base = `spec_from_pane` of a record carrying a key) fails if `Plan::record` writes `None`.
- **Hedged — AC 1's "message states the grammar" pinned lightly** (`contains("agent")`, the
  `EnvNameInvalid` subject-naming precedent) plus per-case no-echo; exact wording left to F.
- **Assumed — the guard's shape mirrors `EnvVarName`** (`parse(&str) -> Result<Self, PaneError>`,
  `as_str`, `Debug`, root re-export, serde through `deserialize_parsed`). If F deviates, the
  compile-RED tests say so at T-green and the repair is a test edit, not a redesign.
- **Evidence — A's §3 literal table over-lists T's compile-forced sites** (second correction, after
  W1): `launch.rs:57`, `rig.rs` ~330 and `list.rs:121` are spreads (`..sample`/`..pane(name)`) — no
  edit forced; the only full literal in T's tree is `launch_spec()` (~555), which gained the `None`
  line. `verb_harness/parse.rs` (`SPEC_FLAG_SETS`) and `docs_rows.rs`'s static flag list are outside
  T's boundary → `--agent` parse coverage rides the verb-level tests (W6 optional, also out of
  reach). Green sanity at baseline: `--lib` 12 passed, `profile_verbs` 44, `records_test` 11/11
  pre-existing, `cli_surface_test` 2/3.

## 2026-10-10 ~10:20 AM MDT — Phase 4 (T-red), O recording

- **T authored the suite; RED CONFIRMED** (`handoff-T-red.md`): 10 new tests + 5 extensions across
  8 files — guard tests in `argv_env_test.rs`, serde round-trips in `records_test.rs`, flag/refusal
  and DECISION-1 pinning in `pane_verbs/{launch.rs, relaunch.rs, launch/guards.rs, rig.rs}`,
  `--from-current` copy in `profile_snapshot_test.rs`, one surface-fixture line. RED split: 3
  targets compile-RED on the missing `AgentKey`/`opencode_agent`/`--agent`; 2 runtime-RED
  (`unknown field opencode_agent`; `unexpected argument '--agent'`). Baseline sanity was green
  around the RED (pre-existing tests in the runtime-RED targets still pass).
- **T's correction of A's §3 (journalled):** in T's tree only `launch_spec()` (~555) is a full
  `ProfileSpec` literal; `launch.rs:57`, `rig.rs` ~330, `list.rs:121` are spreads. No boundary impact.
- **`verb_harness/parse.rs` `SPEC_FLAG_SETS` left without `--agent`** (outside T's pane_verbs glob;
  A's W6 already optional; no test demands it — parse coverage rides the verb tests). Recorded, not
  silent; S may inspect.

## 2026-10-10 10:28–10:39 AM MDT — Phase 5 (F attempts 1-2), O recording

- **F attempt 1 STAGE FAILED (refused/permission-denied), 233 s, 37 turns, no code written.** The
  claude-cli executor ran in `dontAsk` with F's write boundary from the role doc's frontmatter
  (untracked local provisioning) — and the allow-list was the Aftersight release's own shape
  (`server/src/**`, `shared/src/**`, `webapp/src/**`), no Holler path at all, so every Edit/Write on
  `crates/**` was denied. F behaved correctly: no bash bypass, only its handoff — with the complete
  patch design. **Run-blocking oddity #1 (for the report): the pipeline install carried the
  Aftersight workspace template into Holler's role boundaries.**
- **Boundary repaired by O (config, not code):** Holler production paths added to the F role doc's
  `permission.edit` (`crates/*/src/**` + `docs/adr/**` allows, both paired forms; `crates/*/tests/**`
  denies LAST so T's suite stays F-denied). No tracked file touched; other role docs left as
  provisioned (their stages dispatch through O, so their boundaries never bind here).
- **F attempt 2 STAGE OK (658 s, 63 turns, out 23,771 / reasoning 4,022 / cache-read 2,243,398 /
  cache-write 99,667, cost n/a):** the exact designed patch, 91 insertions across 10 files — the
  `AgentKey` guard + `AGENT_KEY_INVALID` in `argv.rs`, the field after `model` on `Pane` and
  `ProfileSpec`, DECISION 1's record line in `Plan::record`, the `--from-current` copy + doc row,
  `--agent` in `SpecFlags`/`SpecValues`/`validate`, the `effective_spec` overlay, testkit `None`
  lines, the one dated ADR row (self-describing per W4), and deviation 1 (`lib.rs` root re-export,
  inside the issue's `holler-pane/**` blast radius). No Cargo.toml, no CHANGELOG, no error.rs.
- **F flags one malformed test literal (T's to repair in T-green):** the serde fail-closed test's
  `r#"""#` is a lone quote (a JSON syntax error), not the intended empty-string case; the repair is
  `r#""""#`. F did not work around it in code — correct. F also notes the ADR's old deferred bullet
  (  ~:677) still words the recording as #644's; the one-row constraint kept it, the new row states
  the launch record writes the field (S may inspect).

## 2026-10-10 10:45–11:05 AM MDT — Phase 6 (T-green), T recording

- **Decided:** F's malformed-literal flag upheld and repaired by T. Delimiter-verified
  `r#"""#` = one quote → a serde_json lex error that can never reach `AgentKey::deserialize`
  (the RED was compile-RED, so the runtime defect was invisible at Phase 4); repaired to
  `r#""""#` (JSON `""`). One literal, `argv_env_test.rs:164`, no behavior change.
- **Decided:** two more T-owned repairs — my RED lines in `pane_verbs/launch.rs:124` and
  `relaunch.rs:567` exceeded rustfmt's width (the only fmt delta of the whole run vs base
  `3d95aec`, proven by diffing `cargo fmt --all --check` file-sets base vs HEAD in a throwaway
  worktree, since removed); hand-rewrapped to rustfmt's exact form rather than a workspace-wide
  `cargo fmt` (would have touched ~180 pre-existing out-of-boundary hunks).
- **Hedged:** two suite flakes during verification — `interrupt_test` (6 fails under full-workspace
  load, 11/11 in isolation) and `body_run_test::fresh_hello_and_presence_on_every_reconnect`
  (fixed port 41918 in TIME_WAIT; the #242/#259 race class named in the test's own docs; passes
  after the wait and in the final full run). Ruled environmental, not 34ca660 — it touches no
  hub/body/interrupt path, and the final configured run was exit 0. Follow-up issue-worthy.
- **Assumed:** the RED→GREEN transition stands in for remove-the-behavior spot-checks (mutation
  testing barred for this run by the operator).
- **Evidence:** configured suite exit 0 (all binaries ok); clippy `--workspace --all-targets`
  exit 0 silent; no `unsafe` in the two crates at HEAD or base; `--agent` ×1 in the surface
  fixture; ADR-0021 exactly `1 insertion(+)`. `GREEN: CONFIRMED` (`handoff-T-green.md`).

## 2026-10-10 ~11:05 AM MDT — Phase 6 (T-green), O recording

- **GREEN CONFIRMED** (`handoff-T-green.md`): configured suite exit 0 (pane_verbs 219/219,
  argv_env_test 8/8, records_test 14/14, cli_surface_test 3/3). Tier 2 clean: clippy
  `--workspace --all-targets` silent, fmt file-set byte-identical to base, no `unsafe` at HEAD or
  base, `--agent` exactly once in the surface fixture, ADR-0021 exactly one dated insertion. F
  touched zero test paths (`--stat` verified).
- **T's repairs (its own suite, both legitimate):** the `r#"""#` lone-quote literal → `r#""""#`
  (F's flag upheld — a JSON lex error never reached the guard), and two RED-authored lines
  re-wrapped to rustfmt's exact form (the run's only fmt delta). T declined F's two optional
  test-side items (A's W6-optional; coverage rides the verb tests).
- **Two environmental flakes noted (pre-existing, out of this story's paths, follow-up
  issue-worthy):** `interrupt_test` under full-workspace parallel load, and `body_run_test`'s fixed
  port 41918 TIME_WAIT race (#242/#259 class). Both pass isolated and in the final full run.
- Mutation testing barred by the operator for this run; the RED→GREEN transition stands in for
  remove-the-behaviour spot-checks (T hedged this in its handoff).
