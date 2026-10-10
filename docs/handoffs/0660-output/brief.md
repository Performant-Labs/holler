# Brief: #660 — the `--format=json` envelope module and its conformance suite

Repo: `Performant-Labs/holler`. Issue: #660 (epic #633, wave 3). Rigor: **in-session**. UI surface: no
(D and U are N/A). Branch: `issue-0660-output` (plugin-named) from `origin/main` at `519947a`. Run
worktree: `~/Projects/holler/.claude/worktrees/0660-output`; session root: the primary
`~/Projects/holler` (re-rooted off `agent-c4r1` — see decisions.md).

## Objective

Make `crates/holler-cli/src/output.rs` and its suite satisfy #660's acceptance **as main stands today**:
the module is already built (#670, #676 — see `docs/handoffs/660/survey.md`), so this run delivers the
**conformance layer**: the #638 checker wired over the module's outputs, the missing goldens, and the
signature pins — plus any conformant fix the suite exposes in `output.rs` itself.

## Acceptance criteria (from the issue, read against current main)

- [ ] Golden tests for a success, a refusal and a usage error in both formats, asserted through
      `holler_pane_testkit::envelope::check_envelope` (the #638 helper — the same helper every verb
      story's tests use).
- [ ] NDJSON (`emit_stream` in JSON mode): each line parses on its own, through `check_ndjson`.
- [ ] A test forces a diagnostic in JSON mode and asserts stdout still parses as one envelope
      (banner/diagnostic on stderr, envelope intact on stdout — process level is legitimate).
- [ ] A table-driven test asserts the exit code is the same in both formats (extend the existing
      `ALL_CODES` table with checker assertions where it adds rule coverage).
- [ ] A golden of an envelope carrying a `GridPos` serializes `{"row":R,"col":C,"pos":"rRcC"}`,
      row first (epic decision 7).
- [ ] A compile test pins the #637-fixed signatures (`emit`, `emit_stream`, `emit_usage_error` and
      the fixed types) — the module compiles against them unchanged.
- [ ] `cargo fmt` clean, clippy clean, no new `unsafe`, full CI command green.

**Exit codes — recorded resolution:** the issue's line "0 ok, 1 refused or failed, 2 usage" predates
ADR-0021 §9, which #676 landed and the #638 checker itself enforces (`class_of(code).exit_code()`):
0 ok, 1 runtime failure, 2 usage, **3 refusal**. The ADR wins (the epic's contract section is fixed
under it); the tested property is **parity between formats**, which both texts state.

## RED policy (test-first, honestly)

T authors the suite above against current `output.rs`. Any check that faults **is** the RED and names
F's work. **If the suite is green on contact**, T records that verdict with evidence in
`handoff-T-red.md` and the run **stops for the operator** — no sham F stage. (Survey probes expect
green or near-green; the checker may still fault on a rule the local helper never checked.)

## Boundaries (operator-set, stricter than the defaults)

- **F:** `crates/holler-cli/src/output.rs` only. **T:** `crates/holler-cli/tests/**` only. Extend
  `pane_verbs/output_api.rs`, or add a new module under `pane_verbs/` wired into the existing
  `pane_verbs` target's `main.rs` (an **autotests=false caveat: a new top-level `tests/*.rs` file
  silently does not build** — top-level targets are the declared ones only); process-level tests
  under `pane_verbs/process/` where a real binary is needed. **No `Cargo.toml` edits**
  (`holler-pane-testkit` is already a dev-dep).
- Do not touch `holler-pane/**`, `holler-pane-testkit/**`, `cli.rs`, `main.rs`, verb files, or any
  Aftersight/opencode config — the parallel session (c3r1) owns the sibling stories in this repo.
- No mutation testing. One cargo at a time. Never touch a live fleet, a running pane or a real Herdr
  session; never print or commit a credential.

## Handoffs

`docs/handoffs/0660-output/` (**in the run worktree**, not the primary, not `docs/handoffs/660/`):
`survey.md`, `decisions.md`, `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`,
`handoff-T-green.md`, `handoff-A-dup.md`, `handoff-S.md`.

## Verification

Narrow: `cargo test -p holler-cli`. Full (pre-merge, as CI runs it):
`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`.
