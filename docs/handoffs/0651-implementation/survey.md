# Survey: #651 — the Herdr display plugin (display only)

Run: `0651-implementation`, branch `issue-0651-implementation`, worktree `<run-worktree>`
(repo-relative: `.claude/worktrees/0651-implementation`), from `origin/main` at `3d95aec`.
Rigor: in-session. UI surface: no (D and U are N/A; the plugin renders through Herdr's own
sidebar, which is not a Holler UI).

## The story

A Herdr plugin — `herdr-plugin.toml` plus a small binary — that reads the hub's pane records and
shows, per pane, the project, SHOWN against DRIVEN and the hold in Herdr's sidebar. Display only:
no action it offers changes a pane, a session or the layout; every action calls a `holler pane`
verb. With the hub unreachable it shows "unknown", never stale data as current. A test asserts it
contains no call to the Herdr pane-changing API. Blast radius: `plugins/herdr-holler/**` (new).
Depends on #639 (closed, merged) and #636 (closed, merged) — both outputs are on main today.

## What the dependencies left us (facts, all on main)

### The Herdr plugin surface (#636, `docs/research/herdr-api-spike.md` §12 + §13)

- A plugin is a `herdr-plugin.toml` (manifest) with `id`, `name`, `version`, `min_herdr_version`,
  `description`, `[[actions]]` (`id`, `title`, `contexts`, `command` argv) and `[[events]]`
  (`on` = **dotted** subscription name like `pane.created`, `command` argv). `min_herdr_version`
  is required; the supported target is **Herdr 0.9.1, protocol 22, schema version 1**.
