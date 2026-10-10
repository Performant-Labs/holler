# Handoff-A-dup: Phase 7 - #651 the Herdr display plugin (anti-duplication gate)

**Date:** 2026-10-10
**Branch:** `issue-0651-implementation` (run worktree `<run-worktree>`)
**Diff base:** `3d95aec891e746d183aa9853b54b11bcf91d0670` (origin/main)   **Diff head:** `78a68b49cbafb138b5f5d3d0adb8a5abcd0d87b6`
**Reuse map:** `docs/handoffs/0651-implementation/survey.md` (Reuse & Analogous-Feature map)
**Verdict:** PASS

## Summary

F extended everything the map said to extend and duplicated only the two slices the up-front gate
had already ruled forced: every shared fact is imported — the hub read goes through the merged
one-shot client `holler_hub::control::run` with a public-field `ControlCall`, the reply decodes
through `holler_pane`'s own `PaneReply::into_result` into `Vec<Pane>`, and the sync word calls the
imported `shown_differs` behind a doc-cited, case-for-case mirror of `SessionSync::of` (enforced by
the suite's source scans). The one genuinely new code path — the plugin's minimal Herdr report
client — is the brief's binding A-warn 1 outcome: the adapter's `Request` enum is closed over nine
hub-adapter methods with no report variants (re-verified at
`crates/holler-adapter-herdr/src/protocol.rs:112`), so no reuse existed inside the blast radius, and
the new client mirrors the adapter's transport discipline without sharing its code. Blast radius is
exactly as declared; the three warns below are journalled follow-up material, not rework.

## The five duplication judgments the gate asked for

1. **Minimal Herdr report client vs `holler-adapter-herdr`'s transport — forced, verified.** The
   up-front ruling still holds: `Request` (`protocol.rs:112-135`) offers only `ping`,
   `session.snapshot`, `layout.export`, `workspace.create`, `pane.split`, `pane.send_text`,
   `pane.send_keys`, `pane.read`, `pane.close` — no `report_metadata` variant exists to reuse, and
   `Transport::exchange` takes `&Request`, so the adapter's client cannot send a report at all;
   adding a variant was a `crates/**` edit outside the boundary. The new `src/herdr.rs` is genuinely
   minimal for its job: a **closed two-variant `Method` enum** (no third request is constructible —
   this also backs the allowlist scan), one request per connection, one 2 s deadline over
   connect+write+read on a worker thread with `recv_timeout`, replies capped at 64 KiB (the
   adapter's 16 MiB would be absurd for a report ack), no request/reply bytes in any error. It
   mirrors the adapter's discipline (`bound`/`retry`/`remaining`/`left`, the macOS `InvalidInput`
   socket-timeout tolerance) with its own smaller error type — discipline mirrored, not code
   imported. Warn 1 records the maintenance coupling this leaves behind.
2. **Sync word vs `SessionSync::of` — a doc-cited mirror, not a second rule.** `report.rs`
   `sync_word` is structurally identical to `crates/holler-cli/src/pane/list.rs:230-240`
   (`at <= 0 || no session of record` → `-`; `shown_differs` → `mismatch`; else `ok`, with `None`
   read as the home screen inside the one imported rule at
   `crates/holler-pane/src/reconcile.rs:179`), its doc names `SessionSync::of` and the path, and
   `tests/display_only_allowlist.rs::sync_comes_from_shown_differs_and_cites_the_cli_cells` makes
   the cite and the no-raw-`shown`-comparison rule standing guards. `report.rs` is the only source
   that names `shown_differs`. The lowercase `mismatch` (vs the CLI's loud `MISMATCH`) is the
   brief's own token vocabulary, presentation differing per medium — not drift. The 3-of-4 test
   case coverage is T's journalled suite-widening follow-up, not a gate finding.
3. **hold/project/pos tokens vs `list.rs` cell semantics — mirror, no re-derivation.** `pos` is the
   `GridPos` Display the CLI row prints; `project` is `host.cwd`; `shown`/`driven` come from
   `last_observed`; `hold_word` maps the imported `Hold` to the same `none`/`parked`/`drained`
   words `list.rs:262-268` prints. Importing the CLI's own helpers would have inverted the layering
   (a plugin depending on the whole CLI lib+bin), which is exactly why the map said *mirror*. Two
   micro-notes folded into warn 2: token values are sent verbatim without `text_value`'s
   terminal-safety escaping (the implementer pane's journalled finding 6), and an empty stored
   string renders `-` where the CLI cell would print a quoted `""` — degenerate display cases on a
   display-only surface, recorded for the follow-up, not drift from the plan.
4. **JSON framing vs `PaneReply`/`Pane` — decoded through the crate's types.** `read_registry`
   builds `ControlCall { method: "pane/list", params: None, timeout }` (the struct's fields are
   `pub` precisely for this; the up-front review's Ruling 3 named the construction), calls
   `holler_hub::control::run`, decodes the result as `PaneReply`, goes through
   `PaneReply::into_result`, then `Vec<Pane>` — zero new framing, no re-defined record shapes.
   `PaneId`/`PaneName`/`GridPos`/`Hold`/`LastObserved` are all `holler-pane`'s own types.
5. **Test fakes vs `holler-pane-testkit` — no testkit fake existed to duplicate.** The testkit's
   `FakeHerdr` (`crates/holler-pane-testkit/src/herdr.rs`) is an in-memory `HerdrPort`
   (ensure_pane/send_text/read/close/snapshot/version) — no socket, no JSON lines, no
   report_metadata; `pane_store.rs` fakes `PaneStore` in-process. Neither can stand in for the
   wire-level peers the plugin needs. The suite **reuses what the map named** — the
   `holler_pane_testkit::fixture::sample_pane` fixtures build every fake registry pane — and
   authors its own `FakeSocket` (one line in, one line out, captured request log) for the two
   sockets. The adapter's wire-level fake (`crates/holler-adapter-herdr/tests/wire_herdr/`) is
   test-scoped inside that crate's `tests/` tree and not importable from another crate. Warn 3
   records that the workspace now holds two wire-level socket fakes.

## Drift, blast radius, parallel-path checks

- **Plan conformance:** display only (allowlist scan + closed `Method` enum); unknown-never-stale
  with active `unknown` re-report under the same source; `ttl_ms` on pane reports **only**
  (`report.rs` builds the workspace params without it; both socket tests assert the split) — the
  four binding A-warns are each honoured as written. Actions are argv arrays of the five merged
  pane verbs the brief listed; event hooks on `pane.created`/`layout.updated` run the plugin binary's
  `refresh`; the crate follows the house member shape (`version.workspace`, `[lints] workspace =
  true`, machete-clean deps). No `unsafe`, no `#[allow]` in `src/**`, no `process::exit`.
- **Blast radius:** `git diff 3d95aec..HEAD --name-only` is `plugins/herdr-holler/**` (crate,
  manifest, src, tests), run handoffs, root `Cargo.toml` (the members line plus the three-line
  comment documenting the declared exception — the one functional line is exactly as approved),
  `Cargo.lock` (the new package entry only), and `.aftersight/pipeline.config.json`. **No
  `crates/**`, no `docs/research/**`, no scripts.** The config chore (`0d2a03d`) is byte-for-glob
  what `decisions.md` journals: `paths.test` gains `plugins/*/tests/**`, `paths.production` gains
  `plugins/*/src/**`, `plugins/*/Cargo.toml`, `plugins/*/herdr-plugin.toml` — the operator-approved
  unblock of implement attempts 1–2, predating the implementer commit, and nothing else in that
  file moved.
- **No parallel paths for later stories:** no second pane registry (`Reporter::reported` holds only
  the Herdr pane ids of the last good read, to re-report `unknown` — it is not a registry and stores
  no records); the hub client is the one merged client; the Herdr client is the forced one. The
  known gap that actions cannot yet carry their pane operand is `crates/holler-cli` wiring owned by
  #649, correctly not reached into from here.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `plugins/herdr-holler/src/herdr.rs:99-299` | The new client's `Exchange` is a structural twin of `crates/holler-adapter-herdr/src/transport.rs`'s `Exchange` (worker thread + deadline + `bound`/`retry`/`remaining`/`left`, same macOS `InvalidInput` rule). Forced and sanctioned by A-warn 1, but the two implementations now evolve separately and can drift silently (the reply caps already differ by design: 64 KiB vs 16 MiB). The up-front review asked that the escalation be noted in the crate's docs; `herdr.rs` cites the mirroring but not the "second consumer → move `report_metadata` into the adapter crate" escalation. | Add that one doc line to `src/herdr.rs` (or the crate docs) now; if any second display/report consumer appears, extract `report_metadata` into `holler-adapter-herdr` and delete the plugin's client. |
| 2 | warn | `plugins/herdr-holler/src/report.rs:97-103` | The cell-semantics mirror omits `text_value`'s terminal-safety escaping: stored strings (`host.cwd`, session names) are sent as token values verbatim, and an empty string renders `-` where the CLI cell prints a quoted `""`. Whether Herdr's sidebar escapes token values is unverified (the implementer pane's finding 6, journalled). | Follow-up story: verify Herdr's sidebar escaping against a real binary; if it does not escape, apply the CLI's `text_value` rule (quoted `{:?}` form) in `word()` and pin the empty-string case either way. |
| 3 | warn | `plugins/herdr-holler/tests/report_contract.rs:85-193` | The workspace now holds two wire-level one-line-in/one-out Unix-socket test fakes: the adapter's `crates/holler-adapter-herdr/tests/wire_herdr/serve.rs` and this suite's `FakeSocket`. Both were unavoidable (test modules are not importable across crates; the testkit has no socket fake), and the fixtures the map named are reused. | If a third crate needs a wire-level Herdr or control-socket fake, extract a shared socket fake into `holler-pane-testkit` first and converge both existing fakes on it. |

## Notes for F

None — no rework. The three warns are doc-line and follow-up material for O to schedule; none is a
parallel path the map called for extending.

## Patterns referenced

1. `crates/holler-adapter-herdr/src/protocol.rs:112-135` + `src/transport.rs` — the closed `Request`
   enum and the transport discipline the plugin's client mirrors (A-warn 1's forcing fact,
   re-verified).
2. `crates/holler-cli/src/pane/list.rs:205-268` — `SessionSync::of` and the hold words the plugin
   mirrors case for case.
3. `crates/holler-hub/src/control.rs:66-70,441` + `crates/holler-pane/src/reply.rs:94-102` — the
   one-shot client and reply decode the read path reuses unchanged.
4. `crates/holler-pane-testkit/src/herdr.rs`, `src/pane_store.rs`, `src/fixture.rs` — the in-process
   fakes (not duplicated) and the `sample_pane` fixtures (reused).
5. `docs/handoffs/0651-implementation/handoff-A.md` (up-front PASS, warns 1-4) and
   `decisions.md` (the operator-approved config chore the diff's fourth file is).
