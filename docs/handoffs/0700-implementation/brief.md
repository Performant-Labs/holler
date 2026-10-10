# Brief — #700 `feat(pane)`: `Pane.opencode_agent`, its guard and the `--agent` spec flag

- **Issue:** #700 (an amendment to #637's contract; epic #633 decision 8, from pfleet#372)
- **Run:** `0700-implementation`, worktree `<run-worktree>`, branch `issue-0700-implementation`
- **Rigor:** `in-session` (issue text; no outside-model gates — brief/diff dual-review N/A)
- **UI surface:** none → D and U are N/A (recorded, not silent)
- **Test command:** `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`
  (the configured suite; `cargo fmt --check` + `cargo clippy` clean beside it)
- **Handoffs:** `docs/handoffs/0700-implementation/` (this brief, survey, decisions journal,
  `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A.md`, `handoff-A-dup.md`,
  `handoff-S.md`)
- **Rules:** issue #700's Rules section + the epic's: Conventional Commits, `cargo fmt`, clippy clean,
  no new `unsafe`, never a live fleet / running pane / real Herdr session, never print or commit a
  credential. No mutation testing (operator, this run). On green the pane self-merges (squash) after
  CI and gates pass, per the in-session rule.

## Objective

Add the OpenCode agent a pane's hub-delivered messages run as, to the pane record and to the spec
flags, as one small additive change that merges first so no later story edits `holler-pane/**` for
this field (#642 applies it, #644 uses/validates it, #647 reports on it). `None` means the server's
default agent. The key is a name, never a secret (I7).

## Survey summary (see survey.md for the map)

Everything extends an existing analogue: the `AgentKey` guard sits beside `EnvVarName` in
`crates/holler-pane/src/argv.rs`; the field sits after `model` on `Pane` (`pane.rs`) and
`ProfileSpec` (`profile.rs`) with the standard optional-field serde attributes; the `--agent KEY`
flag extends `SpecFlags`/`SpecValues`/`validate()` in `crates/holler-cli/src/pane/args.rs` (beside
`--role`; NOT literally `src/cli.rs` — the issue's path names are approximate, survey deviation 1-2);
the overlay extends `effective_spec` in `launch.rs`; `--from-current` extends `spec_from_pane` in
`profile_snapshot.rs`; the testkit builders (`fixture.rs`) gain `None`; the surface fixture's
`--model/--effort/--role` launch line gains `--agent orchestrator`; ADR-0021 §1's Pane table gains
one dated row. **No new files; no `Cargo.toml` edits; no `error.rs` list change** (`agent-key-invalid`
is an open code: `pub const AGENT_KEY_INVALID: RefusalCode = RefusalCode::from_static("agent-key-invalid")`
beside the guard, carried by `PaneError::Refused`; `class_of` already classes it a refusal, exit 3).

## Acceptance criteria (tests first — every box is a T-authored test)

1. - [ ] `AgentKey` accepts `orchestrator`, `feature-implementor`, `mo`; refuses the empty string, a
       string with a space, a newline, `=` or a slash — each with the code `agent-key-invalid`
       (carried by `PaneError::Refused`; the message states the grammar and does not echo the text).
2. - [ ] `AgentKey` serde: serializes as a plain string; a record with an invalid key fails to decode
       (fail closed, the `EnvVarName` precedent).
3. - [ ] A `Pane` and a `ProfileSpec` round-trip through serde with `opencode_agent` set (JSON field
       `opencode_agent`) and with it absent (field omitted when `None`; an old record with no field
       reads back `None`). Field position: after `model` (epic contract order).
4. - [ ] `--agent KEY` parses on `launch` and on `relaunch` (shared `SpecFlags`), typed through
       `AgentKey::parse`; an invalid key is refused with `agent-key-invalid` **before anything else
       runs** (validate is step 1; no port call, no record write — assert via the rig's call log).
5. - [ ] The flag overlays like its siblings: given → replaces; absent → base spec's value; no base →
       `None`. (`effective_spec`.)
6. - [ ] `profile create --from-current` copies `opencode_agent` from the fake `PaneStore`'s records
       (`spec_from_pane`), like `model`.
7. - [ ] `cli-surface.txt` shows the flag exactly once (the extended `--model … --role agent` launch
       line still parses; the fixture test enforces parse + leaf parity as always).
8. - [ ] ADR-0021 §1's Pane table carries one dated row for the field (docs check; no other ADR prose
       changes).
9. - [ ] Every existing test passes unedited-in-behaviour: compile-forced `opencode_agent: None`
       lines in full literals only (`profile_scope.rs` helper, testkit `fixture.rs`,
       `conformance/pane_store.rs`); every existing outcome unchanged.
10. - [ ] `cargo fmt --check` and `cargo clippy` clean (workspace warnings policy as configured); no
        new `unsafe`.

## Boundaries (the epic's hot-spot ownership, corrected paths)

- **F may edit:** `crates/holler-pane/src/{argv.rs, pane.rs, profile.rs, tx_launch.rs (the record
  literal), profile_snapshot.rs}`, `crates/holler-cli/src/pane/{args.rs, launch.rs (overlay),
  profile_scope.rs (helper literal only)}`, `crates/holler-pane-testkit/src/{fixture.rs,
  conformance/pane_store.rs}`, `crates/holler-cli/tests/fixtures/cli-surface.txt` (one line),
  `docs/adr/ADR-0021.md` (one row), and these handoffs.
- **F may not edit:** `error.rs` (no closed-code list change), `ports.rs` (no `HarnessPort` change),
  `profile_diff.rs` (the drift vocabulary is #647's), any `Cargo.toml`, `CHANGELOG.md` (outside the
  issue's blast radius — journalled, not silently skipped), the hub crate, any other verb file.
- **T may edit:** `crates/holler-pane/tests/{argv_env_test.rs, records_test.rs,
  profile_snapshot_test.rs}` and `crates/holler-cli/tests/pane_verbs/**` (existing targets only —
  `autotests = false`), plus the one surface-fixture line with F. Nothing in `src/`.
- Verification uses the test kit's fakes and temporary directories only. One `cargo` at a time.

## Open question put to A (the plan gate rules on it)

**Does `tx_launch::record()` write `spec.opencode_agent` now, or `None`?** The issue's purpose line
("merges first so no later story edits `holler-pane/**`") + the silent-drop landmine (a relaunch would
reset a stored key to `None`) + the C1 precedent (model/effort/env/context are recorded mechanically
by the same literal) argue for **copying from the spec now** — #644's "last part" then reduces to the
CLI-side project-defined-key validation, and #642's to the adapter application. ADR-0021's deferred
list words the recording as #644's; both readings are recorded in the journal. The acceptance tests
above do not pin either answer; A's ruling is journalled and F implements it.

## Non-goals (recorded, not silent)

- Applying the key anywhere: no `HarnessPort` change, no OpenCode adapter change, no
  `create_session`/`serve` parameter (#642).
- Validating the key against a project's `.opencode/agents/` (#644's last part).
- `profile_diff`/`profile show` comparison and the `agent-cannot-dispatch` doctor code (#647).
- CHANGELOG entry, version bumps, protocol v2, golden files — untouched.

## Delivery

Rebase on `origin/main` before the PR (main moves fast); Conventional-Commit subject
`feat(pane): Pane.opencode_agent, its AgentKey guard and the --agent spec flag`; `Co-Authored-By`
trailer per hooks; PR to `main` closing #700, body carrying the repo's AI disclosure; CI green
(ubuntu + macos matrix), then the pane self-merges (squash, branch deleted) and reports.