- A plugin process receives `HERDR_SOCKET_PATH`, `HERDR_BIN_PATH`, `HERDR_SESSION`,
  `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID` and the `HERDR_PLUGIN_*` set — and Herdr
  does **not** stop it calling mutating methods. "Display only" is therefore **Holler's rule,
  enforced by code and test** (the issue's acceptance): the plugin calls only
  `pane.report_metadata` / `workspace.report_metadata` and read-only methods; a test fails if its
  source names any other mutating method.
- What a report may carry: a pane `title`, a `display_agent`, `state_labels`, and up to **16
  `tokens` per report** (names `^[A-Za-z0-9_-]{1,32}$`, 32 kept per pane — SCHEMA limits). The
  sidebar renders tokens where the operator's `[ui.sidebar.agents] rows` / `[ui.sidebar.spaces]
  rows` name them as `$token`. The workspace method carries `tokens` only.
- Reports are **lost on a Herdr server restart** and `ttl_ms` can expire them; re-report on
  `layout.updated`, on `pane.created` and after a restart. Use **one `source` id** (e.g.
  `holler`) so the plugin's tokens can be cleared together.
- Agent rows appear only for panes Herdr considers to have an agent; a pane without one gets only
  the workspace row and the pane `title`. (Design consequence: per-pane facts must survive in
  `title`/workspace tokens too, or accept that some panes show less.)
- Wire: one JSON line in, one JSON line out, over the session socket; errors are
  `{code, message}`. The spike's example calls: `pane report-metadata <pane> --source holler
  --token pos=r1c1`, `workspace report-metadata <ws> --source holler --token profile=...`.

### The hub read path (#639, merged as 2a6f349)

- The hub serves `pane/get`, `pane/list`, `pane/cas_put`, `pane/delete`, `pane/watch` on the
  **control socket** (`holler_hub::control`); replies are a `PaneReply {ok, data, error}` JSON-RPC
  **result**, and `pane/list`'s `data` is every `Pane` record, sorted by name
  (`crates/holler-hub/src/panes/handlers.rs`).
- The merged one-shot client is `holler_hub::control::{run, ControlCall}` — exactly what
  `crates/holler-cli/src/transport.rs` uses for the hub-only verbs. The CLI pane verbs' live
  wiring is story #649 (open): `Wiring::connect` still hands out `Unwired`. **The plugin therefore
  talks to the control socket itself and does not go through `holler pane list`.**
- Display fields of `Pane` (`crates/holler-pane/src/pane.rs`): `host.cwd` (the project directory),
  `herdr.grid` (`GridPos`, Display `r2c1`), `herdr.pane_id` / `herdr.workspace` (Herdr's ids —
  the plugin's addressing keys), `last_observed.shown` (SHOWN), `last_observed.driven` (DRIVEN),
  `session_of_record`, `hold` (`none`/`parked{reason, release_when, since}`/`drained`),
  `profile`, `harness.health`. The one SHOWN/DRIVEN rule is
  `holler_pane::reconcile::shown_differs` (SHOWN vs `session_of_record`); SYNC is `ok`,
  `mismatch` or unobserved — `crates/holler-cli/src/pane/list.rs` builds exactly these cells.

### Actions (every action = a `holler pane` verb)

Merged verbs today: `list`, `get`, `watch` (#643), `launch`, `relaunch` (#644 p1), `switch`,
`reset` (#645 p1), `park`, `unpark` (#646 p1), `doctor` (#647 p1). `close` and routed `say` are
not merged — **not offered**. An action's process receives `HERDR_PANE_ID`; the plugin maps that
to the pane name through the registry record's `herdr.pane_id` and runs the verb (argv array,
never a shell string — B2's rule for every stored command).

## Reuse & Analogous-Feature map (extend vs new)

| Need | Closest existing thing | Recommendation |
|---|---|---|
| Control-socket client | `holler_hub::control::{run, ControlCall}` (used by `transport.rs`) | **Reuse** — depend on `holler-hub`; no new client framing |
| Pane record types / sync rule | `holler-pane::{Pane, PaneName, GridPos, Hold, reconcile::shown_differs}` | **Reuse** — decode `pane/list` data as `Vec<Pane>`; one SHOWN/DRIVEN rule, never a second |
| Sidebar row semantics | `crates/holler-cli/src/pane/list.rs` (`PaneRow`, SYNC cells) | **Mirror** the cell semantics (project, SHOWN, DRIVEN, SYNC, HOLD) — the plugin shows the same facts, derived through the same `shown_differs` |
| Manifest + report calls | `scripts/spikes/herdr-plugin.sh` (spike probe) | **Copy the shapes** (manifest keys, `--source`/token model → socket params from spike §4 table) |
| Tests: hub side | `holler-pane-testkit` (`pane_store.rs` fake, harness) — the fake registry the verb tests use | **Reuse** the fake `PaneStore` fixture panes; the plugin's own tests fake the two sockets (see RED policy) |
| Crate shape | `crates/*` workspace members with `[lints] workspace = true`, `version.workspace = true` | **New crate** at `plugins/herdr-holler` following the house crate layout |

**Extend-vs-new verdict: one new small crate that extends nothing and duplicates nothing** — every
shared fact (record types, sync rule, client) is imported from the existing crates; the only new
code is the plugin loop (read registry → diff → report), the manifest, and the actions.

## Tensions found (for A to rule on)

1. **Workspace membership.** Root `Cargo.toml` has `members = ["crates/*"]`; the blast radius is
   `plugins/herdr-holler/**` (new) and "edit only the files in the Blast radius". Without
   membership, `cargo test --workspace` (the pipeline's unit command and CI) never builds or runs
   the plugin's tests — the acceptance test would not run anywhere. Recommendation: **one declared
   line** — add `"plugins/*"` to `members` — stated in the brief and the PR body as the single
   blast-radius exception. Alternative (standalone crate with `[workspace]` opt-out) leaves the
   acceptance untested in CI: worse.
2. **"Never stale" needs a clock.** Reports expire (`ttl_ms`) and vanish on restart; "unknown,
   never stale data as current" therefore needs (a) an active re-report that sets `unknown` on
   read failure (so failure leaves visible `unknown`, not old values), and (b) a `ttl_ms` backstop
   so a plugin that dies leaves expiring data, not permanent lies. The refresh triggers are the
   manifest event hooks (`pane.created`, `layout.updated`) plus each action run (an action re-reads
   before acting anyway); a `--watch` long-poll loop (`pane/watch` on the control socket) is the
   natural continuous mode if the event hooks prove not to fire in testing.
3. **CLI verbs as actions vs display-only.** Actions shell out to the `holler` binary (found via
   `HERDR_BIN_PATH`-style discovery or `PATH`); the plugin itself never mutates. The test must
   also cover this: action argv arrays are `holler pane <verb>` calls, nothing else.

## Test kit / verification facts

- Unit command (pipeline config): `cargo test --workspace -- --skip
  roster_stays_accurate_under_concurrent_body_load`.
- The plugin's tests need **no real Herdr and no live hub**: both peers are fakes — a fake control
  socket (or an in-process `PaneState`-backed dispatcher) for reads, and a fake Herdr socket
  (captured JSON lines) for reports. Nothing touches a live fleet, a running pane or a real Herdr
  session; temp dirs only.
- No `unsafe`, `cargo fmt` clean, clippy clean (workspace lints deny `unwrap`/`expect`/`panic`).
