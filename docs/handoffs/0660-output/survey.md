# Survey: #660 — the `--format=json` envelope module and its conformance suite

Run: issue #660 (epic #633, wave 3), rigor **in-session**, no UI surface. Branch `issue-660-implementation`
from `origin/main` at `519947a`. Run root: the `agent-c4r1` worktree (detached → branch cut at T-red).

## What the issue asks

Fill `crates/holler-cli/src/output.rs` — which #637 pre-created with fixed types (`Format`, `Envelope`,
`ErrorBody`, `ErrorCode`), fixed signatures (`emit`, `emit_stream`, `emit_usage_error`) and a minimal
body — to full conformance **without changing a signature**, plus its conformance suite.

## What main already carries (the survey's main finding)

The story's premise is dated: the module is already built, by the closed sibling stories of the same
epic. #660's real delta is the **conformance layer** named by its acceptance bullets, not a from-scratch
module.

| Landed | Commit | What it already delivers |
|---|---|---|
| #670 (CLOSED) | `55dba00` | `output.rs` complete: routing (text→out/err, json→one envelope on out, nothing on err), `emit`/`emit_stream`/`emit_usage_error`, NDJSON one envelope per line flushed per line, `one_line` messages, write-failure rules (`settle`), `scan_args`/`usage_message`; and `tests/pane_verbs/output_api.rs` — 20 tests incl. golden success/refusal/usage in both formats, NDJSON per-line parse, exit-parity tables |
| #676 (CLOSED) | `3f9fbf2` | Exit classes per **ADR-0021 §9**: 0 ok, 1 runtime failure, 2 usage, **3 refusal**, same in both formats, via `holler_pane::error::class_of` (no central CLI table) |
| #637 (CLOSED) | — | `GridPos` serde `{"row":R,"col":C,"pos":"rRcC"}` row-first (`crates/holler-pane/src/grid.rs:129`); `ErrorCode` validated by the workspace's one validator `is_valid_code` |
| #638 (CLOSED) | — | `holler-pane-testkit` `envelope::{check_envelope, check_ndjson}` — the 13-rule + NDJSON conformance checker (ADR-0021 §5: it cannot depend on the CLI's `Envelope` type) |
| #643 (CLOSED) | `efd9a00` | Roster/pane verbs live; roster's envelope path is in `main.rs` (`resolve_format`, `json_explicit`), outside this story's file |

Verified conformant already (read-only probes, no tests written): envelope key set/order/compactness,
`usage`-coded envelope at exit 2, NDJSON `NotLastFailure` semantics, GridPos row-first serde, exit
parity through `ALL_CODES`.

## What is actually missing (the deliverable)

1. The #638 checker is **not wired to the CLI's output tests**: `output_api.rs` asserts through the
   local `verb_harness::one_envelope` (a parse-and-shape helper), not
   `holler_pane_testkit::envelope::check_envelope`/`check_ndjson`. Acceptance bullet 2 names the #638
   helper as the authority ("the same helper every verb story's tests use").
2. No golden of an **envelope carrying a `GridPos`** (issue scope, epic decision 7).
3. No **forced-diagnostic-in-json-mode** test asserting stdout still parses as one envelope
   (`process/stub.rs:199` covers a `--debug bogus` refusal in json mode; the banner-diagnostic +
   envelope-on-stdout combination is untested through the checker).
4. No explicit **compile test pinning the #637-fixed signatures**.

## Reuse & Analogous-Feature map

- Closest existing suite: `crates/holler-cli/tests/pane_verbs/output_api.rs` — module of the declared
  `pane_verbs` test target. **Recommendation: extend it** (and, where a real binary is needed,
  `tests/pane_verbs/process/`). `autotests = false` in `crates/holler-cli/Cargo.toml` means a **new**
  test file would silently not build — and `Cargo.toml` is outside both T's glob (`crates/*/tests/**`)
  and the story's blast radius. `holler-pane-testkit` is already a dev-dependency (no manifest change).
- Analogous checker consumer: `holler-pane-testkit`'s own `tests/envelope_test.rs` (the checker's
  self-tests) — the usage pattern to copy.

## Boundaries (operator, this run)

- F edits **only** `crates/holler-cli/src/output.rs`. T edits **only** `crates/holler-cli/tests/**`
  (expected: `pane_verbs/output_api.rs`, `pane_verbs/process/*`).
- Nobody touches: `Cargo.toml` (any), `holler-pane/**`, `holler-pane-testkit/**`, `cli.rs`, `main.rs`,
  verb files — the parallel session (c3r1) owns the sibling #633 stories in this same repository.
- No mutation testing (Holler rule): RED comes from real gaps against current `output.rs`, never from
  editing tracked files to break them.
- One cargo at a time (shared `target/` via `worktree.linkDirs`); CI command is
  `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`, narrower
  `cargo test -p holler-cli` during stages.
