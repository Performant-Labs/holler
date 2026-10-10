# Handoff-T-red: Phase 4 — the RED suite for #651 (Herdr display plugin)

**Date:** 2026-10-10
**Branch:** `issue-0651-implementation` (run worktree `<run-worktree>`, repo-relative
`.claude/worktrees/0651-implementation`)
**Brief / wireframe reviewed:** `docs/handoffs/0651-implementation/brief.md` (no wireframe —
UI surface is no, D/U N/A); survey.md; spike §3/§4/§12/§13.

## A precondition

Confirmed: A returned **PASS** on the plan (0 blockers, 4 warns), all four warns folded into
the brief as binding for T/F and honoured by this suite (allowlist not denylist — test 1;
no `ttl_ms` on workspace reports — test 2; the `SessionSync::of` doc-cite gate — test 1b;
the minimal Herdr client's transport discipline is F's to build against the fakes here).

## What was built (compile-minimal skeleton, per the brief's RED policy)

- `plugins/herdr-holler/` — new crate `herdr-holler` (lib `herdr_holler`), house member
  layout (`version.workspace`, `edition.workspace`, `license.workspace`, `[lints] workspace
  = true`; path deps `holler-pane` + `holler-hub`; dev-deps `serde_json`, `toml`,
  `tempfile`, `holler-pane-testkit`).
- **The one declared blast-radius exception:** root `Cargo.toml` `members` gains
  `"plugins/*"` (A-approved). `Cargo.lock` picked up the new package entry as a generated
  side effect (12 added lines, no other change) — staged with the rest so the commit builds.
- `src/lib.rs` — signatures only; every body is a refusing stub (`Err(PlugError::…)` /
  `None`), no `unimplemented!()`, no fake-passing logic.
- `herdr-plugin.toml` — the manifest the tests assert: 5 actions (switch/reset/doctor/
  park/unpark — argv `["holler","pane",<verb>]`), 2 event hooks (`pane.created`,
  `layout.updated` → `["herdr-holler","refresh"]`), `min_herdr_version = "0.9.1"`.

## Tests authored (all integration tests, in `plugins/herdr-holler/tests/`)

Cheapest sufficient tier for every criterion: the whole story is library + manifest
behaviour over local Unix sockets, so unit/integration level in-crate is right; no e2e (no
UI surface — D/U N/A). No two tests pin the same behavior.

| # | Test file :: test | Acceptance criterion | Tier | What it pins |
|---|---|---|---|---|
| 1 | `display_only_allowlist.rs` :: `sources_and_manifest_name_only_allowed_herdr_methods` | No call to the Herdr pane-changing API | integration (source scan) | **Allowlist** (A-warn 3): `src/**` + manifest may name only `pane.report_metadata`, `workspace.report_metadata`, `ping`, `session.snapshot`, `pane.get`, `pane.list`, `pane.read`, `events.subscribe`; any other dotted name under a Herdr namespace fails (unknown names too) |
| 1b | `display_only_allowlist.rs` :: `sync_comes_from_shown_differs_and_cites_the_cli_cells` | SYNC through the one rule | integration (source scan) | src must derive sync via `holler_pane`'s `shown_differs`, the caller must doc-cite `SessionSync::of`, and no src file may compare `shown` itself (no second comparison) |
| 2 | `report_contract.rs` :: `unknown_never_stale` | Unknown, never stale | integration (two fake sockets) | one good refresh, then the hub socket removed → the next refresh re-reports the same pane ids under source `holler` with `state_labels: ["unknown"]`, **every token re-sent as `unknown`**, `ttl_ms` present, no workspace report; across both waves `ttl_ms` on pane reports only (A-warn 4) |
| 3 | `report_contract.rs` :: `per_pane_display_facts` | Per-pane display facts | integration (two fake sockets) | 3 fixture panes → one `pane.report_metadata` per registry pane addressed by `herdr.pane_id` (tokens `pos`=`r2c1` GridPos Display, `project`=`host.cwd`, `shown`/`driven` (or `-`), `sync` ok/mismatch/`-`, `hold` none/parked/drained); token names obey `^[A-Za-z0-9_-]{1,32}$`, ≤16 per report; profile as a workspace token per `herdr.workspace`; hub wire sees `pane/list` only |
| 3b | `report_contract.rs` :: `endpoints_come_from_the_environment` | the injectable seams | integration (env) | `Endpoints::from_env()`: `HERDR_SOCKET_PATH` → Herdr socket, `HOLLER_STATE_DIR` → `<state>/hub/control.sock` |
| 4a | `manifest.rs` :: `every_action_is_a_holler_pane_verb` (+ `manifest_is_valid_and_every_command_is_argv`) | Every action is a `holler pane` verb; manifest validity | integration (manifest parse) | every action command is argv `["holler","pane",<merged verb>]`; id/name/version/description present, `min_herdr_version == "0.9.1"`, every command (actions and events) an argv array, never a shell string |
| 4b | `action_resolution.rs` :: `resolves_the_registry_pane_by_herdr_pane_id` / `an_unknown_herdr_pane_id_resolves_to_none` | action pane resolution | integration (pure fn) | `resolve_action_pane(panes, HERDR_PANE_ID)` → the record whose `herdr.pane_id` it is, by pane name; unknown id → `None` |
| 5 | `manifest.rs` :: `event_hooks_rerun_the_refresh_on_the_two_triggers` | re-report triggers | integration (manifest parse) | `[[events]]` wired on `pane.created` and `layout.updated`, each command argv running the plugin binary's `refresh` entry |

