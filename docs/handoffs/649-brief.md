# [test(pane): hub wiring and the integration scenario with the real adapters against a scratch Herdr, tmux and OpenCode](https://github.com/Performant-Labs/holler/issues/649)

Part of epic #633.

(consolidated 2026-10-11, rewritten from merged `main` at 7284001 (#802): the operator's decisions D1 to D20 below are final and override every earlier wording of this issue, the amendment marks of the earlier text are kept in the Scope paragraph where they still bind. The Evidence section is verbatim from `main`; the pipeline copies this body into the brief, so cite an entry as E<n>.)

## Scope
Two things, in this order.

**1. The wiring.** Replace the stub body of `Wiring::connect` and the fields of `Wiring` in `crates/holler-cli/src/pane/wiring.rs` (E1, E2) with the real stores and adapters, so that `holler pane ...` and `holler profile ...` run against the hub's registries and the real Herdr, tmux and OpenCode. `Unwired` stays: it is the one not-implemented port set, and the in-process verb harness builds on it. The hub constructs no adapter (E5, E56); the verbs run CLI-side. The adapter crates `holler-adapter-herdr`, `holler-adapter-host` and `holler-adapter-opencode` become `holler-cli` dependencies (E52).

**2. The scripted scenario** (opt-in, scratch only) under `crates/holler-cli/tests/pane_integration/` (E50, E51): launch two panes, relaunch one, reset one with a first message, switch one, park one, kill a server and watch `doctor` find it, delete a session under a TUI and watch it be reported, restart the hub and find the registry intact. (amended 2026-10-08, profiles) Add a **profile scenario** on the same scratch instances: `holler profile create <scratch> --from-current`; edit it through `holler pane launch --profile`; `holler pane close --profile`; `holler profile show` and `list`; `holler profile delete` refused while its panes are live, then `delete --keep-panes` (the panes keep running with no profile). The apply step is not here: the real-adapter run of `holler profile apply` is #667. (amended 2026-10-08, grid) Launch one scratch pane with `--grid c1r2` and one with `--grid 2,1`, each into an otherwise empty scratch workspace, and assert from Herdr's own snapshot that each sits at row 2, col 1 as Herdr counts it (through #640). Every verb the scenario runs with `--format=json` (each `watch` line too) parses against the envelope with #638's helper (E55).

## Decisions (final)

**Wiring**

D1. **What `connect` builds.** A store client for the pane registry and one for the profile registry, both over the hub's control socket; a lazily connected Herdr adapter; a `TmuxHost`; an `OpenCodeHarness`; `SystemProber` (E21); and `StoreScope::new(profiles, panes, actor)` as the `ProfileScope` (E19). The actor is `holler pane` (a constant, as each profile verb names its own, E20). `Wiring::ports()` lends them as the seven ports of `Ports` (E4). `connect` may change signature freely (nothing outside `main.rs` and the verb harness calls it).

D2. **`connect` needs the hub and nothing else.** It makes one `control/status` exchange (`ControlCall::status()`, E6); with no hub running (`ControlError::NoLiveHub`, E8) it fails `unavailable` and the run exits 1 with that code, whose message names the missing hub (E3 reports it). `Io`, `BadReply` and `Refused` of that exchange are `unavailable` too. Herdr, tmux and OpenCode are not touched at `connect`: `TmuxHost::new` and `OpenCodeHarness::new` do no I/O (E27, E33), and the Herdr adapter pings Herdr when it is built (E23), so it is built on its first use and the result kept (`OnceLock`). A verb that never touches Herdr (`pane list`, `pane get`, `pane watch`, `profile list`, `profile show`) therefore works with no Herdr running.

D3. **The store clients** implement `PaneStore` and `ProfileStore` by `holler_hub::control::run` (E7) with the `ControlCall` of the method names (E9, each `&'static str`) and the params structs of E12 and E13; they parse the result with `PaneReply::into_result` (E10, E11) and decode `data` as E14 and E15 give it. Every call is bounded by 10 s (I5). A request that never reached the hub (`NoLiveHub`) is `unavailable`. **A write (`cas_put`, `delete`, `rename`) whose reply is lost after it was sent is `timeout`, not `unavailable`** (E40): the write may have landed, and the verb says so. `profile/rename` is PROPOSED (#665): the client sends it and passes the hub's `not-implemented` through.

D4. **Watch.** `PaneStore::watch` and `ProfileStore::watch` return a `Watch` (E17) that sends one `pane/watch` or `profile/watch` per `next()`, with `ControlCall.timeout` = `holler_hub::panes::WATCH_WAIT` plus 5 s (E57). A batch with events yields them in order; an empty batch yields `Ok(None)` (the port's idle item); the next request resumes from the reply's cursor (E16). A store or transport error yields one `Err` and ends the stream. A `since` ahead of the head is the hub's `usage`, passed through.

D5. **Herdr** is `HerdrAdapter::connect(HerdrConfig)` (E22, E23) from the environment of the process: the session is `HOLLER_HERDR_SESSION`, else `HERDR_SESSION` (Herdr sets it inside its panes, E25); the socket is `HOLLER_HERDR_SOCKET`, else `HERDR_SOCKET_PATH`, else `$XDG_CONFIG_HOME/herdr/sessions/<session>/herdr.sock` (`~/.config` when `XDG_CONFIG_HOME` is unset, E25); the workspaces are `HOLLER_HERDR_WORKSPACES=label=RxC[,label=RxC]...` (rows by columns, E24, E37, E38). No session configured: the Herdr port answers `usage` naming the two variables when it is first used (store-only verbs are unaffected, D2). `--herdr-session` stays required by `launch` as ADR-0021 records it (E37); the adapter refuses a name other than its configured session as `unavailable` (E24). Defaulting the flag from the wiring (ADR-0021 line 522) is **not** done here: it edits `launch.rs`, outside the Blast radius, and stays recorded as deferred.

D6. **tmux** is one `HOLLER_TMUX_SOCKET` (an absolute path becomes `TmuxSocket::Path`; unset is `TmuxSocket::Default`), used for **both** `TmuxHost` (E27) and `OpenCodeConfig.tmux` (E30), whose two `TmuxSocket` types the wiring maps from the one value. The binary is `tmux` from `PATH`.

D7. **OpenCode** is `OpenCodeConfig` (E30) with: `opencode_bin` = `HOLLER_OPENCODE_BIN`, else `opencode` resolved on `PATH` to an absolute path (E30 says wiring passes one); `serve_args` = `HOLLER_OPENCODE_SERVE_ARGS`, a JSON array of strings, default none; `env` = `ProcessEnv::Inherit` always (E29); the default `Timeouts`. No binary found: the harness answers `unavailable` naming `HOLLER_OPENCODE_BIN` when first used. **No `HostPort` amendment and no credential handling here**: a provider credential reaches the server through the operator's tmux server environment (E29, E39); how production sets it is the cutover's (#654), and nothing in this story puts a credential on an argv, in a file or in a log.

D8. **The `tui_session` resolver** (`Resolver<PaneId, PaneName>`, E31, E32) answers from, in this order: (a) the **bindings** of a launch in progress; (b) the pane registry (a record whose `herdr.pane_id` is the id); (c) the pane's label in Herdr's snapshot (E26: only a pane someone labelled, e.g. an imported one; `launch` never writes one); otherwise `pane-not-found` (E32). The bindings exist because the engine calls `attach_tui` with the Herdr pane id before the record exists (E34): a `HerdrPort` decorator remembers the `pane_id` that `ensure_pane` returned, and a `HostPort` decorator binds it to the name when `ensure_session(name, ..)` follows (the engine's order, E34). The bindings are per run and in memory. This couples the wiring to the engine's call order; a test pins it, and carrying the name in `attach_tui` is a later change to the port, not this story's.

D9. **Budgets.** The verbs keep `TxOptions::default()` (budget `DEFAULT_BUDGET`, 10 s, E35, E36); every adapter and the store client keep their own 10 s per-call bound. There is no new flag, no per-verb option and no environment variable for a budget. If a real `opencode` cannot start inside the budget in the scenario, that is a finding to report (the launch rolls back with `timeout`), not a reason to add a knob here.

**Scenario and tests**

D10. **Scratch-only, four guards.** (1) *Opt-in:* every scratch test is `#[ignore]` and runs only with `HOLLER_TEST_PANE_INTEGRATION=1` exactly (E43, E49); opted in, a missing `herdr`, `tmux` or `opencode` is a failure, not a skip (E48). (2) *Unique prefix:* the Herdr session is `holler649-<8 lower-case hex>`, every pane name is `hlr649-...`, and `default` is refused by name (E44). (3) *Temp state dir:* the hub, its registries, the tmux socket and Herdr's whole `HOME`/`XDG_*` live in one fresh `/tmp` directory, removed by a guard that kills only the processes it started (E42, E45, E49). (4) *Refusal of real names:* the test checks every name and path before it runs a verb and fails closed on a Herdr session that is not the scratch one, a tmux socket outside the scratch directory, a state dir outside it, or a pane name without the prefix; the binary is run with `HERDR_*`, `TMUX` and `TMUX_PANE` removed and `HOLLER_STATE_DIR` set (E53). Each guard is a pure function the default run tests.

D11. **Reuse, do not copy.** The scratch Herdr guard (`tests/scratch_herdr/mod.rs` of `holler-adapter-herdr`, E42 to E44), the OpenCode rig's dead-end provider and port picker (E45, E47) and the private tmux pattern (E49) are included from their crates with `#[path = "..."] mod ...;`, not copied. Anything that cannot be included as it is gets one small generalisation in place; a second copy of a guard is a defect.

D12. **No model call.** The only provider is the dead-end one of E47 (a closed local port): no model can be reached and no prompt reaches one. After every server is healthy the test checks it reports exactly that provider, or fails without going on. `reset --first TEXT` sends its message through `HarnessPort::send_prompt` (ADR-0021, #642 part 3); with the dead-end provider the server accepts it and no model is called. No step types into a TUI (I4).

D13. **Two kinds of test, one target.** Tests that need only a hub subprocess and a state dir (the `Hub` and `StateDir` harness, E53, E54) run in the default run (CI): the store clients' round trips, the no-hub failure (`unavailable`, exit 1, D2), a lost write reply (D3), a hub restart with the registry intact, and the watch test of D14. Tests that need Herdr, tmux and OpenCode are the scenario and are `#[ignore]` (D10). Both live in `tests/pane_integration/main.rs`'s modules, the one `[[test]]` that exists (E50); no manifest edit beyond the dependencies (D16).

D14. **`*/watch` is wired with one minimal test**: against a real hub, `holler pane watch --until-idle --format=json` from cursor 0 prints the put of a record written through the client, each line a valid envelope, and `ProfileStore::watch` of the client yields the put of a profile from cursor 0; nothing more.

D15. **The #802 follow-up belongs here.** One scratch test runs the server argv `OpenCodeHarness::serve_command` builds, through `TmuxHost::run` (E28) on the private tmux server (a real pty), waits for the health with the engine's `wait_healthy`, checks `verify_serving`, stops it with `stop_owned`, and asserts the port then refuses a connection and no server process of the test is left. The adapter's own rig cannot do it (it may not depend on the host adapter, E45).

D16. **Manifest.** Only `crates/holler-cli/Cargo.toml` changes: the three adapter crates as `[dependencies]` with a comment naming this story (E52). `holler-hub/src/pane_wiring.rs` is left empty and unedited (E56); `lib.rs` is #637's.

D17. **Docs.** ADR-0021 gains a "Wiring as built (#649)" paragraph in the section that names the adapters' construction (E5), recording D1 to D9 in prose; the CHANGELOG gets an `## [Unreleased]` entry linking this issue; the module doc of `wiring.rs` replaces the stub text (E1) and lists every environment variable of D5 to D7. `docs/pane-control.md` is #652's.

D18. **Not done here, and not decided here.** (a) *DRIVEN:* how the hub's DRIVEN session follows `session_of_record` stays deferred to #654 (E41); this story wires nothing for it, `last_observed.driven` stays as stored, and the scenario asserts SHOWN against `session_of_record` only. (b) The whole-fleet defaults of ADR-0021 line 522. (c) Moving the first message to the hub's prompt path (#646). (d) Importing the live fleet (#650) and the cutover (#654).

D19. **Order with #642 part 3.** The scenario's `reset --first` step and the real `send_prompt` and `session_activity` come from #642 part 3. This story is built on `main` at launch; if #642 part 3 has merged by the time F runs, the step runs as written, and if it has not, the PR waits for it (merge `main` in before the PR; do not stub the step out).

D20. **Rigor** is `second-opinion` (the operator's choice for the epic's remaining stories).

## Acceptance
- With no hub running, every `holler pane ...` and `holler profile ...` verb exits 1 with `unavailable`, naming the missing hub, and touches no adapter (a default-run test).
- `pane list`, `pane get`, `pane watch` and `profile list` work against a hub with no Herdr, no tmux and no OpenCode present (D2).
- A write whose reply is lost answers `timeout` (a default-run test with a hub that drops the reply, D3).
- The scenario passes on a developer machine with only scratch Herdr, tmux and OpenCode instances; nothing outside the scratch set is touched; the test refuses the real session names (D10) and the guards are tested in the default run.
- The scenario runs, in order: two launches; a relaunch of one; a reset of one with a first message; a switch of one; a park of one; a killed server found by `doctor`; a session deleted under a TUI reported; a hub restart after which `pane list` equals what it was.
- No step types into a TUI; at the end the registry equals the observed state (`doctor` reports no finding and `pane list` equals Herdr's snapshot for the scratch panes).
- The profile scenario passes (create from current, launch with `--profile`, close with `--profile`, show, list, delete refused then `--keep-panes`, the panes still running with no profile).
- The two grid launches sit at row 2, column 1 in Herdr's own snapshot (`c1r2` and `2,1`).
- Every `--format=json` output of the scenario parses against the envelope, each `watch` line too (D14).
- The `launch` of the scenario resolves `attach_tui` before the pane's record exists (D8), pinned by a test.
- The server argv runs under a real tmux pty (D15).
- No credential is printed, written to a file or put on an argv.

## Blast radius
- crates/holler-cli/src/pane/wiring.rs (the stub is replaced; `Unwired` stays)
- crates/holler-cli/src/main.rs (only if `connect`'s signature changes; the call at E3)
- crates/holler-cli/Cargo.toml (the adapter crates as dependencies)
- crates/holler-cli/tests/pane_integration/** (new files under the existing target)
- crates/holler-cli/tests/verb_harness/** and crates/holler-cli/tests/pane_verbs/** only for the one call of `Wiring::connect`, if its signature changes
- docs/adr/ADR-0021.md (D17), CHANGELOG.md
- crates/holler-hub/src/pane_wiring.rs: not edited (D16)

## Depends on
All merged except #642 part 3 (D19): #639 to #647, #660 to #663, #643, #637, #669, #670, #802, #808.

## Pipeline
`rigor: second-opinion` (D20).

## Rules for this story
See the epic (#633): the contract section there is fixed, so this story builds against it. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, `cargo fmt`, clippy clean, no new `unsafe`). Edit only the files in the Blast radius. No test touches a live fleet, a running pane or a real Herdr session; the scenario runs only on scratch instances it starts itself. Never print or commit a credential.

## Evidence (verbatim, from `main` at 7284001)

- **E1.** Today's `Wiring::connect` is a stub that builds nothing and cannot fail; `Unwired` is the one not-implemented port set and stays.
  **Source:** `crates/holler-cli/src/pane/wiring.rs:1-14`
  **Verbatim excerpt:**
  > //! The ports a `holler pane` or `holler profile` verb runs against (epic #633).
  > //!
  > //! [`Wiring`] owns them: [`Wiring::connect`] builds them, which can fail (no hub, no Herdr
  > //! socket), and [`Wiring::ports`] lends them as the [`Ports`] bundle a verb holds. `main.rs` calls
  > //! `connect` once per run and reports its error the way a verb reports one; `pane::run` and
  > //! `profile::run` never see it.
  > //!
  > //! **Stub (story #670).** `connect` hands out [`Unwired`], whose every method answers
  > //! `not-implemented`, so a verb that runs before its wiring exists fails loudly and never acts.
  > //! Story #649 replaces the body of `connect` (and the fields of `Wiring`) with the real stores and
  > //! adapters, and keeps `Unwired`: it is the one not-implemented port set, and the in-process verb
  > //! harness (`tests/verb_harness`) builds its `Ports` from it until the test kit's fakes (#638)
  > //! replace it there.
  >

- **E2.** `Wiring` holds one `Unwired` and lends it as every port of `Ports`.
  **Source:** `crates/holler-cli/src/pane/wiring.rs:24-47`
  **Verbatim excerpt:**
  > /// The ports a verb runs against, owned.
  > pub struct Wiring {
  >     ports: Unwired,
  > }
  >
  > impl Wiring {
  >     /// Build the ports of a run. Fails when something they need cannot be reached.
  >     pub fn connect() -> Result<Self, PaneError> {
  >         Ok(Self { ports: Unwired })
  >     }
  >
  >     /// The ports, lent for one verb.
  >     pub fn ports(&self) -> Ports<'_> {
  >         Ports {
  >             pane_store: &self.ports,
  >             profile_store: &self.ports,
  >             herdr: &self.ports,
  >             host: &self.ports,
  >             harness: &self.ports,
  >             scope: &self.ports,
  >             prober: &self.ports,
  >         }
  >     }
  > }

- **E3.** `main.rs` builds the wiring once per run and reports a `connect` failure the way a verb reports an error (exit 1 with the error's code).
  **Source:** `crates/holler-cli/src/main.rs:252-270`
  **Verbatim excerpt:**
  >
  > /// Run a `pane` or `profile` verb over the real wiring, printing to stdout and
  > /// stderr. A wiring that cannot be built is reported the way a verb reports an
  > /// error (exit 1, with the error's code).
  > fn run_with_stdio(format: Format, verb: impl FnOnce(&mut VerbCtx<'_>) -> i32) -> i32 {
  >     let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
  >     let mut sink = Sink {
  >         out: &mut out,
  >         err: &mut err,
  >     };
  >     match Wiring::connect() {
  >         Ok(wiring) => verb(&mut VerbCtx {
  >             format,
  >             ports: wiring.ports(),
  >             sink,
  >         }),
  >         Err(error) => output::emit_error(&mut sink, format, ErrorBody::from(&error)),
  >     }
  > }

- **E4.** `Ports` is seven `&dyn` ports and is `Copy`.
  **Source:** `crates/holler-pane/src/ports.rs:356-366`
  **Verbatim excerpt:**
  > /// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
  > #[derive(Clone, Copy)]
  > pub struct Ports<'a> {
  >     pub pane_store: &'a dyn PaneStore,
  >     pub profile_store: &'a dyn ProfileStore,
  >     pub herdr: &'a dyn HerdrPort,
  >     pub host: &'a dyn HostPort,
  >     pub harness: &'a dyn HarnessPort,
  >     pub scope: &'a dyn ProfileScope,
  >     pub prober: &'a dyn Prober,
  > }

- **E5.** The pane and profile registries are the hub's; the hub constructs no adapter (ruling 1).
  **Source:** `docs/adr/ADR-0021.md:292-295`
  **Verbatim excerpt:**
  > **Decided (ruling 1): verbs run CLI-side, against the ports; the hub is the store only.** The hub holds the pane registry
  > and the profile registry and answers their store methods; it constructs no adapter and runs no verb. The adapters are
  > constructed in `holler-cli/src/pane/wiring.rs` (#670 creates it, #649 fills it). The hub's `pane_wiring.rs` (#669) is an empty
  > placeholder with no job under this ruling.

- **E6.** A control call's method is a `&'static str`; `control::run` sends it to the hub named by the ambient `HOLLER_STATE_DIR`.
  **Source:** `crates/holler-hub/src/control.rs:66-70`
  **Verbatim excerpt:**
  > pub struct ControlCall {
  >     pub method: &'static str,
  >     pub params: Option<serde_json::Value>,
  >     pub timeout: std::time::Duration,
  > }

- **E7.** `control::run` is one synchronous exchange.
  **Source:** `crates/holler-hub/src/control.rs:431-443`
  **Verbatim excerpt:**
  > fn exchange_call(id_literal: &str, call: &ControlCall) -> Result<serde_json::Value, ControlError> {
  >     exchange_with_timeout(id_literal, call.method, call.params.clone(), call.timeout)
  > }
  >
  > /// Run any [`ControlCall`] over the local control socket — the generic
  > /// entry `holler-cli`'s own `transport` module calls for the **local** half
  > /// of MO 8's one dispatch point. Every verb-specific public fn above is a
  > /// thin wrapper around this same path; the control socket does not
  > /// correlate concurrent calls by id (see [`send_over`]'s own doc), so a
  > /// fixed literal here is exactly as good as each of their own per-verb ones.
  > pub fn run(call: &ControlCall) -> Result<serde_json::Value, ControlError> {
  >     exchange_call("b-transport", call)
  > }

- **E8.** A control exchange with no hub running fails `NoLiveHub` (the socket is missing); `Io` is a socket that was there and failed.
  **Source:** `crates/holler-hub/src/control.rs:18-26`
  **Verbatim excerpt:**
  > /// Why a control exchange could not reach the live hub.
  > #[derive(Debug)]
  > pub enum ControlError {
  >     /// No socket at the expected path (the hub is not running, or the state
  >     /// dir is wrong) — the spec's `no live holler hub reachable at <dir>`.
  >     NoLiveHub,
  >     /// The socket was present but the exchange failed (I/O or a bad reply).
  >     Io(std::io::Error),
  >     /// The reply did not parse as a v2 envelope.

- **E9.** The `pane/*` and `profile/*` method names.
  **Source:** `crates/holler-proto/src/methods.rs:113-133`
  **Verbatim excerpt:**
  > /// The hub's `pane/*` control-socket methods (the pane registry; issue #637). Outside
  > /// [`CATALOG`]: see the module docs.
  > pub const PANE_METHODS: &[&str] = &[
  >     "pane/get",
  >     "pane/list",
  >     "pane/cas_put",
  >     "pane/delete",
  >     "pane/watch",
  > ];
  >
  > /// The hub's `profile/*` control-socket methods (the profile registry; issue #637).
  > /// `profile/rename` is PROPOSED (#665). Outside [`CATALOG`]: see the module docs.
  > pub const PROFILE_METHODS: &[&str] = &[
  >     "profile/get",
  >     "profile/list",
  >     "profile/cas_put",
  >     "profile/delete",
  >     "profile/watch",
  >     "profile/log",
  >     "profile/rename",
  > ];

- **E10.** A reply is a JSON-RPC result `PaneReply`; `into_result` rebuilds the `PaneError` (a closed code to its variant, an open code to `Refused`, a malformed reply to `unavailable`).
  **Source:** `crates/holler-pane/src/reply.rs:50-57`
  **Verbatim excerpt:**
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > pub struct PaneReply {
  >     pub ok: bool,
  >     #[serde(default)]
  >     pub data: Option<Value>,
  >     #[serde(default)]
  >     pub error: Option<ReplyError>,
  > }

- **E11.** `into_result` and the parse-back rule.
  **Source:** `crates/holler-pane/src/reply.rs:85-95`
  **Verbatim excerpt:**
  >     /// The parse-back rule: a closed code maps to its own [`PaneError`] variant,
  >     /// any other well-formed code to [`PaneError::Refused`], and a code that is not
  >     /// well-formed to [`PaneError::Unavailable`]. A reply whose `ok` and `error`
  >     /// disagree is malformed and is also `unavailable`.
  >     ///
  >     /// A closed variant that carries a payload takes it from `detail`, or from the
  >     /// whole `message` when there is none; a closed variant without a payload (such
  >     /// as `not-implemented` or `generation-conflict`) is rebuilt from the code alone,
  >     /// and the peer's message text is not kept.
  >     pub fn into_result(self) -> Result<Option<Value>, PaneError> {
  >         match (self.ok, self.error) {

- **E12.** The params structs the client sends are in `holler_pane::reply`, shared by server and client; the `data` of the other methods is untyped there.
  **Source:** `crates/holler-pane/src/reply.rs:120-153`
  **Verbatim excerpt:**
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct PaneGetParams {
  >     pub name: PaneName,
  > }
  >
  > /// Params of `pane/cas_put`: the record, and the generation the caller read.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct PaneCasPutParams {
  >     pub pane: Pane,
  >     pub expected_generation: u64,
  > }
  >
  > /// Params of `pane/delete`.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct PaneDeleteParams {
  >     pub name: PaneName,
  >     pub expected_generation: u64,
  > }
  >
  > /// Params of `pane/watch` and `profile/watch`: resume after `since` (0, the default,
  > /// is from the beginning).
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct WatchParams {
  >     #[serde(default)]
  >     pub since: Cursor,
  > }
  >
  > /// The `data` of a `pane/watch` or `profile/watch` reply: the batch of events the
  > /// long-poll collected, and the cursor to resume from.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

- **E13.** `profile/*` params carry an `Actor`.
  **Source:** `crates/holler-pane/src/reply.rs:160-175`
  **Verbatim excerpt:**
  > /// Params of `profile/get`.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct ProfileGetParams {
  >     pub name: ProfileName,
  > }
  >
  > /// Params of `profile/cas_put`.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct ProfileCasPutParams {
  >     pub profile: Profile,
  >     pub expected_generation: u64,
  >     pub actor: Actor,
  > }
  >

- **E14.** The data of each `pane/*` method on the wire.
  **Source:** `crates/holler-hub/src/panes/handlers.rs:17-21`
  **Verbatim excerpt:**
  > //!
  > //! The `data` of each method (D7) is its closure's value: the `Pane` or `null` for
  > //! `pane/get`, an array of `Pane` sorted by name for `pane/list`, the stored `Pane` for
  > //! `pane/cas_put`, `null` for `pane/delete`, and a `WatchReply<PaneEvent>` for
  > //! `pane/watch`.

- **E15.** The data of each `profile/*` method on the wire.
  **Source:** `crates/holler-hub/src/profile/handlers.rs:5-10`
  **Verbatim excerpt:**
  > //! The `data` of each method (ADR-0021 §6) is its closure's value: the `Profile` or `null`
  > //! for `profile/get`, an array of `Profile` for `profile/list` (in slug order, which is not
  > //! pinned), the stored `Profile` for `profile/cas_put`, `null` for `profile/delete`, a
  > //! `WatchReply<ProfileEvent>` for `profile/watch`, and an array of `ProfileLogEntry`, oldest
  > //! first, for `profile/log`.
  > //!

- **E16.** The hub's `pane/watch` is one long-poll: the events owed now, or, after the window, an empty batch; the reply's cursor is the head.
  **Source:** `crates/holler-hub/src/panes/store.rs:252-266`
  **Verbatim excerpt:**
  >     /// One long-poll of the feed from `since` (D6): the events owed now, or, when there are
  >     /// none, the first ones written within the window. When the window ends first, the
  >     /// batch is empty. The reply's cursor is the head when the reply is made.
  >     ///
  >     /// The wait does not hold the lock: `Condvar::wait_timeout` releases it while the thread
  >     /// waits and takes it back before it returns. Every other method, and every other
  >     /// watcher, runs during the window. `a_waiting_watch_wakes_on_the_next_write`
  >     /// (`tests/pane_feed_test.rs`) writes while a watcher waits out a window far longer than
  >     /// the test.
  >     ///
  >     /// Rule 1 (a cursor ahead of the head is `usage`) is checked by `feed::select` on every
  >     /// pass. `pane/watch` passes the client's cursor straight here, so for that method this
  >     /// is the only check. A cursor that passed it once never fails it later, because the
  >     /// head only grows: each write takes `head + 1`.
  >     pub(crate) fn poll(&self, since: Cursor) -> Result<WatchReply<PaneEvent>, PaneError> {

- **E17.** `PaneStore::watch` returns a `Watch`, an iterator whose item `Ok(None)` is an idle batch; `holler pane watch` reads only `PaneStore::watch` and `ProfileScope::resolve`.
  **Source:** `crates/holler-pane/src/ports.rs:56-62`
  **Verbatim excerpt:**
  > ///     and the stream stays usable. A hub long-poll that sees it answers
  > ///     `{events: [], cursor}`;
  > ///   - `Err(..)`: a failure. `Err(PaneError::Timeout)` means the store did not
  > ///     answer within the bound (a wedged store), never "idle". Any error ends the
  > ///     stream (call `watch` again).
  > pub type Watch<T> = Box<dyn Iterator<Item = Result<Option<T>, PaneError>> + Send>;
  >

- **E19.** The real `ProfileScope` already exists CLI-side and takes the two stores and an actor; its module doc says #649 wires it in.
  **Source:** `crates/holler-cli/src/pane/profile_scope.rs:58-74`
  **Verbatim excerpt:**
  > /// The real [`ProfileScope`] over any [`ProfileStore`] and [`PaneStore`]; building it does no I/O.
  > pub struct StoreScope {
  >     profiles: Arc<dyn ProfileStore>,
  >     panes: Arc<dyn PaneStore>,
  >     /// Who every profile write of this scope is logged as.
  >     actor: Actor,
  > }
  >
  > impl StoreScope {
  >     /// A scope over `profiles` and `panes` that logs every profile write as `actor`.
  >     pub fn new(profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor) -> Self {
  >         Self {
  >             profiles,
  >             panes,
  >             actor,
  >         }
  >     }

- **E20.** Profile verbs name their actor with a per-verb constant.
  **Source:** `crates/holler-cli/src/profile/create.rs:36-40`
  **Verbatim excerpt:**
  > use crate::output::{emit, ErrorBody, ErrorCode, VerbCtx};
  >
  > /// Who the profile's change log says created it (#662 Decision 9).
  > const ACTOR: &str = "holler profile create";
  >

- **E21.** `SystemProber` is the real `Prober`.
  **Source:** `crates/holler-pane/src/ports.rs:345-354`
  **Verbatim excerpt:**
  > /// The [`Prober`] that runs the real probe: it calls the free [`run_probe`].
  > #[derive(Debug, Clone, Copy, Default)]
  > pub struct SystemProber;
  >
  > impl Prober for SystemProber {
  >     fn run_probe(&self, argv: &Argv, expect: &[String], timeout: Duration) -> ProbeResult {
  >         run_probe(argv, expect, timeout)
  >     }
  > }
  >

- **E22.** `HerdrConfig` is the session, its socket, a rows-by-columns extent per workspace label, and the call bound (10 s).
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:74-103`
  **Verbatim excerpt:**
  > pub struct HerdrConfig {
  >     /// The one Herdr session served.
  >     pub session: String,
  >     /// That session's socket, absolute.
  >     pub socket: PathBuf,
  >     /// Workspace label to rows by columns.
  >     pub workspaces: BTreeMap<String, Extent>,
  >     /// The bound of one `HerdrPort` call.
  >     pub timeout: Duration,
  > }
  >
  > impl HerdrConfig {
  >     /// A config with no workspace and [`DEFAULT_TIMEOUT`].
  >     pub fn new(session: impl Into<String>, socket: impl Into<PathBuf>) -> Self {
  >         Self {
  >             session: session.into(),
  >             socket: socket.into(),
  >             workspaces: BTreeMap::new(),
  >             timeout: DEFAULT_TIMEOUT,
  >         }
  >     }
  >
  >     /// This config, with the workspace `label` of `extent`.
  >     pub fn with_workspace(mut self, label: impl Into<String>, extent: Extent) -> Self {
  >         self.workspaces.insert(label.into(), extent);
  >         self
  >     }
  > }
  >
  > /// The `HerdrPort` over `T`.

- **E23.** `HerdrAdapter::connect` pings once; a Herdr protocol it does not know is `herdr-version-unsupported`, no reply in time is `timeout` naming `herdr.connect`.
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:111-131`
  **Verbatim excerpt:**
  >     /// Connect to the socket of `config`, as [`HerdrAdapter::connect_with`] does. This
  >     /// is the way production builds an adapter.
  >     pub fn connect(config: HerdrConfig) -> Result<Self, PaneError> {
  >         let transport = UnixSocketTransport::new(config.socket.clone());
  >         Self::connect_with(config, transport)
  >     }
  > }
  >
  > impl<T: Transport> HerdrAdapter<T> {
  >     /// Connect over `transport`. A config the adapter cannot serve is `usage`, before
  >     /// any request. Then one `ping`: a Herdr protocol the adapter does not know is
  >     /// `herdr-version-unsupported`, a `ping` that runs out of time is `timeout` naming
  >     /// `herdr.connect`, and either way no adapter is made. Over a transport other than
  >     /// the socket this is a test seam; production uses [`HerdrAdapter::connect`].
  >     pub fn connect_with(config: HerdrConfig, transport: T) -> Result<Self, PaneError> {
  >         validate(&config)?;
  >         let adapter = Self { config, transport };
  >         adapter.run_as(OP_CONNECT, |deadline| adapter.supported_server(deadline))?;
  >         Ok(adapter)
  >     }
  >

- **E24.** A workspace label with no configured extent is `unavailable`, and the adapter serves only its configured session.
  **Source:** `crates/holler-adapter-herdr/src/adapter.rs:178-200`
  **Verbatim excerpt:**
  >     /// served. Neither check sends a request.
  >     fn extent_of(&self, spec: &HerdrSpec) -> Result<Extent, PaneError> {
  >         if spec.session != self.config.session {
  >             return Err(PaneError::Unavailable {
  >                 what: format!(
  >                     "Herdr session {:?}: this adapter serves only the session {:?}",
  >                     spec.session, self.config.session
  >                 ),
  >             });
  >         }
  >         self.config
  >             .workspaces
  >             .get(&spec.workspace)
  >             .copied()
  >             .ok_or_else(|| PaneError::Unavailable {
  >                 what: format!(
  >                     "Herdr workspace {:?} of session {:?} is not configured, so its rows and \
  >                      columns are not known",
  >                     spec.workspace, spec.session
  >                 ),
  >             })
  >     }
  >

- **E25.** A named Herdr session's socket is `$XDG_CONFIG_HOME/herdr/sessions/<name>/herdr.sock`; inside a Herdr pane `HERDR_SOCKET_PATH` names it.
  **Source:** `docs/research/herdr-api-spike.md:106-108`
  **Verbatim excerpt:**
  > - A Unix stream socket, owner-only (`srw-------`). Default session: `$XDG_CONFIG_HOME/herdr/herdr.sock` (on Linux
  >   `~/.config/herdr/herdr.sock`); a named session: `$XDG_CONFIG_HOME/herdr/sessions/<name>/herdr.sock`.
  >   `herdr session list` prints each session's socket path. Inside a Herdr pane, `HERDR_SOCKET_PATH` names it.

- **E26.** Launch and relaunch never write a Herdr label, so a launched pane's `herdr.label` is `None`; the label is an observation (#808).
  **Source:** `crates/holler-pane/src/pane.rs:93-108`
  **Verbatim excerpt:**
  > /// what `HerdrPort::snapshot` lists; a pane Herdr reports also carries its label.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct HerdrPane {
  >     pub session: String,
  >     pub workspace: String,
  >     pub pane_id: PaneId,
  >     pub grid: GridPos,
  >     /// The pane's label as Herdr reports it, read by the Herdr adapter through its
  >     /// `label_of`: an observation (I6), and the join key from a pane name to a live Herdr
  >     /// pane (#808). `launch` and `relaunch` never write one, so a record's `herdr.label`
  >     /// is `None` and its JSON has no `label` key. `None` stands both for a pane Herdr
  >     /// sent no label for and for one it sent an empty label for.
  >     #[serde(default, skip_serializing_if = "Option::is_none")]
  >     pub label: Option<String>,
  > }

- **E27.** `TmuxSocket` and `TmuxHost::new`: a host on a socket with `tmux` from `PATH`, a 10 s bound, and no I/O.
  **Source:** `crates/holler-adapter-host/src/lib.rs:150-188`
  **Verbatim excerpt:**
  > /// Which tmux server the adapter talks to.
  > #[derive(Debug, Clone, PartialEq, Eq)]
  > pub enum TmuxSocket {
  >     /// tmux's own default socket (no `-L` or `-S`). `$TMUX` is never used to find it.
  >     Default,
  >     /// A named socket in tmux's socket directory (`-L <name>`).
  >     Name(String),
  >     /// A socket at a path (`-S <path>`).
  >     Path(PathBuf),
  > }
  >
  > /// The real `HostPort`: a local tmux server, driven by argument vectors, never a shell.
  > /// See the crate docs for the rules each method keeps.
  > ///
  > /// Building one does no I/O; every method blocks, as the port says. It is plain data, so
  > /// it is `Send + Sync`.
  > #[derive(Debug, Clone)]
  > pub struct TmuxHost {
  >     socket: TmuxSocket,
  >     tmux: PathBuf,
  >     kill: PathBuf,
  >     config: Option<PathBuf>,
  >     timeout: Duration,
  >     stop_grace: Duration,
  > }
  >
  > impl TmuxHost {
  >     /// A host on `socket`, with `tmux` and `kill` from `PATH`, no config file, a 10 s
  >     /// bound and a 2 s stop grace. Does no I/O.
  >     pub fn new(socket: TmuxSocket) -> Self {
  >         Self {
  >             socket,
  >             tmux: PathBuf::from("tmux"),
  >             kill: PathBuf::from("kill"),
  >             config: None,
  >             timeout: DEFAULT_TIMEOUT,
  >             stop_grace: DEFAULT_STOP_GRACE,
  >         }
  >     }

- **E28.** `TmuxHost::run` starts the server's argv in the pane's tmux session, in `cwd`, and tags the window.
  **Source:** `crates/holler-adapter-host/src/lib.rs:321-337`
  **Verbatim excerpt:**
  >             return match self.run_tmux(&tmux::new_session(name, cwd), call)? {
  >                 Reply::Done(_) | Reply::Refused(Refusal::Duplicate, _) => Ok(()),
  >                 Reply::Refused(_, what) => Err(PaneError::Unavailable { what }),
  >             };
  >         }
  >         match self.run_tmux(&tmux::has_session(name), call)? {
  >             Reply::Done(_) => Ok(()),
  >             Reply::Refused(refusal, _) if refusal.is_missing() => Err(PaneError::Usage {
  >                 message: format!(
  >                     "cannot create the tmux session {name}: its directory must be an \
  >                      absolute path to an existing directory"
  >                 ),
  >             }),
  >             Reply::Refused(_, what) => Err(PaneError::Unavailable { what }),
  >         }
  >     }
  >

- **E29.** `ProcessEnv::Inherit`: under tmux the server inherits the tmux server's environment, so a provider credential reaches it only through the pane's tmux session, never an argv.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:151-164`
  **Verbatim excerpt:**
  > /// The environment the adapter's OpenCode processes, the server and the TUI, start with.
  > /// Both are started by tmux (the server by the host adapter's `run`, #802), and both argvs
  > /// begin with the one `env` prefix it gives (`ProcessEnv::prefix`).
  > #[derive(Debug, Clone, PartialEq, Eq)]
  > pub enum ProcessEnv {
  >     /// Each inherits the tmux server's environment (tmux starts it, so not the adapter's),
  >     /// with `OPENCODE_DISABLE_TERMINAL_TITLE` removed (`env -u`), because the adapter reads
  >     /// the TUI's title. A provider credential reaches the server only that way, through the
  >     /// pane's tmux session (#649), never through an argv.
  >     Inherit,
  >     /// Each gets exactly these variables (`env -i` semantics). Tests use it for the scratch
  >     /// `HOME` and `XDG_*`. It must not set `OPENCODE_DISABLE_TERMINAL_TITLE`, which turns
  >     /// off the title the adapter reads.
  >     ///

- **E30.** `OpenCodeConfig`: the binary (wiring passes an absolute path), serve args, env, the tmux config, the `tui_session` resolver and the bounds.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:211-232`
  **Verbatim excerpt:**
  > }
  >
  > /// How the adapter reaches OpenCode and tmux.
  > #[derive(Clone)]
  > pub struct OpenCodeConfig {
  >     /// The `opencode` binary, for the server and the TUI. Its path must be valid UTF-8,
  >     /// since it goes on their command lines. tmux starts both, so a bare name is looked up
  >     /// on the `PATH` of the tmux server; wiring (#649) passes an absolute path.
  >     pub opencode_bin: PathBuf,
  >     /// Appended after `serve --port P --hostname 127.0.0.1`; the tests pass `["--pure"]`.
  >     pub serve_args: Vec<String>,
  >     /// The environment of the server and of the TUI's `attach` (see [`ProcessEnv`]).
  >     pub env: ProcessEnv,
  >     /// The tmux server the panes' TUIs run on.
  >     pub tmux: TmuxConfig,
  >     /// The tmux session name a pane's TUI runs in, never a tmux target (#649:
  >     /// `Pane.host.tmux`, which equals `Pane.name`, ADR-0021 line 37). The adapter alone
  >     /// turns it into a target.
  >     pub tui_session: Resolver<PaneId, PaneName>,
  >     /// The bounds.
  >     pub timeouts: Timeouts,
  > }

- **E31.** `Resolver` and its precondition: it must answer for a pane being launched, before its record exists, and is wiring's job (#649).
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:142-149`
  **Verbatim excerpt:**
  >
  > /// Maps a key to what the adapter needs; an `Err` is returned to the caller as it is.
  > ///
  > /// Precondition (ADR-0021 section 2, "`HarnessPort` as built (#642)", fact 6): a resolver
  > /// must answer for a pane that is being launched, before its registry record exists (#644
  > /// writes the record after the act). Meeting it is wiring's job (#649), not a registry
  > /// lookup's.
  > pub type Resolver<K, V> = Arc<dyn Fn(&K) -> Result<V, PaneError> + Send + Sync>;

- **E32.** The adapter's own words on the lookup: it cannot be backed by a registry lookup alone; an unknown `PaneId` is the lookup's own error `pane-not-found`.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:80-85`
  **Verbatim excerpt:**
  > //! - **The lookup is wiring's job** (fact 6, see [`Resolver`]). `tui_session` must answer
  > //!   for a pane that is being launched, before the pane's record exists, so #649 cannot back
  > //!   it with a registry lookup alone. The server's directory is no lookup (#802): it comes
  > //!   from the pane spec, through the host's `run`.
  > //! - **One divergence from the test kit's `FakeHarness`** (fact 6), which the conformance
  > //!   suite pins neither way: a `PaneId` the `tui_session` lookup does not know is the

- **E33.** `OpenCodeHarness::new(config)` starts nothing until a method is called.
  **Source:** `crates/holler-adapter-opencode/src/lib.rs:245-254`
  **Verbatim excerpt:**
  > /// The `HarnessPort` over OpenCode.
  > pub struct OpenCodeHarness {
  >     config: OpenCodeConfig,
  > }
  >
  > impl OpenCodeHarness {
  >     /// An adapter with `config`. It starts nothing until a method is called.
  >     pub fn new(config: OpenCodeConfig) -> Self {
  >         Self { config }
  >     }

- **E34.** The launch engine's call order: `ensure_pane` (Herdr, a `HerdrSpec` with no name), then `ensure_session` and `run` (host, which take the pane's name), then `server::start`, then `attach_tui(&herdr.pane_id, ..)`. The pane id is known at `ensure_pane`, the name at `ensure_session`.
  **Source:** `crates/holler-pane/src/tx_launch.rs:650-668`
  **Verbatim excerpt:**
  >     let herdr = ports.herdr.ensure_pane(&plan.target)?;
  >     if !seen.listed.iter().any(|p| same_pane(p, &herdr)) {
  >         made.pane = Some(herdr.pane_id.clone());
  >     }
  >     budget.check()?;
  >     let cwd = &plan.spec.host.cwd;
  >     ports.host.ensure_session(plan.name, cwd)?;
  >     made.session = true;
  >     if let Some(command) = &plan.spec.command {
  >         budget.check()?;
  >         ports.host.run(plan.name, cwd, command)?;
  >     }
  >     server::start(ports, plan, seen, budget, boot_poll)?;
  >     budget.check()?;
  >     let session = session_of_record(ports, plan)?;
  >     budget.check()?;
  >     ports
  >         .harness
  >         .attach_tui(&herdr.pane_id, plan.port, &session)?;

- **E35.** Every pane verb runs its transaction with `TxOptions::default()`, whose budget is `DEFAULT_BUDGET` (10 s).
  **Source:** `crates/holler-pane/src/tx_launch.rs:64-70`
  **Verbatim excerpt:**
  > /// (never adopted). A refusal, exit 3.
  > pub const PORT_IN_USE: RefusalCode = RefusalCode::from_static("port-in-use");
  > /// The budget of one whole verb run (I5's default bound), checked before every step.
  > pub const DEFAULT_BUDGET: Duration = Duration::from_secs(10);
  > /// The timeout handed to `Prober::run_probe`.
  > pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
  > /// How often the act asks a server it started whether it is healthy (#802).

- **E36.** `TxOptions` and its defaults.
  **Source:** `crates/holler-pane/src/tx_launch.rs:104-127`
  **Verbatim excerpt:**
  > /// constant.
  > #[derive(Debug, Clone, Copy)]
  > pub struct TxOptions {
  >     /// The budget of the whole run ([`DEFAULT_BUDGET`]).
  >     pub budget: Duration,
  >     /// The timeout of the health probe ([`PROBE_TIMEOUT`]).
  >     pub probe_timeout: Duration,
  >     /// How often the health wait asks the started server ([`BOOT_POLL`]); the wait ends with
  >     /// the budget.
  >     pub boot_poll: Duration,
  >     /// The wall clock stamped into `last_observed.at` (`holler_proto::clock::now_millis`).
  >     pub now_ms: fn() -> i64,
  > }
  >
  > impl Default for TxOptions {
  >     fn default() -> Self {
  >         Self {
  >             budget: DEFAULT_BUDGET,
  >             probe_timeout: PROBE_TIMEOUT,
  >             boot_poll: BOOT_POLL,
  >             now_ms: holler_proto::clock::now_millis,
  >         }
  >     }
  > }

- **E37.** ADR-0021: the frozen `HerdrPort` cannot tell the engine its session; `--herdr-session` is required and the wiring defaults it from the adapter's one configured session. A workspace's extent is wiring's.
  **Source:** `docs/adr/ADR-0021.md:519-523`
  **Verbatim excerpt:**
  > pane with no record, never a record naming the wrong pane; the run then exits 1 with `unavailable` and P keeps its edit.
  > Every failure once the act has started ends with the reconcile step of step 6, appended only when the message lacks it. A
  > run is bounded as section 12 records. `--herdr-session` is required because the frozen `HerdrPort` cannot tell the
  > engine its session; the wiring (#649) defaults it from the Herdr adapter's one configured session. What a failure or a
  > crash leaves is not all reported by the merged doctor: only a whole-fleet pass reports a Herdr pane no record names

- **E38.** ADR-0021: the extent per workspace label is the Herdr adapter's configuration, supplied by the wiring.
  **Source:** `docs/adr/ADR-0021.md:696-699`
  **Verbatim excerpt:**
  >   a cell outside its workspace's rows and columns (`r1c2` in a workspace of 2 rows by 1 column), which the `HerdrPort`
  >   conformance suite pins (#638 amendment 2026-10-08, grid; #683). Herdr has no grid, so a workspace's extent is the Herdr
  >   adapter's configuration: rows by columns per workspace label (`HerdrConfig.workspaces`, #640), which the wiring supplies
  >   (#649). A cell inside the extent that no single right or down split reaches is the open code `grid-unreachable` (#640).

- **E39.** ADR-0021: a provider credential reaches the server only through the pane's tmux session; that is #649's to set.
  **Source:** `docs/adr/ADR-0021.md:178-183`
  **Verbatim excerpt:**
  > sessions of their own (`scripts/spikes/opencode-lib.sh`), which the deleted `kill_group` did not stop either. A provider
  > credential never goes on an argv (`ps` and tmux show it): under tmux the server inherits the tmux server's environment, not
  > the adapter's, so a credential reaches it only through the pane's tmux session. That is #649's to set, and `HostPort`
  > has no environment parameter, so #649 may need a `HostPort` amendment of its own. tmux also limits a command's size
  > (tmux 3.7c refused a 20,000-byte `new-window` argument and took 15,000), so #642 part 3's `OPENCODE_CONFIG_CONTENT`
  > carries only `{"default_agent": KEY}`, never an agent's definition.

- **E40.** ADR-0021: the store client answers `timeout`, not `unavailable`, when it loses the reply to a write it sent.
  **Source:** `docs/adr/ADR-0021.md:417-423`
  **Verbatim excerpt:**
  > 2. **Profile CAS first.** `ProfileStore::cas_put(P_edited, g, actor)`. A conflict here comes before anything live changed:
  >    the verb exits 1 with `generation-conflict` and can be run again. A first write whose outcome is unknown (a `timeout`)
  >    may have landed, so the act does not run, and the answer is `timeout`, saying that P may hold the edit, with the
  >    reconcile step. The store client (#649) answers `timeout`, not `unavailable`, when it loses the reply to a write it
  >    sent. Any other error of this write is answered as it is (#663).
  > 3. **Act** (skipped with `--spec-only`), then observe.
  > 4. **Record.** Write the pane record with its own compare-and-swap (`cas_put`, or `delete` for `close`).

- **E41.** ADR-0021: how the hub's DRIVEN session follows `session_of_record` is deferred to #649 and #654.
  **Source:** `docs/adr/ADR-0021.md:733-738`
  **Verbatim excerpt:**
  > **Deferred to #649 (wiring) and #654 (cutover):** how the hub's DRIVEN session is pointed at `session_of_record` for each
  > registered pane, and when it moves after a switch or reset.
  > Until #649 wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the hub
  > drives, and leaves `last_observed.driven` as stored.
  > A switch or reset (#645) likewise leaves `last_observed.driven` as stored; the hub's DRIVEN session follows the new
  > `session_of_record` once #649 wires it.

- **E42.** The opt-in, scratch-only test pattern for Herdr: a gate on exactly `1`, a unique name prefix, `default` refused by name, and a proof before anything connects.
  **Source:** `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs:1-14`
  **Verbatim excerpt:**
  > //! A scratch Herdr server for the opt-in tests (#640 part 3): the first Rust code that
  > //! starts a real Herdr. It ports the spike's isolation (`scripts/spikes/herdr-lib.sh`)
  > //! and depends on nothing else of this crate's tests, so a later story can move it.
  > //!
  > //! Three guards keep the operator's Herdr out of reach, each a pure function that the
  > //! default run tests:
  > //!
  > //! - **The environment** ([`scratch_env`]): no `HERDR_*`, `TMUX` or `TMUX_PANE`, and
  > //!   `HOME` and every `XDG_*` directory inside a fresh root, so even Herdr's default
  > //!   session resolves to a socket inside the root.
  > //! - **The name** ([`check_name`]): every call carries `--session holler640-<8 hex>`,
  > //!   and `default` is refused by name.
  > //! - **The proof** ([`prove`]): the server must report that session and a socket inside
  > //!   the root before anything connects to it.

- **E43.** The Herdr scratch gate and name check.
  **Source:** `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs:33-36`
  **Verbatim excerpt:**
  >
  > /// The variable that opts in; only the exact value "1" runs a scratch server.
  > pub const GATE_VAR: &str = "HOLLER_HERDR_SCRATCH";
  > /// Every scratch session's name is this prefix and 8 lower-case hex digits.

- **E44.** The scratch name check.
  **Source:** `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs:90-106`
  **Verbatim excerpt:**
  >             "refusing the Herdr session {name:?}: it is Herdr's default session, never a \
  >              scratch one"
  >         ));
  >     }
  >     let hex = name.strip_prefix(NAME_PREFIX).unwrap_or("");
  >     let is_hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
  >     if name.starts_with(NAME_PREFIX) && hex.len() == 8 && hex.chars().all(is_hex) {
  >         return Ok(());
  >     }
  >     Err(format!(
  >         "refusing the Herdr session {name:?}: a scratch session is {NAME_PREFIX} and 8 \
  >          lower-case hex digits"
  >     ))
  > }
  >
  > /// A fresh name that passes `check_name` (std only, for example `RandomState`).
  > pub fn new_name() -> String {

- **E45.** The OpenCode rig: scratch `HOME`, the dead-end provider (no model call), ports 48100-48199, a private tmux server, a process-group guard.
  **Source:** `crates/holler-adapter-opencode/tests/real_opencode/rig.rs:1-20`
  **Verbatim excerpt:**
  > //! The rig of `real_opencode_test.rs` (#642b, the brief's "Real OpenCode" rules): one per
  > //! test, and one per conformance case. It is the spike's shell rig
  > //! (`scripts/spikes/opencode-lib.sh`) in Rust:
  > //!
  > //! - a fresh scratch dir in `/tmp`, short enough for a tmux socket path; `HOME` and the
  > //!   `XDG_*` dirs inside it, so no real OpenCode config, auth or session store is read;
  > //! - the dead-end provider config, verbatim from the spike, so no model can be reached:
  > //!   127.0.0.1:9 must refuse, and after every server the rig launches is healthy it must
  > //!   report exactly that provider, or the test fails without going on. No prompt is ever
  > //!   sent;
  > //! - a launcher (#802): `serve_command` only builds the server's argv, so the rig runs that
  > //!   argv itself, in a process group of its own, in the scratch project dir, waits for its
  > //!   health with the engine's own wait (`holler_pane::tx_launch::wait_healthy`), then runs
  > //!   the model guard. This crate may not depend on the host adapter (ADR-0021 section 5), so
  > //!   the server does not run under tmux here;
  > //! - two ports from 48100-48199 only, each refused and bindable when picked and not yet
  > //!   handed out in this process;
  > //! - a private tmux server (`-S <scratch>/tmux.sock -f /dev/null`) with sessions
  > //!   `demo-c1r1` and `demo-c2r1` running a placeholder (`sleep 3600`), for the panes
  > //!   `w9:p1` and `w9:p2`; never the default tmux server.

- **E47.** The dead-end provider config, verbatim from the spike.
  **Source:** `crates/holler-adapter-opencode/tests/real_opencode/rig.rs:62-80`
  **Verbatim excerpt:**
  >   "disabled_providers": ["opencode"],
  >   "model": "deadend/none",
  >   "small_model": "deadend/none",
  >   "autoupdate": false,
  >   "share": "disabled",
  >   "provider": {
  >     "deadend": {
  >       "npm": "@ai-sdk/openai-compatible",
  >       "name": "dead end (closed local port)",
  >       "options": { "baseURL": "http://127.0.0.1:9/v1" },
  >       "models": { "none": { "name": "none" } }
  >     }
  >   }
  > }
  > "#;
  >
  > /// Whether this run opted in. When it did not, the test returns at once, saying why.
  > pub fn opted_in(test: &str) -> bool {
  >     let on = std::env::var(OPT_IN).is_ok_and(|v| v == "1");

- **E48.** The opt-in check of the real OpenCode tests.
  **Source:** `crates/holler-adapter-opencode/tests/real_opencode/rig.rs:82-91`
  **Verbatim excerpt:**
  >         eprintln!("{test}: skipped: set {OPT_IN}=1 to run it against a real OpenCode and tmux");
  >     }
  >     on
  > }
  >
  > /// `name` on `PATH`, as an absolute path.
  > fn on_path(name: &str) -> Option<PathBuf> {
  >     let path = std::env::var_os("PATH")?;
  >     std::env::split_paths(&path)
  >         .map(|dir| dir.join(name))

- **E49.** The real tmux tests: `#[ignore]`, a private server on a socket in a fresh `/tmp` directory, no default socket, names under `demo-*`, a guard that kills the server.
  **Source:** `crates/holler-adapter-host/tests/real_tmux_test.rs:1-12`
  **Verbatim excerpt:**
  > #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #641
  > //! `TmuxHost` against a real tmux (#641, AC 1-5 and AC 11-14). Every test is
  > //! `#[ignore]` (the issue's opt-in): run them with
  > //! `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored`. Each returns
  > //! early, saying why, when `tmux -V` cannot run.
  > //!
  > //! Every host is built by [`private`] on its own tmux server, on a socket in a fresh
  > //! `/tmp/hlr-tmux-*` directory, with a config that forces `/bin/sh` (AC 9). The guard
  > //! kills that server and removes its directory. No test touches the default socket or
  > //! names a session outside `demo-*`. The real `kill` is used, and only ever on a process
  > //! the test started on its private server (Decision 14).
  >

- **E50.** The placeholder target and its manifest entry; the pane_integration test file is `tests/pane_integration/main.rs`.
  **Source:** `crates/holler-cli/Cargo.toml:500-508`
  **Verbatim excerpt:**
  > # (profile apply on scratch instances) fill these in; declared here so neither
  > # story edits this manifest.
  > [[test]]
  > name = "pane_integration"
  > path = "tests/pane_integration/main.rs"
  >
  > [[test]]
  > name = "profile_apply_scenario"
  > path = "tests/profile_apply_scenario/main.rs"

- **E51.** The placeholder file #649 fills.
  **Source:** `crates/holler-cli/tests/pane_integration/main.rs:1-2`
  **Verbatim excerpt:**
  > //! Placeholder target for story #649 (hub wiring and the pane-control integration
  > //! scenario). Declared by #670 so #649 adds a file here and edits no manifest.

- **E52.** `holler-cli` has no adapter crate as a dependency today.
  **Source:** `crates/holler-cli/Cargo.toml:390-402`
  **Verbatim excerpt:**
  > [dependencies]
  > clap = { workspace = true }
  > # body_cmd.rs's shared `doc_outcome` (issue #229): generic over any of
  > # `holler_body::query`'s document types via their derived `Serialize`.
  > serde = { workspace = true }
  > # stub-acp bin (serializes its ACP frames) and the body-join e2e test's
  > # `#[cfg(test)]` unit test (decodes the credential/status documents it inspects).
  > serde_json = { workspace = true }
  > holler-proto = { path = "../holler-proto" }
  > # #670: the pane/profile verbs take every error code, guard and port type from
  > # the holler-pane crate (`Ports`, `PaneError`, `GridPos`, `Argv`, `EnvVarName`).
  > holler-pane = { path = "../holler-pane" }
  > # Story #143: the `hub serve` / `hub status` leaves drive the hub crate (the

- **E53.** The CLI test harness runs the real `holler` binary on a state dir through `HOLLER_STATE_DIR`.
  **Source:** `crates/holler-cli/tests/support/mod.rs:112-125`
  **Verbatim excerpt:**
  > }
  >
  > /// Build a `Command` that invokes the built `holler` binary with this state
  > /// dir active and deterministic logging (quiet + json) so tests can parse
  > /// stdout/stderr as JSON lines.
  > pub fn holler_cmd(state: &StateDir) -> Command {
  >     let mut cmd = Command::new(holler_bin());
  >     cmd.env("HOLLER_STATE_DIR", state.path())
  >         .env("HOLLER_DEBUG", "quiet")
  >         .env("HOLLER_LOG_FORMAT", "json");
  >     cmd
  > }
  >
  > /// The absolute path to the built `holler` binary.

- **E54.** The hub harness: `Hub::start(&StateDir)` and `stop`.
  **Source:** `crates/holler-cli/tests/support/mod.rs:241-256`
  **Verbatim excerpt:**
  >     pub fn start(state: &StateDir) -> Hub {
  >         Self::start_with_env(state, &[])
  >     }
  >
  >     /// [`Hub::start`] with extra environment variables on the hub process
  >     /// itself (issue #190's `say_to_stalled_session_names_stalled`: the
  >     /// `HOLLER_STALL_MS` a busy-check reads is the *hub* process's own env,
  >     /// not the racing `say` CLI invocation's — setting it only on the racer
  >     /// has no effect, since the check runs inside the hub over the control
  >     /// socket).
  >     pub fn start_with_env(state: &StateDir, envs: &[(&str, &str)]) -> Hub {
  >         Self::start_with_args(state, envs, &["--listen", "127.0.0.1:0"])
  >     }
  >
  >     /// [`Hub::try_start_with_args`], panicking (with the hub's stderr) if the
  >     /// hub never reports a listening address.

- **E55.** The envelope helper every JSON line must pass.
  **Source:** `crates/holler-pane-testkit/src/envelope.rs:195-203`
  **Verbatim excerpt:**
  > impl std::error::Error for EnvelopeFault {}
  >
  > /// Checks that `stdout` is exactly one envelope that agrees with `exit_code`, the exit
  > /// code of the verb that wrote it.
  > ///
  > /// Returns the envelope, or the fault of the first rule it breaks, in the order the
  > /// module doc lists.
  > pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
  >     check_exit_code(exit_code)?;

- **E57.** The long-poll window a client of `*/watch` sizes its timeout from: `WATCH_WAIT` (4 s) plus a margin.
  **Source:** `crates/holler-hub/src/panes/mod.rs:72-82`
  **Verbatim excerpt:**
  >
  > /// The long-poll window of `pane/watch` and `profile/watch` (#661), and of `next()` on a
  > /// `Watch` iterator of either registry: how long a request waits for a change before it
  > /// answers an empty batch (D5).
  > ///
  > /// It is below the control client's default timeout (5 s) and within I5's 10 s. A client
  > /// that sends `pane/watch` must set its own timeout to this window plus a margin. The
  > /// control precedent adds 5 s (`control/say`, `control/wait`), because the 1 s left under
  > /// the default can be used up by a writer that holds the lock across a slow save. #649
  > /// reads this constant to size `ControlCall.timeout`.
  > pub const WATCH_WAIT: Duration = Duration::from_secs(4);

- **E56.** The hub is the store only and registers no adapter; `pane_wiring.rs` is an empty placeholder.
  **Source:** `crates/holler-hub/src/pane_wiring.rs:1-6`
  **Verbatim excerpt:**
  > //! Reserved by issue #669 (epic #633); deliberately empty.
  > //!
  > //! The pane verbs run CLI-side, against the ports, and the hub is the store only: it
  > //! registers no adapters (epic #633, ruling 0). The adapters are wired in
  > //! `holler-cli`'s `pane/wiring.rs` (#649), so this module has no job. It stays
  > //! declared in `lib.rs` as an empty placeholder.
