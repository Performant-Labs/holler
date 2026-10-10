# Handoff-A: #651 the Herdr display plugin (up-front plan review, Phase 3)

**Date:** 2026-10-10
**Run:** `0651-implementation`, branch `issue-0651-implementation` from `origin/main` at `3d95aec`
**Brief reviewed:** `docs/handoffs/0651-implementation/brief.md` · **Reuse map:** `docs/handoffs/0651-implementation/survey.md` (Reuse & Analogous-Feature map) · **Wireframe:** N/A (no UI surface; D/U recorded N/A)
**Verdict:** PASS (4 `warn` findings, 0 `block`)

## Summary

The plan is sound against the issue's scope and the codebase's actual seams, and every load-bearing
fact the survey claims was verified in source: `pane/list` is served on the control socket and its
`data` is every `Pane` sorted by name; `holler_hub::control::run` is the merged one-shot client the
CLI itself uses; `Wiring::connect` still hands out `Unwired`, so the plan is right to read the
control socket directly rather than shell to `holler pane list`; `shown_differs` is importable and
is the one SHOWN/DRIVEN rule; the manifest event-hook names `pane.created`/`layout.updated` are the
schema-valid dotted forms. The declared blast-radius exception (one line, `"plugins/*"` in root
`members`) is **approved** — without it the acceptance tests run nowhere. The only real duplication
is two small slices the blast radius *forces* (a minimal Herdr report client, and the SYNC gate
wrapper), both recorded below as `warn` with the discipline to mirror.

## Ruling 1 — plan vs the issue's scope and acceptance