Fakes only, as the brief requires: a fake hub control socket (v2 envelope whose result is a
successful `PaneReply` with `Vec<Pane>` data, built from `holler-pane-testkit`
`sample_pane` fixtures) and a fake Herdr socket capturing every request line; temp dirs;
no real Herdr session, no live hub, no fleet. Env-touching tests serialize on one mutex and
restore prior values (`HOLLER_STATE_DIR`, `HERDR_SOCKET_PATH`,
`HOLLER_TEST_NO_CONTROL_SOCKET` forced off).

## RED confirmation

Narrow command: `cargo test -p herdr-holler --no-fail-fast` → **10 tests: 5 FAILED, 5
passed** (plus empty lib/doc targets). Full command (worktree root): `cargo test
--workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` →
**only the three herdr-holler test targets failed** (`action_resolution`,
`display_only_allowlist`, `report_contract`); every other crate green, the one configured
skip skipped. The rest of the workspace stays green.

Each failure fails for the RIGHT reason — the feature assertion, not setup:

- `resolves_the_registry_pane_by_herdr_pane_id` — `assertion left == right failed … left:
  None, right: Some("demo-beta")` (the resolver does not exist).
- `sync_comes_from_shown_differs_and_cites_the_cli_cells` — `the sync token must be
  derived through holler_pane's shown_differs (the one SHOWN/DRIVEN rule), somewhere under
  src/` (no derivation exists yet).
- `per_pane_display_facts` / `unknown_never_stale` — `called Result::unwrap() on an Err
  value: Herdr("the refresh loop is not implemented yet")`.
- `endpoints_come_from_the_environment` — `called Result::unwrap() on an Err value:
  Env("endpoints are not resolved yet")`.

## Green on contact (recorded, per the RED policy — not weakened)

Five tests pass before F exists, each because the artifact it checks is statically correct
on arrival, not because the check is hollow:

- `sources_and_manifest_name_only_allowed_herdr_methods` — the skeleton and manifest are
  clean by construction; the test is the standing guard over everything F adds.
- the three `manifest.rs` tests — the manifest is part of the T skeleton and already
  conforms; they guard it against F regressions.
- `an_unknown_herdr_pane_id_resolves_to_none` — the stub returns `None` everywhere, which
  is the correct answer for an unknown id; the boundary is pinned, the positive case
  (which fails) is the RED.

## The API surface the stubs define (for F)

```rust
pub struct Endpoints { pub herdr_socket: PathBuf, pub hub_control_socket: PathBuf }
impl Endpoints { pub fn from_env() -> Result<Self, PlugError> }   // HERDR_SOCKET_PATH, HOLLER_STATE_DIR
pub struct Reporter;                       // keeps the panes it last reported
impl Reporter { pub fn new(Endpoints) -> Self; pub fn refresh(&mut self) -> Result<Refreshed, PlugError> }
pub struct Refreshed { pub pane_reports: usize, pub workspace_reports: usize, pub unknown: bool }
pub enum PlugError { Env(String), Herdr(String) }
pub fn resolve_action_pane(panes: &[Pane], herdr_pane_id: &str) -> Option<PaneName>
```

- **Hub read:** build `ControlCall { method: "pane/list", params: None, timeout }` and go
  through `holler_hub::control::run` (precedent `crates/holler-cli/src/transport.rs`) —
  it resolves `<HOLLER_STATE_DIR>/hub/control.sock` from the ambient env, which is exactly
  the seam the fakes inject. Decode the returned result as `PaneReply`, its `data` as
  `Vec<Pane>` (`holler_pane::reply`). **Any** hub-read error is the degraded path.
- **Herdr reports:** the plugin's own minimal client (forced, A-warn 1): one JSON line in,
  one out, one request per connection, over `HERDR_SOCKET_PATH`; request shape
  `{"id","method","params"}`, success reply `{"id","result":{"type":"ok"}}`. `tokens` is
  pinned as a JSON object `name -> string` (the spike left the wire shape of tokens
  unverified; the tests define it now).
- **Token vocabulary:** pane reports carry exactly `pos`, `project`, `shown`, `driven`,
  `sync`, `hold` (absent shown/driven render `-`; sync is `ok`/`mismatch`/`-` with
  `SessionSync::of`'s cell semantics, derived through `shown_differs`; hold is
  `none`/`parked`/`drained`); workspace reports carry `profile`. Source id `holler`
  everywhere. Pane reports: `ttl_ms` > 0. Workspace reports: **no** `ttl_ms`.
- **Degraded contract (why tokens are re-sent):** Herdr's merge semantics per source are
  unverified, so dropping tokens might leave the old values live — re-send each token as
  `unknown` plus `state_labels: ["unknown"]`.
- **Binary:** add the `herdr-holler` bin whose `refresh` subcommand is the event-hook
  entry the manifest names.
- Housekeeping: `holler-hub` is a declared-but-unconsumed dep until F's code calls
  `control::run` (skeleton stubs don't); `cargo machete` passes once F lands.

## Ready for F

Confirmed: the RED is valid. F may implement against these tests (boundaries: everything
in `plugins/herdr-holler/**`; `crates/**` is read-only for F too).
