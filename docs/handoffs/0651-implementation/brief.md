# Brief: #651 — the Herdr display plugin (display only)

Repo: `Performant-Labs/holler`. Issue: #651 (epic #633, wave 3, optional). Rigor: **in-session**.
UI surface: no (D and U are N/A). Branch: `issue-0651-implementation` from `origin/main` at
`3d95aec`. Run worktree: `.claude/worktrees/0651-implementation` (repo-relative).

## Objective

A Herdr plugin — `herdr-plugin.toml` plus a small binary in a **new crate `plugins/herdr-holler`**
— that reads the hub's pane records (`pane/list` over the control socket, the #639 handlers,
through the merged one-shot client `holler_hub::control::run`) and shows, per pane, in Herdr's
sidebar: the **project**, **SHOWN against DRIVEN**, and the **hold** — as `pane.report_metadata` /
`workspace.report_metadata` tokens under one `source` id (`holler`), with `ttl_ms` so nothing
outlives its freshness. **Display only**: the plugin offers actions, and every action's command is
a `holler pane` verb (`switch`, `reset`, `doctor`, `park`, `unpark` — the merged set); the plugin
itself never changes a pane, a session or the layout. With the hub unreachable (or a read that
fails) it actively reports **`unknown`** — it never leaves stale data reading as current.

## Acceptance criteria (from the issue, as tests)

- [ ] **No call to the Herdr pane-changing API.** A test scans the plugin crate's sources and the
      manifest and fails if any mutating Herdr method name appears (denylist from the spike §3
      method list: `pane.split`, `pane.swap`, `pane.move`, `pane.zoom`, `pane.layout`,
      `pane.send_text`, `pane.send_keys`, `pane.send_input`, `pane.close`, `pane.rename`,
      `pane.resize`, `pane.scroll`, `pane.clear`, `layout.apply`, `layout.set_split_ratio`,
      `server.*` mutators, `integration.*`, `plugin.*` mutators, `tab.*` mutators, ...). Allowed:
      `pane.report_metadata`, `workspace.report_metadata`, and read-only methods (`ping`,
      `session.snapshot`, `pane.get`, `pane.list`, `pane.read`, `events.subscribe`).
- [ ] **Unknown, never stale.** With the hub unreachable (fake control socket absent/refusing),
      the reporter emits `unknown` state (state_labels / token values `unknown`) for the panes it
      had been showing, under the same `source` — a test drives one successful refresh, then a
      failed read, and asserts the second report set is `unknown`, not the first set repeated.
      Every report carries `ttl_ms`, so a plugin that stops reporting leaves expiring data.
- [ ] **Per-pane display facts.** One hub read (`pane/list`, `Vec<Pane>`) yields one
      `pane.report_metadata` per registry pane addressed by its `herdr.pane_id`: tokens for the
      position (`pos=r2c1` via `GridPos` Display), the project (`host.cwd`), SHOWN
      (`last_observed.shown`), DRIVEN (`last_observed.driven`), sync (`ok`/`mismatch`/`-` through
      `holler_pane::reconcile::shown_differs` — the one rule; never a second comparison), and the
      hold (`none`/`parked`/`drained`); profile name as a workspace token where the workspace
      maps. Token names obey `^[A-Za-z0-9_-]{1,32}$`; ≤16 tokens per report (SCHEMA limits).
- [ ] **Every action is a `holler pane` verb.** A test asserts each manifest action's command is
      an argv array whose program is `holler` and whose subcommand is a merged pane verb, and that
      the pane it acts on is resolved from the registry (the `HERDR_PANE_ID` the action receives →
      the record with that `herdr.pane_id`) — never by the plugin mutating anything itself.
- [ ] Re-report triggers wired: manifest event hooks `pane.created` and `layout.updated` re-run the
      refresh; a refresh after a Herdr restart re-reports (metadata is lost on restart — spike
      §12.3).
- [ ] `cargo fmt` clean, clippy clean, no new `unsafe`, full unit command green.

## Declared blast-radius exception (for A to confirm)

Root `Cargo.toml` `members = ["crates/*"]` gains `"plugins/*"` — **one line** — so the new crate's
tests run under `cargo test --workspace` (CI and the pipeline's unit command). Without it the
acceptance tests run nowhere. Stated here and in the PR body; everything else stays inside
`plugins/herdr-holler/**`.

## RED policy (test-first, honestly)

T authors the suite in `plugins/herdr-holler/tests/**` against the not-yet-existing crate. The
crate skeleton (manifest, `Cargo.toml`, `src/lib.rs`) may land with T **only as much as the tests
need to compile** — a RED that fails to build counts as RED, but T records which failures are
"does not exist yet". F then implements until GREEN. Fakes only: a fake control socket for hub
reads and a fake Herdr socket that captures report lines; temp dirs; **no real Herdr session, no
live hub, no fleet**. If any acceptance check is green on contact, T records it with evidence in
`handoff-T-red.md` and the run surfaces it to the operator instead of inventing work.

## Boundaries (operator-set, stricter than the defaults)

- **F:** `plugins/herdr-holler/**` plus the one declared `Cargo.toml` line above. **T:**
  `plugins/herdr-holler/tests/**` (plus the compile-minimal skeleton noted above).
- No edits to `crates/**` (the epic's shared hot spots are owned by sibling stories), no
  Aftersight/opencode config, no `docs/research/**` changes (the spike is closed).
- The plugin calls only the two `report_metadata` methods and read-only Herdr methods; every
  mutating name is denied by test.
- No mutation testing. One cargo at a time. Never touch a live fleet, a running pane or a real
  Herdr session; never print or commit a credential or secret (env var **names** only, per I7).
- Conventional Commits, `cargo fmt`, clippy clean, no new `unsafe`; handoffs in this directory
  carry neutral placeholders only (public-repo rule).

## Handoffs

`docs/handoffs/0651-implementation/` (in the run worktree): `survey.md` (written), `decisions.md`,
`handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
`handoff-S.md`. D and U are N/A (no UI surface) and are recorded as such, never silently skipped.

## Verification

Narrow: `cargo test -p herdr-holler` (crate name F lands; the display-only scan test runs here).
Full (pre-merge, as CI runs it): `cargo test --workspace -- --skip
roster_stays_accurate_under_concurrent_body_load`.

## A-gate warns (binding for T and F — from handoff-A.md, PASS with 0 blockers)

1. The plugin's own minimal Herdr report client is *forced* (the `holler-adapter-herdr` `Request`
   enum is closed over hub-adapter methods — no reuse within the blast radius); it must mirror the
   adapter's Unix-socket transport discipline (one JSON line in, one out).
2. The SYNC token's gate around the imported `shown_differs` must doc-cite
   `SessionSync::of` (`crates/holler-cli/src/pane/list.rs`) so the two cannot drift silently.
3. The mutating-name safety test is an **allowlist**: the crate's sources and manifest may name
   ONLY `pane.report_metadata`, `workspace.report_metadata` and read-only methods (`ping`,
   `session.snapshot`, `pane.get`, `pane.list`, `pane.read`, `events.subscribe`); any other
   Herdr method name fails the test. (Stronger than the brief's denylist sketch.)
4. `workspace.report_metadata` carries **no `ttl_ms`** (spike §4): send TTL on pane reports only.