Sound. Display-only is enforced the only way it can be (Herdr does not stop a plugin from mutating —
spike §12.2), by code plus a source-scanning test; the per-pane facts (project `host.cwd`, SHOWN
`last_observed.shown`, DRIVEN `last_observed.driven`, hold) all exist on the verified `Pane` record
(`crates/holler-pane/src/pane.rs:226-255`) and one `pane/list` read yields all of them; the offered
verbs (`switch`, `reset`, `doctor`, `park`, `unpark`) are all merged today (#643–#647 on the
branch's history) and `close`/`say` are correctly not offered; hub-unreachable maps to
`ControlError::NoLiveHub` from `run` (`crates/holler-hub/src/control.rs:470-473`) and the plan
actively reports `unknown` under the same `source`. No migrations, no protocol changes, no edits
under `crates/**` — a new leaf. Token budget fits the SCHEMA limits (6 pane tokens used of ≤16;
names pass `^[A-Za-z0-9_-]{1,32}$`).

## Ruling 2 — the declared blast-radius exception: APPROVED

Root `Cargo.toml` line 2 is `members = ["crates/*"]`; the plan adds `"plugins/*"` — one line. It is
**required**, not convenient: the pipeline's unit command is `cargo test --workspace -- --skip
roster_stays_accurate_under_concurrent_body_load` (verified in `.aftersight/pipeline.config.json`), and CI runs the same — without
membership the crate's tests, including the display-only scan that is this story's core safety
test, build and run nowhere. The alternatives are worse: a standalone crate with a `[workspace]`
opt-out leaves the acceptance untested in CI; placing the crate under `crates/` violates the epic's
directory scheme and puts a plugin in the sibling stories' hot zone. The root manifest is already a
pipeline "production" path, and the exception is declared in the brief and the PR body. The glob
form follows the house pattern (`crates/*`); the exact-path form (`"plugins/herdr-holler"`) is an
equally one-line, narrower alternative O may prefer — both acceptable.

## Ruling 3 — the read path: RIGHT SEAM

`holler_hub::control::run` (`crates/holler-hub/src/control.rs:441`) is the merged one-shot client;
the control socket serves `pane/*` (`crates/holler-hub/src/control_server.rs:117` forwards to
`pane_dispatch`, which routes `"pane/list"` at `crates/holler-hub/src/panes/mod.rs:205` to the
handler whose `data` is `Vec<Pane>` sorted by name). `ControlCall`'s fields are `pub`
(`control.rs:66-70`), so the plugin constructs `{method: "pane/list", params: None, timeout}`
without touching the hub crate, decodes the reply as `PaneReply` (`crates/holler-pane/src/reply.rs`)
→ `Vec<Pane>` — zero new framing. Shelling to `holler pane list` is not merely worse, it is
impossible today: `Wiring::connect` returns the `Unwired` stub (`crates/holler-cli/src/pane/wiring.rs:31-33`,
body to be replaced by open story #649), so every pane verb currently answers `not-implemented`.
Depending on `holler-hub` from a plugin binary has exact precedent — `holler-cli` declares the same
dependency for the same reason (`crates/holler-cli/Cargo.toml:405`, used at
`crates/holler-cli/src/transport.rs:35`), and the dependency direction (leaf consumer → lib crate
inside the one workspace) is the established one. No new external dependency brings its own server,
ORM, template engine or state store; the workspace-boundary gates pass. The client is synchronous
(`std:: UnixStream`), so the plugin needs no runtime.

## Ruling 4 — "unknown, never stale": SOUND

The design's three legs check out. (1) **Active re-report**: the hub read and the Herdr socket are
independent, so a failed read (socket absent, `NoLiveHub`, timeout) can still actively write
`unknown` tokens under the same `source` — this is the mechanism that makes failure visible rather
than silently old, and it is what the acceptance test drives. (2) **`ttl_ms` backstop**: pane
reports accept `ttl_ms?` (spike §4), so a plugin that dies leaves expiring data, not permanent
lies; the residual staleness window (registry changed, no trigger yet) is bounded only by the ttl —
F should choose it consciously, and the survey's `pane/watch` long-poll is the documented
escalation if the event hooks prove thin. (3) **Triggers**: `pane.created` and `layout.updated` are
the schema-valid dotted subscription names (spike §10; §13 warns an unknown hook name is *only* a
warning, so the dotted forms must be kept exactly as the manifest has them). A Herdr restart
*deletes* metadata (absent, not stale), and because each invocation is a stateless read→report,
any post-restart trigger re-reports everything — satisfying the "refresh after restart" criterion.

## Ruling 5 — duplication inventory (plan level)

**Imported, not re-derived:** the `Pane` record types and `PaneReply` envelope (`holler-pane`), the
one SHOWN/DRIVEN rule `shown_differs` (`crates/holler-pane/src/reconcile.rs:179`), and the control
client (`holler_hub::control::run`). **Forced duplication (warns 1–2):** the Herdr report client
and the SYNC gate wrapper — both forced by the operator-set boundary (no `crates/**` edits), hence
deliberate and reviewed, not drift. **Trivial mirrors:** the hold words `none`/`parked`/`drained`
(`crates/holler-cli/src/pane/list.rs:262-268`) and the health words — string mappings over imported
types.

## Findings

| # | Severity | Plan element | Dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | plugin's Herdr socket client ("copy the shapes" from the spike) | duplication | `holler-adapter-herdr` already ships the hardened one-line-per-connection client (`Transport`/`UnixSocketTransport`, `crates/holler-adapter-herdr/src/transport.rs`), but its `Request` enum is closed over the hub-adapter methods (`protocol.rs:112-135` — no `report_metadata` variants), and `crates/**` is out of blast radius: the plugin must write its own minimal exchange. Justified new code — but it must mirror the adapter's transport *discipline*. | Mirror and cite the adapter's rules: one request per connection; a deadline bounding the whole exchange; a reply-size cap; error messages that never carry request or reply bytes. If a second display consumer ever appears, `report_metadata` moves into the adapter crate — note that in the crate's docs. |
| 2 | warn | SYNC token (`ok`/`mismatch`/`-`) | duplication | The comparison rule (`shown_differs`) is imported, but its *gating* — `last_observed.at > 0` and a present `session_of_record`, else unobserved — lives in `SessionSync::of` (`crates/holler-cli/src/pane/list.rs:230-240`), and the plugin cannot import it without depending on the whole CLI crate (lib+bin; clap/reqwest/holler-body/rustls). The gate must be restated. | Doc-cite `SessionSync::of` in the plugin's mirror and cover its four cases (never observed / no session of record / mismatch / ok) in tests, so a future change to the CLI gate is caught as a deliberate diff, not silent drift. |
| 3 | warn | acceptance criterion 1 (the mutating-name scan) | pattern consistency | The denylist is illustrative (`...`, family globs); display-changing names outside the listed families — `pane.input.set`, `pane.report_agent`, `notification.show`, `workspace.close` — could pass a literal reading. The brief's own "Allowed:" line already implies the stronger form. | Implement the scan as an **allowlist**: every Herdr method token found in the crate's sources and manifest must be one of `ping`, `session.snapshot`, `pane.get`, `pane.list`, `pane.read`, `events.subscribe`, `pane.report_metadata`, `workspace.report_metadata`; anything else fails closed. |
| 4 | warn | "Every report carries `ttl_ms`" (criterion 2) | abstraction fidelity | Spike §4 (`docs/research/herdr-api-spike.md:142`) lists `ttl_ms?` on `pane.report_metadata` but shows `workspace.report_metadata {workspace_id, source, tokens}` with no `ttl_ms`. Sending it there may error on the real server; a hand-rolled fake will not catch that. | Pane reports carry `ttl_ms`; the workspace token's freshness rests on active re-report (and restart loss). Do not send `ttl_ms` on workspace reports unless the schema is re-checked against a real binary. |

## Notes for O

- No BLOCK findings; no brief amendment required. The four warns are implementable inside the
  declared boundaries and are for T/F to carry (3 and 4 are test-design and wire-fidelity notes).
- The pipeline's `paths.test` glob is `crates/*/tests/**`; the plugin's tests at
  `plugins/herdr-holler/tests/**` fall outside it. The unit command still builds and runs them once
  the crate is a member (which is what the acceptance requires), and the config is out of blast
  radius this run — surfaced only so the path classification surprises no one.
- Manifest action commands must be direct argv arrays (B2), never `/bin/sh -c` — the spike probe's
  own manifest uses a shell string, and that is the one shape not to copy from it.

## Patterns referenced

1. `crates/holler-hub/src/control.rs` (`run`, `ControlCall`, `NoLiveHub`) + `control_server.rs:117` + `panes/handlers.rs` — the read seam.
2. `crates/holler-cli/src/transport.rs:33-38` and `crates/holler-cli/Cargo.toml:405` — the precedent for a binary depending on `holler-hub`.
3. `crates/holler-pane/src/pane.rs`, `reply.rs`, `reconcile.rs:179` — the record, the reply envelope, the one rule.
4. `crates/holler-cli/src/pane/list.rs:205-250` (`SessionSync`, hold words) — the cell semantics to mirror; `pane/wiring.rs:8-33` — the #649 stub.
5. `docs/research/herdr-api-spike.md` §3, §4, §10, §12, §13 — the plugin surface, limits, ttl, triggers, version facts.

VERDICT: PASS
