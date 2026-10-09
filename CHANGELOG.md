# Changelog

All notable changes to `holler` are documented here, per [Keep a
Changelog](https://keepachangelog.com/en/1.1.0/). See
[`docs/releasing.md`](docs/releasing.md) for the entry structure and the process that
fills this file in at release time.

## [Unreleased]

### Enhancements
- Pane control, slice a of the skeleton (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  the new `holler-pane` crate holds the contract that the pane-control stories build against in
  parallel. It has the `Pane`, `Profile` and `ProfileSpec` records, `GridPos` (rows and columns from 1),
  the `Argv` and `EnvVarName` guards (a stored command is an argv array; an environment entry is a
  name, never a value), the closed set of 22 kebab-case error codes with an open `Refused` variant,
  the ports an adapter implements (`PaneStore`, `ProfileStore`, `ProfileScope`, `HerdrPort`, `HostPort`,
  `HarnessPort`, `Prober`) and the `PaneReply` type of the hub's pane and profile control methods.
  Four empty crates (`holler-adapter-herdr`, `holler-adapter-host`, `holler-adapter-opencode`,
  `holler-pane-testkit`) are registered for the stories that fill them, and `holler-proto` gains the
  `pane/*` and `profile/*` method names, outside the closed 22-row wire catalog, so protocol v2 and
  every golden file are unchanged. Types and traits only: no CLI, hub or adapter behaviour yet
  ([#637](https://github.com/Performant-Labs/holler/issues/637)).
- Pane control, slice b of the skeleton (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  the hub's control socket now forwards the `pane/*` and `profile/*` methods to new `panes` and `profile`
  modules instead of answering `method_not_found`. Both are stubs: every one of those methods answers a
  `PaneReply` with the code `not-implemented`, as a JSON-RPC result and not a JSON-RPC error (the pane codes do
  not fit the closed wire code table). The hub builds the state of the two registries once, before it accepts a
  connection, and every control connection shares the one copy, so the stories that fill them add no
  forwarding, state-handle or manifest code of their own. A plain `check_membership` hook (it accepts every
  pane for now) and a `profile/rename` stub of its own are in place for them as well. Nothing else changes: an unknown
  method still answers `method_not_found`, the `control/*` methods behave as before, and the closed 22-row wire
  catalog and every golden file are unchanged. Plumbing only: no registry, persistence, CLI verb or adapter
  yet ([#669](https://github.com/Performant-Labs/holler/issues/669)).
- [ADR 0021](docs/adr/ADR-0021.md), the pane control plane (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  ratifies the pane and profile records, the ports, the verbs and invariants I1 to I8 as merged, and decides what the epic
  left open: where the two registries persist on the hub (`panes.json` and `profiles.json`, failing closed when corrupt), how
  generations fence concurrent writers, the write order of an edit made through `--profile`, the two error-code casings and
  how they are kept apart, the `--format=json` envelope and its exit codes, how attach mode relates, and the migration from
  the launcher's files. The operation id for long work is deferred to #644, and the points that need the operator are listed.
  Protocol v2 is unchanged; its "Not in v2" section now notes that the `pane/*` and `profile/*` methods are control-socket
  methods outside the wire catalog ([#634](https://github.com/Performant-Labs/holler/issues/634)).
- Pane control, the test kit's first part (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  `holler-pane-testkit` now has a fake pane registry that pane-control tests run against instead of a hub. A test can
  inject faults into it: a wedged store that times out, a corrupt one that fails every call, a one-shot error on one method,
  a slow call, and another writer's change. It also records every call made through it. A conformance suite that any pane
  registry runs against itself comes with it: 19 cases covering the compare-and-swap on a pane's generation, delete, list
  order, the watch stream (from the start, resumed from a cursor, idle) and the rule that a pane belongs to at most one
  profile. The fake passes all 19, and a registry that breaks one rule fails the case for it. The profile, Herdr, host and
  harness fakes and the JSON-envelope checker follow in later parts. Test code only: nothing a user runs changes
  ([#638](https://github.com/Performant-Labs/holler/issues/638)).
- Pane control, the test kit's host and harness fakes (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  `holler-pane-testkit` now has a fake host and a fake OpenCode harness. The fake host keeps tmux sessions, the argv of every
  command run in them exactly as given, and the processes each session owns. The fake harness is modelled on the OpenCode
  spike: servers that share one data directory, a frozen or killed server, a session deleted under a TUI, and switches for
  two behaviours of raw OpenCode, a select acknowledged with no TUI and an abort of an unknown session acknowledged. Both
  take the same injected faults as the fake pane registry and record every call made through them. Each comes with a
  conformance suite that an adapter runs against itself, 9 host cases and 15 harness cases: the fakes pass every case, a
  fake with a quirk switched on fails, and a host or harness that breaks one rule fails the case for it. Test code only:
  nothing a user runs changes ([#684](https://github.com/Performant-Labs/holler/issues/684)).
- Pane control, slice c of the skeleton (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  the CLI side. `holler pane` (`list get watch launch relaunch switch reset park unpark close doctor
  import`) and `holler profile` (`create delete list show apply rename export import`) are in the
  clap tree, one file and one set of arguments per verb, with the shared flags: `--profile`,
  `--spec-only`, `--take-over` and the spec flags (`--project --workspace --grid --model --effort
  --role --env --ctx-soft --ctx-hard --port-policy --command-arg/--command-json
  --check-arg/--check-json --expect`). Each verb refuses with `error: not implemented (story #NNN)`
  and exit 1 until its story lands. The global `--format text|json` is new: `--format=json` is
  `--json`, and `--json` with `--format=text` is a usage error. Under `pane` and `profile` JSON mode
  prints one envelope, `{"schema_version":1,"ok":false,"data":null,"error":{"code":...,"message":...}}`,
  and a usage error there is an envelope with code `usage` (exit 2); every other verb's output and
  usage errors are unchanged. `say`, `interrupt` and `answer` take a SESSION or `--pane NAME` (the
  existing forms parse and resolve as before), and with `--pane` or `--profile` they refuse with
  `not implemented (story #646)`, as does `roster --profile` (`#648`), in plain text before any hub
  is contacted. ADR 0003 has the new rows
  ([#670](https://github.com/Performant-Labs/holler/issues/670)).
- Pane control, the hub's pane registry (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  `pane/get`, `pane/list`, `pane/cas_put`, `pane/delete` and `pane/watch` now answer from a real registry
  instead of `not-implemented`. The hub keeps one record per pane in `<state dir>/hub/panes.json`, written
  atomically at mode `0600`, so a record survives a hub restart unchanged. Every write is a compare-and-swap
  on the record's generation: a stale one answers `generation-conflict` and changes nothing, and deleting a
  pane that has no record answers `pane-not-found`. `pane/watch` is a long-poll with a 4 s window that
  returns each change once, in order, with a cursor that never goes backwards across a restart. A watcher
  that resumes across a restart, or falls more than 1024 changes behind, gets the latest state of each pane
  that changed, deletions included. A corrupt or unreadable file fails closed: every pane method answers
  `store-corrupt`, the file is left in place and never rewritten, and an `error` event `pane_registry_corrupt`
  names the file and the problem without quoting its content. A file written by a newer build with a field
  this build does not know fails closed the same way. A write the hub cannot save answers `unavailable`,
  changes nothing, and logs the `error` event `pane_registry_write_failed`. Nothing else changes: no CLI
  verb or adapter yet, and the closed 22-row wire catalog and every golden file are unchanged
  ([#639](https://github.com/Performant-Labs/holler/issues/639)).
- `holler pane` and `holler profile` now exit 3 for a refusal and 1 for a runtime failure, the same in text
  and JSON mode, and the envelope's `ok` is false for both (as it is for a usage error, exit 2). A refusal is a
  request the system understood and declined, working as designed (`pane-in-other-profile`,
  `profile-secret-refused`, `command-not-argv`, `grid-ambiguous`, and any code a verb or adapter declares as
  its own); a failure is something that went wrong while doing the work (`timeout`, `generation-conflict`,
  `unavailable`). One function, `holler_pane::error::class_of`, decides which code is which, and the table of
  every code is in [ADR 0021](docs/adr/ADR-0021.md) section 9. The stubs still exit 1: `not-implemented` is a
  failure ([#676](https://github.com/Performant-Labs/holler/issues/676)).
- Pane control, the test kit's fake profile registry (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  `holler-pane-testkit` now has a fake profile registry that profile tests run against instead of a hub. A test can
  inject the same faults as into the fake pane registry: a wedged store that times out, a corrupt one that fails every
  call, a one-shot error on one method, a slow call, and another writer's change. It has a clock the test sets, and it
  keeps each profile's append-only change log (who wrote, when, and the generation after the write), which outlives a
  delete. A conformance suite that any profile registry runs against itself comes with it: 23 cases covering the
  compare-and-swap on a profile's generation, the refusal of a second name with a stored profile's slug, delete, the
  change log, `rename` (not implemented yet), environment entries that are names only, and the watch stream. The fake
  passes all 23, and a registry that breaks one rule fails the case for it. The fake profile scope follows in the next
  part. Test code only: nothing a user runs changes ([#682](https://github.com/Performant-Labs/holler/issues/682)).

- Pane control, the test kit's Herdr and probe fakes (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  `holler-pane-testkit` now has a fake Herdr that pane-control tests run against instead of a Herdr server. It places
  panes in the row-and-column cells of a workspace the test declares with a size, and gives each pane an id that is
  never reused and does not change when another pane closes. A test can switch it to a split-only mode, pick a
  supported or an unsupported Herdr version, wedge the server, make a pane's shell exit or a call slow, and read back
  everything typed into a pane. A fake health probe answers a scripted result per command and records every run. A
  conformance suite that the Herdr adapter runs against itself comes with them: 11 cases, which the fake passes and
  which reject an adapter that swaps rows and columns. [ADR 0021](docs/adr/ADR-0021.md) now records that
  `grid-out-of-range` also covers a cell outside its workspace. Test code only: nothing a user runs changes
  ([#683](https://github.com/Performant-Labs/holler/issues/683)).

- Pane control, the test kit's envelope checker (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
  `holler-pane-testkit` can now check the `--format=json` output of a `holler pane` or `holler profile` verb against
  its exit code, as one envelope or as an NDJSON stream (`pane watch`) in which only the last line may be a failure.
  It accepts exactly the envelope of [ADR 0021](docs/adr/ADR-0021.md) section 9: `schema_version` 1, `ok` true
  exactly at exit 0, `error` null exactly when `ok` is true, no other key, a kebab-case code with a one-line message,
  and an exit code that matches the class of the code. Otherwise it names the first rule the output breaks, such as
  a log line before the envelope, a `detail` key or a refusal at exit 1. It classifies codes with
  `holler_pane::error::class_of`, so it keeps no table of its own. Test code only: nothing a user runs changes
  ([#681](https://github.com/Performant-Labs/holler/issues/681)).

- Herdr adapter, part 1 (epic [#633](https://github.com/Performant-Labs/holler/issues/633)): the pure Herdr protocol
  and grid core in `holler-adapter-herdr`. It reads a pane's row and column from Herdr's split tree (Herdr has no
  grid, and this crate is the only place that converts between the two), plans the right and down splits that reach a
  cell (`grid-unreachable` when Herdr cannot reach it without nesting a split inside a cell, `grid-out-of-range`
  outside the workspace's configured size), builds Herdr's socket requests and decodes its replies, and gates Herdr's
  protocol version (22; any other is `herdr-version-unsupported`). No I/O yet: the socket adapter follows in part 2,
  so nothing a user runs changes ([#640](https://github.com/Performant-Labs/holler/issues/640)).

## [0.4.0] - 2026-09-29

### Enhancements
- `roster`, `say`, `interrupt`, `answer`, `wait`, `hub status` and `hub query` all take `--server URL`
  to run against a remote hub instead of the local one, over the existing hub WebSocket circuit
  (no new port, no new credential type). The remote form authenticates with this machine's own
  bound body credential and sends `circuit/hello role:"admin"` instead of `"body"`; the hub
  services the seven `admin/*` methods this adds without ever superseding the credential's live
  body connection or creating a roster row for the admin socket. `--json` output is byte-identical
  to the local form. A plaintext `ws://` to a non-loopback host is refused (exit 3), matching
  `body join`'s own policy ([#508](https://github.com/Performant-Labs/holler/issues/508),
  [#509](https://github.com/Performant-Labs/holler/issues/509), part of epic
  [#506](https://github.com/Performant-Labs/holler/issues/506); see [ADR 0020](docs/adr/ADR-0020.md)).
- CI: the Clippy step retries once from a clean build directory when, and only when, it fails with `could not parse/generate dep info` (a half-written file in the self-hosted runner's persistent build directory); any other failure, including every real lint, still fails on the first attempt ([#492](https://github.com/Performant-Labs/holler/issues/492)).
- Docs: the README has a Codex recipe, `docs/compatibility.md` records the Codex results (three authentication routes, permission requests, a multi-turn session with a hub restart) with their limits, and the monitoring guidance's Codex section describes how a permission request is surfaced and answered ([#474](https://github.com/Performant-Labs/holler/issues/474), [#303](https://github.com/Performant-Labs/holler/issues/303)).
- The body's ACP driver now also authenticates to an ACP v2 adapter that requires it. When a v2
  adapter refuses `session/new` as unauthenticated (JSON-RPC `-32000`) and the session's
  `auth_method` names a method the adapter advertised (not a terminal-type method, nor one of a
  type the body does not know), the body sends one `auth/login` (v2's `authenticate`) for it and
  retries `session/new` once, with the v1 path's guarantees and failure wording: only the method
  id is sent or logged, adapter text in a reason is capped and never includes the error's `data`,
  and a failure is fatal and never falls back to v1. A v2 startup failure with `auth_method` set
  says whether it was not applied, pending, or sent without the retry completing; without
  `auth_method` the v2 path and its errors are unchanged
  ([#459](https://github.com/Performant-Labs/holler/issues/459), follow-up to
  [#439](https://github.com/Performant-Labs/holler/issues/439)).
- Join held and a one-time release grant, on top of the session hold. `holler hub serve --join-held [GLOB]`
  (off by default) makes sessions join held: a default hold with the reason `held on join`, shown in
  `roster` as `held (default)`. `holler release SESSION --once [--ttl DURATION]` mints a grant for exactly
  one prompt and prints its id; `holler say --grant ID` gets that one prompt through, and the session is
  held again in the same step the hub accepts it (of two racing senders, exactly one gets through). An
  unused grant expires; a hub restart voids it while the default hold persists. A grant lifts only a
  default hold: an operator hold still refuses, and says so (`data.hold_kind`). Plain `release` peels the
  operator hold first, then the default hold. New wire error `-32012 invalid_grant` (`unknown`, `expired`,
  `used`, `other_session`), exit code 5 for `say --grant`. `hub query` and every other path are unchanged;
  with neither option used nothing changes ([#460](https://github.com/Performant-Labs/holler/issues/460),
  on [#437](https://github.com/Performant-Labs/holler/issues/437)).
- `holler hold SESSION [--reason TEXT]` and `holler release SESSION` (both idempotent, both take `--json`)
  set and lift the session hold; `holler roster` gains a `HOLD` column and `roster --json` rows carry
  `hold`, `hold_reason` and `held_since` while a session is held. `holler say` (and `interrupt SESSION
  TEXT`, whose redirect is a prompt) to a held session prints the reason and the since-time and exits **4**,
  distinct from a failed say (1); with `--json` the refusal is one JSON object on stdout. Documented in
  `docs/protocol/v2.md` §10 and the README ([#443](https://github.com/Performant-Labs/holler/issues/443),
  part of [#437](https://github.com/Performant-Labs/holler/issues/437)). `hold --all` and a hold line in
  `hub status` are not part of this change.
- `holler-load-test --scenario session-hold`: the session hold's cost under load — the say path with 0/10/50% of sessions held against a twice-measured baseline, refusal cost, `roster` scale, hold/release churn and a hub restart with 2,000 persisted holds. It hard-fails if a `say` is delivered to a held session or a hold is lost, and fails when a threshold written down from its first baseline is exceeded (`--report-only` to just print). Documented, with the measured baseline, in `docs/testing.md` ([#444](https://github.com/Performant-Labs/holler/issues/444), part of [#437](https://github.com/Performant-Labs/holler/issues/437) and [#366](https://github.com/Performant-Labs/holler/issues/366)).
- The hub enforces the session hold: `send_prompt`, the one place the hub sends a `session/prompt`, refuses
  a held session with `-32011 session_held` for plain `say`, `say --queue` and `say --replace`, while the
  running turn, turns already accepted into the body's queue, `interrupt` and `roster` are untouched. Holds
  are kept per session name in `hub/holds.json` (atomic, mode 0600), survive a body re-join, a roster prune
  and a hub restart, are refused for a session the hub has never seen, and fail safe on a corrupt or
  unwritable state file. The control socket gains `control/hold` and `control/release` and `control/roster`
  rows carry the hold fields; the CLI verbs follow in #443. `hub query TARGET` can no longer forward a
  method other than `query/*` to a body, which closes a way to deliver a prompt around a hold
  ([#442](https://github.com/Performant-Labs/holler/issues/442), part of
  [#437](https://github.com/Performant-Labs/holler/issues/437)).
- Wire vocabulary for the session hold (wire and docs half; no hub behavior yet): the new error code
  `-32011 session_held`, carrying `data.reason` (the operator's text, when given) and `data.since`
  (RFC 3339), and the additive optional roster fields `hold`, `hold_reason` and `held_since`
  (`holler_proto::SessionHold`). No protocol version bump; existing wire fixtures are unchanged.
  `docs/protocol/v2.md` §7, §8 and §10 describe them, and a new docs test pins every error code to a
  row of §8 ([#441](https://github.com/Performant-Labs/holler/issues/441), part of
  [#437](https://github.com/Performant-Labs/holler/issues/437)).
- The docs doc-test (`every_documented_holler_command_parses`) skips `docs/handoffs/`, the coding pipeline's per-story scratch, so a handoff that quotes a command fragment can no longer fail `cargo test --workspace` in a pipeline worktree; every other `docs/` directory is still scanned ([#462](https://github.com/Performant-Labs/holler/issues/462)).
- The body's ACP driver can now authenticate to an adapter that requires it. A spawn-mode session
  names the adapter's auth method with the new optional `auth_method` key; when a v1 adapter refuses
  `session/new` as unauthenticated (JSON-RPC `-32000`), the body sends one `authenticate` for that
  method and retries `session/new` once, or fails startup with a reason that names `auth_method`
  and the advertised method ids. The credential stays in the adapter's environment (the session's
  `env` or the body's own); only the method id is sent or logged. Attach mode ignores the key with
  a warning, and an empty value is a config error
  ([#439](https://github.com/Performant-Labs/holler/issues/439), for [#303](https://github.com/Performant-Labs/holler/issues/303)).
- The repository is wired to the Performant Labs coding pipeline: project role overlays in `docs/agent-overlays/` (the role docs themselves are generated per clone and gitignored), a Pipeline section in `CLAUDE.md`, and tracked git hooks (`.githooks/`, enabled with `scripts/setup-hooks.sh`) for a secret scan, a Conventional Commit subject check and the agent `Co-Authored-By` trailer.
- The hub now logs every rejected authentication (`auth_rejected`, with a stable `reason` such as `token_not_bound`, the token id and the failure count) and the lockout lifecycle (`lockout_tripped`, `lockout_cleared`) as `warn` events, visible at the default log level. Previously an expired token silently tripped a peer-wide lockout and only `lockout_refused` was ever logged ([#450](https://github.com/Performant-Labs/holler/issues/450), part of [#431](https://github.com/Performant-Labs/holler/issues/431)).
- `hub status` now shows the live lockout state: which peers are locked out or have failures building up, the time left on each cooldown, the failure reasons, and the token ids each peer named with their labels. `hub status --json` gains a top-level `lockout.peers` list (`{"peers": []}` when nothing is failing); every existing field is unchanged, and `hub status` without `--json` gets a short `lockout:` section only while there is a peer to show ([#451](https://github.com/Performant-Labs/holler/issues/451), part of [#431](https://github.com/Performant-Labs/holler/issues/431)).
- The body's session config now accepts an explicit `[ext.<namespace>]` table (top level) and
  `[session.ext.<namespace>]` table (per session) for other tools' own data, and the setup wizard's
  master-file keys (`hub_host`, `layout`, `[[orchestrator]]`, per-session `remote_host` and
  `remote_tailnet_host`); all are ignored by the body and only type-checked, so the master file can be
  passed to `holler body run --config` directly. Unknown keys elsewhere are still refused, and a
  non-table `ext` is refused ([#436](https://github.com/Performant-Labs/holler/issues/436)).
- Docs: `docs/compatibility.md` lists every agent and setup Holler has actually been run against, with links to the evidence, and marks what is untested.
- Docs, tests, fixtures and CI comments: replaced machine names, a tailnet name and account names with generic placeholders, and made the setup wizard read the OpenCode model from each host's own config instead of a hardcoded one.
- Dev scripts: `./scripts/run app:hub:run|launch|stop|status` and `app:body:run|stop` (no tunnel),
  ported from the retired `holler-server` repo ([#425](https://github.com/Performant-Labs/holler/issues/425)).
  `app:hub:stop` refuses to signal a pid that is not a `holler hub serve` process. Address and binary
  are overridable via `HOLLER_HUB_LISTEN`, `HOLLER_HUB_ADVERTISE`, `HOLLER_BIN`. `scripts/run` no longer
  uses the bash-4-only `;&` fallthrough.
- Research memo `docs/research/session-status-and-wait.md`, ported from the retired `holler-server`
  repo's never-merged `research/session-status-and-wait` branch (2026-09-08): why a wire push plus a
  blocking `wait` beats an A2A-style webhook for telling an orchestrator a session went idle, blocked,
  or failed, and why `idle` is not `done`. Indexed in `docs/research/README.md`. Not a decision record.
- A French translation of the README front door, [`README.fr.md`](README.fr.md) (Why Holler?,
  Install, Quick Start), following the existing zh-CN/ja/es/de pattern: AI-assisted, everything
  else stays English-only, and every language switcher now lists Français.
- The `setup-wizard` Claude Code skill now ships in the repo at
  [`agent-skills/setup-wizard/SKILL.md`](agent-skills/setup-wizard/SKILL.md) instead of living only
  in one operator's home directory. The README's agent prompt uses the skill if it is installed and
  otherwise fetches it from the repo, so it launches the real wizard on a machine that has never seen
  it (it previously fell back to an approximation rebuilt from `docs/setup-wizard.md`). The README
  also has a one-line install for Claude Code.
- Test harness: a started body's stdout and stderr are now kept in `<state>/body.log`, and
  `interrupt_test` warms each session up through the new shared `wait_warm`, which waits for a
  `connected` roster row and retries only `unknown session`. Any other warm-up failure, such as
  `reconnecting`, now fails at once with the roster, the hub log and the body log in the panic
  message, so the next CI failure of the flaky interrupt test carries its own evidence. The
  flake's cause is still open ([#420](https://github.com/Performant-Labs/holler/issues/420)).

### Bug Fixes
- `holler hub status`'s human-readable output no longer prints a hardcoded `(protocol 2)` — the protocol
  bump to 3 (#340/#352, ADR 0019's Noise XK hard re-pair) never updated this display string, so it lied
  about the running hub's actual protocol version. Now reads the real value from the status document, the
  same way `version`/`clients`/`sessions` already do — no longer goes stale on the next protocol bump.
- A token's label can be reused. `holler hub token delete ID` (and its aliases `rm`/`remove`) now removes
  an `unused` or `revoked` token's record from `hub/tokens.json`, which frees its label for a new
  `hub token mint`; it prints `deleted <id> (<label>, <prior state>)`, and `--json` reports
  `"state": "deleted"`. Before, `delete` did the same as `revoke` (it printed `invalidated` or `revoked`
  and kept the record), so nothing ever removed a record and a label, once minted, could never be minted
  again. `hub token revoke` is unchanged: it cuts a token off and keeps its record, which keeps holding the
  label. **Behaviour change:** `delete` on a `bound` token no longer revokes it: it exits 3, changes
  nothing, and names the command to run first (`holler hub token revoke <id>`). A mint over a held label
  still exits 3 with `already in use`, and the message now names the token that holds the label and the
  commands that free it. A body that was only detached still holds its token and label until the token is
  revoked and deleted, so the store growth under join/detach churn that #400 measured remains
  ([#454](https://github.com/Performant-Labs/holler/issues/454), refs
  [#400](https://github.com/Performant-Labs/holler/issues/400)).
- One body's bad token no longer locks out every body behind the same reverse proxy. The failed-authentication
  lockout was keyed by the peer's address alone, and behind a proxy every body arrives from the proxy's
  address, so one body's retries locked out all of them. The lockout is now keyed by the address and the
  token id the connection names: a token that fails 5 times in 10 minutes (the defaults) is refused from that
  address for the cooldown, still with close 1008 but now once its `circuit/authenticate` is read, while other
  tokens from the address keep connecting. A peer whose failures name 8 distinct token ids within the window
  is still locked out as a whole, before any frame is read (the [#184](https://github.com/Performant-Labs/holler/issues/184)
  flood guard); 8 is fixed. A successful authentication clears only that token's failures. No forwarded-address
  header is ever read. `hub status --json` `lockout.peers` now has one entry per token id (or one per peer
  locked out as a whole), each with a new `scope` (`token` or `peer`), sorted by peer, scope and id; the text
  view shows a whole-peer entry as `all tokens locked out`. The `lockout_tripped` log event gains `token_id` and
  `scope`, `lockout_cleared` names the token, and a token's refusal is logged `lockout_refused` with `peer`,
  `token_id` and `scope`. A failed authentication from an IPv6 peer is now counted; it never was before
  ([#455](https://github.com/Performant-Labs/holler/issues/455), part of
  [#431](https://github.com/Performant-Labs/holler/issues/431)).
- A second `holler hub serve` or `holler body run` started against a running one no longer blanks the
  running instance's lock file (`hub/serve.lock`, `body/run.lock`). The loser emptied the file before it
  found the lock held, so its refusal read `(pid )` instead of naming the running process, and the file
  lost that PID; `holler body detach`, which checks the body's lock to see whether a run is live, blanked it
  the same way. Only the instance that takes the lock now empties the file and writes its own PID, so the
  refusal names the holder and the file keeps its PID. The refusal text and exit code are unchanged
  ([#489](https://github.com/Performant-Labs/holler/issues/489)).
- `holler hub serve` with no `--listen` now listens on `127.0.0.1:41807`, the loopback default the protocol
  documents and the address `hub token mint`'s join line already pointed at. Before, the hub started and
  then panicked in its accept loop (`index out of bounds: the len is 0 but the index is 0`), leaving nothing
  listening. Any `--listen` replaces the default rather than adding to it, `hub serve --help` states the
  default, and the accept loop can no longer be built over an empty listener list
  ([#469](https://github.com/Performant-Labs/holler/issues/469)).
- A hub killed while saving its token store can no longer leave `hub/tokens.json` empty. The store was
  truncated and then rewritten, so a kill in between left a 0-byte file, and from then on every body's
  authentication failed with `tokens store is corrupted: EOF while parsing a value` until an operator
  repaired the file. It is now written to a temporary file in the same directory and renamed into place, so
  a kill leaves the old store or the new one. The hub's and the body's other state files get the same
  treatment: `holds.json`, `listening.json`, `advertise.json`, the body's `credential.json` and
  `connection_state.json`. The token pepper and the hub's and the body's X25519 identity keys are created
  once, atomically: a process that races another to create one gets the winner's key rather than a second
  key or a half-written file. `tokens.json` is now mode 0600, like the hub's other private state
  ([#483](https://github.com/Performant-Labs/holler/issues/483)).
- A hub that cannot complete an authentication for a fault of its own no longer counts it against the
  peer. When its token store could not be locked, read or parsed (such as the empty `tokens.json` of
  [#483](https://github.com/Performant-Labs/holler/issues/483)), or its identity key could not be resolved,
  the refusal was logged `auth_rejected ... reason=auth_failed` and counted toward the peer-IP lockout, so
  five of them locked out a body that never presented a wrong credential. Such a refusal is now logged as
  `auth_unavailable` at the new `ERROR` level with its cause, is not counted, and carries
  `error.data.reason: "hub_unavailable"` (still `-32002`, with the same `authentication failed:` message),
  so a body can tell it apart. One behaviour change: token-store lock contention that outlasts the store's
  2 s bounded retry is now reported this way and no longer counted. A wrong token or a failed handshake is
  refused, counted and logged as before ([#485](https://github.com/Performant-Labs/holler/issues/485)).
- `holler body run` keeps reconnecting when the hub refuses it for a fault of its own, and says why when it
  stops on a refused credential. A `-32002` carrying `error.data.reason: "hub_unavailable"` (the hub could
  not read its token store or resolve its identity key, [#485](https://github.com/Performant-Labs/holler/issues/485))
  stopped the body for good with exit 1, so it stayed down after the hub was repaired. The body now retries
  it like a dropped connection, with the same backoff: it reports `reconnecting`, logs `conn_dropped` naming
  `hub_unavailable`, and connects once the hub recovers. The hub never counts these refusals, so the
  retries cannot lock the body out. Every other `-32002` still stops the body with exit 1, since each one is
  counted and a retry would lock it out. The hub now sends each counted refusal's stable code in
  `error.data.reason` (the code its `auth_rejected` event logs, such as `token_not_bound`; a rejected Noise
  message 1 keeps `noise_message_one_rejected`), and `body run` and `body confirm` print it with the reason
  in plain words, for a revoked token "this hub no longer accepts this body's token: it was revoked, or never
  joined; mint a new token and re-run `body join`". The codes are shared constants in `holler_proto::noise`,
  listed in `docs/protocol/v2.md` §8. No new error code and no protocol version bump: an older body ignores
  the new reasons, and a body talking to an older hub stops as before
  ([#486](https://github.com/Performant-Labs/holler/issues/486), part of
  [#452](https://github.com/Performant-Labs/holler/issues/452)).
- `holler answer SESSION once|always|reject` now works for a spawn-mode (ACP) session held on a permission
  request, as `holler answer --help` already said; before, only OpenCode attach sessions accepted these
  words and an ACP session refused them with `does not resolve to any of this field's options`. `once`
  selects the adapter's option of kind `allow_once`, `always` the one of kind `allow_always`, and `reject`
  the one of kind `reject_once`, or `reject_always` when there is no `reject_once` option. A shorthand never
  selects an option of another kind: when the adapter offers none of the right kind, the answer is refused
  with the option labels and nothing is sent. An index, or an option whose key or label is literally the
  word, still takes precedence. `holler answer` can also select an option whose label contains a comma,
  such as Codex's `No, and tell Codex what to do differently`, by that label: a single-question prompt now
  tries the whole choice before splitting it on commas, where it used to fail with `expected 1
  comma-separated choice segment(s)`. Prompts with several questions still take one comma-separated
  segment per question ([#476](https://github.com/Performant-Labs/holler/issues/476),
  [#477](https://github.com/Performant-Labs/holler/issues/477)).
- Log timestamps now carry the real sub-second fraction, six digits of microseconds: 18.372988 s prints as `18.372988Z`. They used to print the microsecond-within-millisecond digits as a three-digit fraction (`18.988Z`), so lines from one process within the same second came out of order and hub and body logs could not be lined up ([#461](https://github.com/Performant-Labs/holler/issues/461)). Values taken from the same clock (`last_turn.ended_at`, the talk log's `ts`, and a working session's `turn_started_at`/`last_update_at`) now carry six fraction digits too; every reader in holler already accepted any number of digits.
- A joined body no longer loses access when its token's `expires` passes. The hub checked `expires` (`hub token mint --ttl`, 24h after the mint by default) on every re-authentication, so a long-running body was refused the first time it reconnected after that deadline, and its retries could lock out every client sharing its address. `expires` now bounds only how long the join secret can be redeemed; a bound token lasts until `hub token revoke` ends it. `hub token ping` no longer reports a bound token as `expired`, and `hub token list` prints `-` in EXPIRES for bound and revoked tokens (`--json` output is unchanged). The `token_expired` rejection reason is retired. Tokens already past `expires` work again after the upgrade, with no new join. Upgrade note: that includes any body you cut off only by letting its token expire. Before upgrading, check `holler hub token list --json` for `bound` rows whose `expires` is in the past, and `hub token revoke` any whose body must stay cut off; the text output shows `-` in EXPIRES for bound rows, so only `--json` shows those dates ([#453](https://github.com/Performant-Labs/holler/issues/453), part of [#431](https://github.com/Performant-Labs/holler/issues/431)).
- `hub token list` now shows a real `LAST_SEEN` for a bound token whose body is connected: the hub persists it on the presence heartbeat, at most once every 30 seconds per connection ([#419](https://github.com/Performant-Labs/holler/issues/419)). It was always empty before because nothing called `touch_last_seen` outside tests.
- `hub token mint`, `list` and `delete` now wait out a busy token store (the same bounded retry the hub's own paths use) instead of failing with "another holler process holds the token lock; retry". #373's churn run saw about 15% of mints fail that way ([#401](https://github.com/Performant-Labs/holler/issues/401)).
- The `v0.2.0` release's binary assets were withdrawn because they embedded the builder's home
  directory; the release page and its notes remain. Use `v0.3.0` or later (`install.sh` and the
  Homebrew formula already do). Documentation examples that pinned `HOLLER_VERSION=v0.2.0` now
  show `v0.3.0`.
- Release binaries no longer embed the builder's home directory. A plain `cargo build --release`
  bakes absolute source and registry paths (panic locations) into the binary, so the published
  macOS and Linux x86_64 `v0.3.0` assets carried the builder's OS username. New
  `scripts/release-build.sh` builds with path remapping and symbol stripping, and
  `scripts/check-binary-paths.sh` fails on any personal path; both are wired into the release
  procedure and checklist. The two `v0.3.0` assets were rebuilt with them and replaced.
- Documentation no longer links to the retired `holler-server` and `holler-client` repositories.
  The 53 links (ADR and protocol references, issue and pull-request references, and commit
  permalinks in the ported research memos) are now plain-text provenance such as
  `holler-server#319` or `holler-server ADR-0004`, and the memo headers say so, so nothing breaks
  when those repositories are deleted.
- Docs: the README said the Claude Code (`claude-agent-acp`) recipe was "currently blocked" on an
  ACP v2 handshake failure. That stopped being true when the v1-compatibility fallback landed
  ([#363](https://github.com/Performant-Labs/holler/pull/363)); a real `say` round trip against
  Claude Code passes ([#294](https://github.com/Performant-Labs/holler/issues/294)). The README now
  says so, and says which parts of that gate (`interrupt`, detach) have not been run.

### Known Issues

- The remote admin channel (`--server URL`) reuses the existing body join credential; there is no
  separate admin credential type yet ([#616](https://github.com/Performant-Labs/holler/issues/616)).
- No read-only/read-write scope split on the remote admin channel: every verb is available to any
  valid credential over it ([#617](https://github.com/Performant-Labs/holler/issues/617)).
- No dedicated rate-limiting or abuse posture on the remote admin channel — no cap on concurrent or
  queued admin requests per socket ([#618](https://github.com/Performant-Labs/holler/issues/618)).
- No new error codes for admin scope/credential-type failures, meaningless while the two items above
  stand as deferred ([#619](https://github.com/Performant-Labs/holler/issues/619)).
- The body does not tell the operator why the hub rejected it (expired, revoked, locked out) and
  keeps hammering the hub with the same doomed credential
  ([#452](https://github.com/Performant-Labs/holler/issues/452)).
- Flaky on Ubuntu CI: `interrupt_test::ack_timeout_message_when_body_stalls` sees `reconnecting`
  instead of the ack-timeout message ([#420](https://github.com/Performant-Labs/holler/issues/420)).
- Spawn-mode `interrupt` cancels the model turn but not a shell command the harness already started;
  the next `say` blocks until that command finishes
  ([#418](https://github.com/Performant-Labs/holler/issues/418)).
- Hub RSS grew 7.9 to 102 MiB over 578 churn cycles; leak versus bounded-but-large is unresolved
  ([#402](https://github.com/Performant-Labs/holler/issues/402)).
- A detached body's token label is never freed, and the credential store grows without bound under
  churn ([#400](https://github.com/Performant-Labs/holler/issues/400)).
- A body never detects that its attach-mode backend died: the roster keeps showing
  `connected`/`idle` because the body itself keeps heartbeating
  ([#397](https://github.com/Performant-Labs/holler/issues/397)).
- Windows is not a supported target: the control-socket transport is Unix domain sockets end to end
  ([#378](https://github.com/Performant-Labs/holler/issues/378)).
- `http_attach_driver`'s real permission/question wire shape has never been independently confirmed
  against a real OpenCode instance, only against hand-authored fake-server fixtures
  ([#365](https://github.com/Performant-Labs/holler/issues/365)).
- `session_manager_test::large_backlog_drains_in_strict_fifo_order_no_loss` failed once under load;
  not yet reproduced ([#491](https://github.com/Performant-Labs/holler/issues/491)).
- Flaky on macOS CI: `token_store_test::concurrent_operator_paths_never_lose_the_lock_race` failed
  once under CI-runner load, passed on an immediate rerun of the same commit; not yet reproduced
  locally ([#629](https://github.com/Performant-Labs/holler/issues/629)).

## [0.3.0] - 2026-09-23

### Enhancements
- Linux arm64 (aarch64) release binary: `holler-ubuntu-arm64`, published alongside the macOS and
  Linux x86_64 assets (originally attached to the [v0.2.0 release](https://github.com/Performant-Labs/holler/releases/tag/v0.2.0), since withdrawn),
  built and verified on GitHub's hosted `ubuntu-24.04-arm` runner via the new
  `.github/workflows/release-arm64.yml` (`workflow_dispatch`-triggered). `install.sh` and the
  [Homebrew tap](https://github.com/Performant-Labs/homebrew-tap)'s formula both detect and
  install it automatically on Linux arm64. See [`docs/releasing.md`](docs/releasing.md)'s
  platform table.
- Homebrew install: `brew tap Performant-Labs/tap && brew install holler`, via the new
  self-hosted [Performant-Labs/homebrew-tap](https://github.com/Performant-Labs/homebrew-tap)
  (fetches the real published release binary, real sha256-verified — not a source build). A
  `scripts/update-formula.sh` there bumps it for future releases; wired into
  [`docs/release-checklist-template.md`](docs/release-checklist-template.md) as its own step so
  it can't silently fall behind a release.
- `install.sh`: a one-line installer (`curl -fsSL .../install.sh | sh`) that downloads the
  right platform binary from the latest (or a pinned, `HOLLER_VERSION=`) GitHub release —
  no `cargo build` required to just use `holler`. Documented in the README.
- Load harness scenario 2 (`--scenario session-scale`): fixes N real `holler body run`
  processes and ramps M spawn-mode `stub-acp` sessions per body — 10 → 100 → 500, so the real
  total session count is 50 → 500 → 2500 at the default N=5 — and measures real
  `session/presence` propagation latency (a real `say`-driven state change timed until the
  hub's roster observes it — the same propagation path issue #359 fixed a race in, so this is
  also a standing regression guard for that fix), `holler roster --json` read latency as
  session count grows, and hard-fails (non-zero exit) if `hub status --json`'s `sessions`
  count ever disagrees with the real configured total. First measured baseline — fan-out
  p50 10.7ms → 40.8ms and roster-read p50 6.3ms → 34.7ms from 50 to 2500 sessions, both
  scaling with session count rather than body count — is documented in
  [`docs/testing.md`](docs/testing.md) ([#369](https://github.com/Performant-Labs/holler/issues/369), [#371](https://github.com/Performant-Labs/holler/issues/371)).
- Load harness scenario 3 (`--scenario sustained-throughput`): starts a real body fleet and
  drives a **sustained** `say --queue` rate across every session concurrently for a real,
  non-trivial duration (`--sustained-secs`, default 60s — not a burst), measuring real `say`
  round-trip latency (p50/p90/p99), whether `SessionManager`'s FIFO queue
  (`QUEUE_CAP = 64`, `crates/holler-body/src/session_manager.rs`) grows unbounded under
  sustained load or drains once load eases, and hub RSS sampled repeatedly across the run
  (not just before/after) so "flat after warmup" vs. "monotonic growth" is a real series. The
  queue-depth signal reuses `holler-body`'s existing `queue_enqueue`/`queue_dequeue`/
  `queue_full` debug events (issue #197) via a new `FleetMember::start_watched` that pipes and
  relays a watched body's stderr (`crates/holler-load-test/src/fleet.rs`) — zero new
  instrumentation landed in `holler-body` itself. First measured baseline (two independent
  60s runs, `--bodies 3 --sessions-per-body 1 --rate 20`): `say` latency p50 ~1.6–1.7s once a
  real backlog exists (vs. a ~173–176ms no-queue floor), queue depth peaked at 29 (well under
  the 64 cap, never refused) and drained to exactly 0 both runs after an 8s cooldown, and hub
  RSS rose 8.0→10.5 MiB during the drive then fell and flattened at 9.3–9.4 MiB afterward — no
  post-cooldown growth in either run. No Holler defect or harness bug surfaced. Documented in
  [`docs/testing.md`](docs/testing.md) ([#369](https://github.com/Performant-Labs/holler/issues/369), [#372](https://github.com/Performant-Labs/holler/issues/372)).
- Load harness scenario 4 (`--scenario churn`): repeated real join → run → detach cycles
  (`--churn-cycles`, default 20), hard-failing the run if `hub status --json`'s `clients` or
  `sessions` doesn't return to baseline after any cycle, plus a dead-backend probe that
  attaches a body session to a real fake-OpenCode process, `SIGKILL`s it, and times how long
  the roster takes to notice. First baseline: 20/20 cycles clean, the hub back to baseline
  11.6ms (p50) after the last body dies. The dead attach backend is **never detected**: the
  roster still shows `connected`/`idle` after 240s, past its own 180s `gone` threshold,
  because the body keeps heartbeating. Also adds `FleetMember::start_attach` and collapses
  `fleet.rs`'s duplicated join/spawn logic into shared helpers. Documented in
  [`docs/testing.md`](docs/testing.md) ([#373](https://github.com/Performant-Labs/holler/issues/373)).
- Load harness scenario 4, Run #2 additions: `--teardown-modes` (rotate `graceful`/`crash`/
  `hang`/`restart` per cycle, so a wave can die without detaching, go silent under SIGSTOP, or
  crash and rejoin on its saved credential), `--parallel-teardown`, `--resident` (a body kept up
  for the whole run, taking `say` traffic while each wave is torn down), `--label-reuse-probe`,
  `--churn-secs`, `--hang-budget-secs`, and a per-cycle series of hub RSS/threads/roster rows/
  credential records. 20-minute baseline (578 cycles, 10 bodies x 10 sessions, 57,800 `say`
  calls, zero failures): cleanup correct 578/578 in every mode (p50 6.7-19.8ms); rejoining on a
  saved credential kept the same `client_id` with no ghost rows across 144 cycles; a hung body
  is dropped at the roster's `reconnect` threshold (45.0s measured at production timers); a
  resident body saw 0 disturbances in 5,780 checks. Two findings: a gracefully detached body's
  label is never freed (the credential store grew to 5,781 records, unbounded, while roster rows
  stayed bounded), and `hub token mint` does not retry under token-store lock contention (864 of
  5,781 mints needed a retry) though the hub's own redeem path does. Documented in
  [`docs/testing.md`](docs/testing.md) ([#373](https://github.com/Performant-Labs/holler/issues/373)).

- README Quick Start: adds a real, verified `sessions.toml` (spawn-mode, pointed at `stub-acp`
  for anyone building from source) instead of only referencing config shape in the abstract, and
  points at [Harness recipes](README.md#harness-recipes) / [Attach convenience](README.md#attach-convenience)
  for the other two real paths (a real coding agent; attaching to an already-running session).
- Docs: the README is reworked (Why Holler?, Install, Quick Start, "The most practical way to run
  Holler", and translated front doors in zh-CN/ja/es/de), and [`docs/setup-wizard.md`](docs/setup-wizard.md)
  documents an agent-driven setup of one or more local orchestrator panes plus a live pane per
  remote Holler session in a [Herdr](https://herdr.dev) workspace: install Herdr, then (as a
  separate step) configure and launch it. Includes the operational rules learned the hard way:
  never kill a process on a shared host without identifying it first, label every token by its hub
  as well as its remote host, and that token labels are permanent.
- `scripts/seed-target-dir.sh`: gives a git worktree its own `target/` seeded copy-on-write from a
  warm one, so a build recompiles only this tree's crates instead of every dependency, without
  sharing a `CARGO_TARGET_DIR` across worktrees (which silently reuses stale binaries)
  ([#396](https://github.com/Performant-Labs/holler/pull/396)).

### Bug Fixes
- Fix (#404): `hub token mint`'s printed `body join` command always suggested `wss://`,
  even for a loopback `--advertise` (e.g. `127.0.0.1:41807`, exactly what the README's own
  Quick Start produces on a single machine). The hub only binds plain `ws://` on loopback, so
  the printed command hung indefinitely on a TLS handshake the hub never answers. Now uses
  `holler_body::server_address::ServerAddress::is_loopback` to pick `ws://` for a bare loopback
  advertise host and `wss://` for everything else, matching the body's own `--server` parsing.
- Fix: `holler interrupt` on an attach-mode session could leave the turn running. Against real
  OpenCode 1.18.32, `POST /api/session/<id>/interrupt` answered `204` but the session stayed `busy`
  in 1 of 2 controlled trials (and for minutes in the acceptance-gate run), while
  `POST /session/<id>/abort` stopped it every time. `HttpAttachDriver::cancel` now sends the abort
  after the interrupt (best effort). With the fix, interrupts of a genuinely running turn confirmed
  in under a second in every valid trial, and `say` returned `prompt was interrupted`
  ([#417](https://github.com/Performant-Labs/holler/pull/417)).
- Fix: every human-readable timestamp printed through `format_epoch` was **one day early** —
  the day-of-month term of its civil-date conversion was missing a `+ 1` (epoch 0 printed as
  `1970-01-00`). Affected `hub token mint`/`list`/`ping`'s `expires` and `last_seen`, and
  `body attach sessions`' `UPDATED` column; `--json` output was always correct (epoch seconds).
  Also corrects the helper's doc comment, which said local time when it formats UTC
  ([#416](https://github.com/Performant-Labs/holler/pull/416)).
- Fix: a panicking or early-returning test could orphan its `Body` child process; `Body` now has a
  `Drop` impl ([#391](https://github.com/Performant-Labs/holler/pull/391)).

### Known Issues
- A body's ACP child crashing mid-turn may not surface as an error promptly (a hang instead of
  `Done(Error)`): a confirmed defect in `agent-client-protocol` 2.1.0's crash detection, not fixable
  in this crate. Four `#[ignore]`d tests document it and still fail when forced
  (`cargo test --workspace -- --ignored`) ([#272](https://github.com/Performant-Labs/holler/issues/272),
  upstream [rust-sdk#250](https://github.com/agentclientprotocol/rust-sdk/issues/250) /
  [#254](https://github.com/agentclientprotocol/rust-sdk/issues/254)).
- Spawn-mode `interrupt` cancels the model turn but not a shell command the harness already started;
  the next `say` blocks until that command finishes (52 s observed with a 40 s loop)
  ([#418](https://github.com/Performant-Labs/holler/issues/418)).
- A body never detects that its attach-mode backend died: the roster keeps showing `connected`/`idle`
  because the body itself keeps heartbeating ([#397](https://github.com/Performant-Labs/holler/issues/397)).
- A detached body's token label is never freed, and the credential store grows without bound under
  churn (5,781 records over 578 cycles) ([#400](https://github.com/Performant-Labs/holler/issues/400)).
- `hub token mint` does not retry under token-store lock contention, though the hub's redeem path
  does ([#401](https://github.com/Performant-Labs/holler/issues/401)).
- Hub RSS grew 7.9 to 102 MiB over 578 churn cycles; leak versus bounded-but-large is unresolved
  ([#402](https://github.com/Performant-Labs/holler/issues/402)).
- `hub token list`'s `LAST_SEEN` is always empty: `touch_last_seen` is never called outside tests
  ([#419](https://github.com/Performant-Labs/holler/issues/419)).
- Flaky on Linux CI: `interrupt_test::ack_timeout_message_when_body_stalls`
  ([#420](https://github.com/Performant-Labs/holler/issues/420)).
- `http_attach_driver`'s real permission/question wire shape has never been independently confirmed
  against a real OpenCode instance, only against hand-authored fake-server fixtures
  ([#365](https://github.com/Performant-Labs/holler/issues/365)).
- Windows is not a supported target: the control-socket transport is Unix domain sockets end to end
  ([#378](https://github.com/Performant-Labs/holler/issues/378)).

## [0.2.0] - 2026-09-21

**Note:** this section had gone unpopulated since #327 (the release-process
infrastructure itself landed) despite many real merged PRs since — `scripts/changelog-check.sh`
only enforces that the `## [Unreleased]` heading exists, not that anyone actually adds an entry
per PR. The entries below cover this session's own work (2026-09-21), verified first-hand.
**Everything merged between #327 and the start of this session is still unaccounted for** —
that gap needs real git/PR archaeology to fill in accurately, not a guess; flagging it here so
it isn't lost, not silently closing it.

### Enhancements
- Pairing SAS confirmation gate (`holler body confirm`) — a one-time, operator-facing
  short-authentication-string check, separate from the fully-automated `run` ([#351](https://github.com/Performant-Labs/holler/issues/351)).
- ACP v1-compatibility fallback: `AcpDriver::spawn` now falls back to a plain v1 connection
  when a real harness only negotiates ACP protocol v1 (as of 2026-09-21, every real
  implementation checked does) — previously every real harness (`opencode`, the Claude Code
  ACP bridge) was completely unusable ([#363](https://github.com/Performant-Labs/holler/pull/363), root cause [#362](https://github.com/Performant-Labs/holler/issues/362)).
- Load/performance harness (`holler-load-test`): a dedicated workspace binary that starts a real
  hub, opens N concurrent real circuit connections (or a fleet of real `holler body run`
  processes driving `stub-acp` sessions), drives a controlled `say`/`interrupt`/`roster` rate,
  and reports per-call latency plus the hub's own RSS/thread/CPU usage as a human table and JSON.
  Scenario 1 (connection scale) and its first measured baseline — 200 of 200 concurrent
  connections, handshake p50 9-12ms / p99 500-518ms on a 10-core macOS box — are documented in
  [`docs/testing.md`](docs/testing.md) ([#369](https://github.com/Performant-Labs/holler/issues/369), [#370](https://github.com/Performant-Labs/holler/issues/370)).

### Bug Fixes
- `hub token mint`'s printed `join_command` omitted the `wss://`/`ws://` scheme entirely,
  producing a join line `body join` itself refuses (fail-closed on a schemeless `--server`)
  ([#356](https://github.com/Performant-Labs/holler/pull/356)).
- `server_address::parse` defaulted a schemeless `wss://host` (no explicit port) to the hub's
  own loopback port (41807) instead of the standard HTTPS port (443) that a real TLS-terminating
  proxy — e.g. `tailscale serve` — actually listens on ([#360](https://github.com/Performant-Labs/holler/pull/360)).
- A real race in `holler-body`'s session manager: `finish_turn` published a transient `Idle`
  presence notice the instant a turn ended, even when a `--queue`d prompt was already about to
  start — an operator could observe the roster go idle and fire a fresh `say`, only for the body
  to correctly refuse it moments later because the queued turn had actually started. Widens
  under real CPU/scheduling pressure, which is why it only reproduced on a loaded CI runner, not
  locally ([#359](https://github.com/Performant-Labs/holler/pull/359)).
- Concurrent authentication failed against itself: the hub read its token store under a
  non-retrying `flock` on both the `circuit/authenticate` and presence paths, so bodies
  connecting at the same time refused each other with `-32002 unauthenticated` — and, because a
  refusal also counts as a failed auth, cascaded into an IP lockout that force-closed further
  connections mid-handshake. Measured by the new load harness: 3 of 50 and 7 of 200 concurrent
  connections completed the handshake before the fix; 50 of 50 and 200 of 200 after. Extends
  [#301](https://github.com/Performant-Labs/holler/issues/301)'s fix to the two sibling call sites it missed ([#370](https://github.com/Performant-Labs/holler/issues/370)).

### Known Issues
- `http_attach_driver`'s real permission/question wire shape (the JSON payload OpenCode sends
  for a pending `session/request_permission`/question, and whether it appears on `/event` at
  all) has never been independently confirmed against a real OpenCode instance — only against
  hand-authored fake-server fixtures ([#365](https://github.com/Performant-Labs/holler/issues/365)).
- Attach-mode `interrupt` confirmation can take far longer than the CLI's ~2s client-side wait
  against a real, actively-streaming backend (observed ~21s against a real OpenCode session) —
  the underlying interrupt is correct, but `holler interrupt`'s own confirmation timeout is
  tuned for something faster than real streaming backends ([#318](https://github.com/Performant-Labs/holler/issues/318)).
- `holler roster` did not detect a real dead attach-mode backend (its process killed externally)
  for several minutes, continuing to report `connected`/`idle` — no fix landed yet, tracked as
  motivation for the load-testing churn scenario ([#373](https://github.com/Performant-Labs/holler/issues/373)).
- Windows is not a supported target — the control-socket transport is Unix domain sockets end
  to end ([#378](https://github.com/Performant-Labs/holler/issues/378)).
