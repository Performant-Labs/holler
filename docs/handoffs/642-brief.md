# Brief: #642 the OpenCode adapter: `HarnessPort` over OpenCode's HTTP API and a tmux-hosted TUI

Repo: Performant-Labs/holler. Issue: #642 (epic #633, wave 3). Rigor: in-session. UI surface: no. Kind: feature (adapter).

**Branch:** `issue-642-implementation`, based on `9d61c9f` (`origin/main`: #637 slice a, ADR-0021, the OpenCode spike #635,
and every slice of the test kit #638, including slice e #684 with `FakeHarness` and the harness conformance suite).
**Design (D):** N/A (no UI). **Forward-compat:** done, see the table below. **Decision record:** ADR-0021 (sections 5 and 7,
"Deferred to named stories"), the epic's "Skeleton split" rulings, and `docs/research/opencode-pane-spike.md`, which is the
source of every fact about real OpenCode used here. The issue is the source of truth; where this brief turns the issue's
wording into a concrete design, the choice is listed under "Decisions made in this brief".

## Size check

**Fits one run; no split.** One component family (one crate, `holler-adapter-opencode`). F writes four files: the manifest
and three source files. T writes two test files. One CHANGELOG entry. Seven files in all, which is at F's cap of about six
plus the CHANGELOG.

| File | Who | Lines (est.) |
|---|---|---|
| `crates/holler-adapter-opencode/Cargo.toml` | F | ~35 |
| `crates/holler-adapter-opencode/src/lib.rs` (config, `OpenCodeHarness`, the `HarnessPort` impl, serve) | F | ~380 |
| `crates/holler-adapter-opencode/src/http.rs` (blocking loopback HTTP/1.1 client) | F | ~200 |
| `crates/holler-adapter-opencode/src/tui.rs` (tmux calls, title and start-command parsing) | F | ~200 |
| `crates/holler-adapter-opencode/tests/hermetic_test.rs` (stub server; runs in CI) | T | ~480 |
| `crates/holler-adapter-opencode/tests/real_opencode_test.rs` (opt-in; real OpenCode and tmux) | T | ~560 |
| `CHANGELOG.md` (one entry) | F | ~8 |
| **Total** | | **~1,860** (production ~815, tests ~1,040) |

For scale: #684 (slice e of the test kit) landed at about 2,500 lines in one passing run. **Fallback if F's scope cap fires
anyway:** 642a = `http.rs` plus `serve`, `health`, `create_session`, `list_sessions`, `abort` (server-side, hermetic tests
only); 642b = `tui.rs` plus `attach_tui`, `select_session`, `shown_session` and the real-OpenCode test file. If `lib.rs` nears
600 lines, move `serve` and its boot poll to `src/server.rs` (`mod server;` in `lib.rs`); if `real_opencode_test.rs` nears
600, move its rig to `tests/real_opencode/rig.rs` (`#[path]` module). No file may reach 900 lines (`scripts/lint.sh`
check 4).

**What of the issue is NOT lifted from the spike scripts, and why:** the restart probe (a restarted server keeps the
sessions and the old TUI re-subscribes), the boot-race measurement and the load run stay in `scripts/spikes/opencode-*.sh`,
unchanged and still runnable. They record recovery and performance behaviour, not the port contract; #647 (reconcile) and
#644 (relaunch) are where they become assertions. Every spike assertion the adapter itself depends on is lifted (AC 13-19).

## Problem

`holler-adapter-opencode` is an empty crate. The pane verbs (#644 launch, #645 switch and reset, #647 reconcile) need a real
`HarnessPort` so that Holler starts each pane's OpenCode server, creates the session of record over the API, attaches the
pane's TUI to exactly that session, switches it without typing, and observes which session the TUI shows. The spike found
that raw OpenCode acknowledges two things it did not do (an abort of an unknown id; a `select-session` with no TUI), broadcasts
a switch to every TUI of a server, has no API that reports the shown session, and wedges in a way only a client timeout can
see. The adapter must hide all of that behind the contract the merged conformance suite pins.

## Evidence (verbatim, as of `9d61c9f`)

The crate today (the whole of both files):
```
crates/holler-adapter-opencode/src/lib.rs:1-5
//! `holler_adapter_opencode` — the OpenCode adapter: it implements
//! `holler_pane::HarnessPort` (serve, health, sessions, abort, attach) for the
//! OpenCode harness (epic #633).
//!
//! Empty skeleton (story #637); story #642 fills it.

crates/holler-adapter-opencode/Cargo.toml:13-19
# Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI
# otherwise). The crate is an empty skeleton; its owning story adds the
# dependencies it uses.
[dependencies]

# Workspace lints (issue #149).
[lints]
workspace = true
```
The port (merged, frozen; this story implements it exactly and changes no signature):
```
crates/holler-pane/src/ports.rs:170-200
/// The harness server of a pane and its sessions (the adapter is
/// `holler-adapter-opencode`, #642). **Provisional** until spike #635 reports.
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
pub trait HarnessPort: Send + Sync {
    /// Start the harness server for `name` on `port`; returns its process id.
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError>;
    /// Whether the server on `port` answers.
    fn health(&self, port: u16) -> Result<bool, PaneError>;
    /// Create a session on the server at `port`; returns its id.
    fn create_session(&self, port: u16) -> Result<String, PaneError>;
    /// The session ids on the server at `port`.
    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError>;
    /// Abort the running turn of `session`.
    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError>;
    /// Attach the TUI of `pane` to `session` on the server at `port`.
    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError>;
    /// Switch the TUI of `pane` to `session`.
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError>;
    /// The session the TUI of `pane` shows, if it can tell.
    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError>;
}
```
Note what the trait does not carry, which shapes the design: `serve` gets no working directory; `select_session` and
`shown_session` get no port; `attach_tui` gets no directory; a `PaneId` is Herdr's opaque id (`w1:p7` form, per
`docs/research/herdr-api-spike.md:165`), not a tmux target.
```
crates/holler-pane/src/pane.rs:77-88
pub struct PaneId(String);
impl PaneId {
    /// A pane id from Herdr's text.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
    /// The id, verbatim.
    pub fn as_str(&self) -> &str { &self.0 }
}
docs/adr/ADR-0021.md:37   | `name` | `PaneName` | ... It is also the tmux session name. ...
docs/adr/ADR-0021.md:39   | `herdr` | `HerdrPane { session, workspace, pane_id, grid: GridPos }` | `pane_id` is Herdr's opaque id (`PaneId`). |
```
The closed codes the adapter returns (no new code; `error.rs` is #637's and frozen):
```
crates/holler-pane/src/error.rs:455-467
    /// `op` names it. (#638-#642.)
    Timeout { op: String },
    /// `pane-not-found`: no pane of that name; `what` is the name. (#638-#642.)
    PaneNotFound { what: String },
    /// `session-not-found`: no harness session of that id; `what` is the id.
    /// (#638-#642.)
    SessionNotFound { what: String },
    ...
    /// `unavailable`: something the verb needs cannot be reached: the hub, the Herdr
    /// socket, a harness. `what` names it. (#638-#642.)
    Unavailable { what: String },
crates/holler-pane/src/error.rs:291    | PaneCode::SessionNotFound => ErrorClass::Refusal,
crates/holler-pane/src/error.rs:298    | PaneCode::Unavailable   (... => ErrorClass::Failure)
docs/adr/ADR-0021.md:393   | `unavailable` | Failure (1) | The hub, the Herdr socket or a harness cannot be reached (also a garbled reply). |
```
The suite the adapter must pass (merged), and the rules it fixes:
```
crates/holler-pane-testkit/src/conformance/harness.rs:10-21
//! with one on fails the suite. Decided here (#684), and binding on the adapter:
//!
//! - `abort`, `attach_tui` and `select_session` of an id the server does not know are
//!   `session-not-found` (cases 8, 11 and 13). Raw OpenCode acknowledges an abort of an
//!   unknown id, so the adapter checks `GET /session/:id` first.
//! - A call to a port whose server does not answer is `unavailable`, checked before the
//!   session id (case 6).
//! - `select_session` on a pane with no TUI is `unavailable`, checked before the session
//!   id (case 14): the method takes no port, so the TUI is what names the server, and
//!   raw OpenCode acknowledges a switch that no TUI saw. Its class is a failure (exit 1),
//!   as a timeout's is.
//! - `shown_session` of a pane with no TUI is `Ok(None)` (case 9).
crates/holler-pane-testkit/src/conformance/harness.rs:50-59
/// What a harness under test gives the suite for one case: two free ports whose servers
/// share ONE data directory, as the live fleet's do, and two panes in which a TUI can be
/// attached.
pub struct HarnessRig {
    /// Two ports with no server on them yet.
    pub ports: [u16; 2],
    /// Two panes with no TUI in them yet.
    pub panes: [PaneId; 2],
}
crates/holler-pane-testkit/src/conformance/harness.rs:122-145
/// `fresh` is called once per case. It returns the harness, the rig the case runs on and
/// a guard that the suite keeps alive for that case only; the harness is dropped before
/// its guard. No server may run on the rig's ports yet, and their servers must share one
/// data directory; no TUI may be in its panes yet. ...
/// // holler-adapter-opencode (#642): per case, a scratch dir with the spike's isolated
/// //   env and dead-end provider (opencode-pane-spike.md:16-35), two free ports from
/// //   48100-48199 whose servers share that one data directory, and two panes of a
/// //   private tmux server; the scratch dir and the process groups as the guard.
pub fn run_harness_conformance<S, K, F>(mut fresh: F) -> Conformance
where S: HarnessPort, F: FnMut() -> (S, HarnessRig, K),
crates/holler-pane-testkit/src/conformance/mod.rs:40-41
/// `Ok` when every case held; otherwise every failure, in case order.
pub type Conformance = Result<(), Vec<CaseFailure>>;
```
The 15 case ids (`conformance/harness.rs:78-115`): `health-of-unserved-port-is-false`, `serve-then-healthy`,
`fresh-server-has-no-sessions`, `create-session-is-listed`, `sessions-shared-across-servers`,
`calls-to-unserved-port-are-unavailable`, `abort-known-session`, `abort-unknown-is-session-not-found`,
`shown-without-tui-is-none`, `attach-shows-the-session`, `attach-unknown-is-session-not-found`,
`select-switches-the-shown-session`, `select-unknown-is-session-not-found`, `select-without-tui-fails`,
`select-reaches-only-its-pane`. Case 15 attaches pane 0 to port 0 and pane 1 to port 1, both on session A, selects B on
pane 0 and requires pane 1 to still show A.

The fake's behaviour for the cases the suite cannot drive (frozen and killed servers), which the adapter matches (AC 15-16):
```
crates/holler-pane-testkit/src/harness.rs:110-121
/// A call that *reaches* a port goes on when the server there runs. When the server is
/// frozen, the call answers `timeout` with the method's [`HarnessOp`] as its `op`, the
/// shape a wedged port answers; when it was killed or never served, the call answers
/// `unavailable` ("the harness server on port N"). ...
/// - `health(port)`: `true` when the server runs, `false` when it is frozen, killed or
///   never served, as a timed request that times out or is refused would tell.
crates/holler-pane-testkit/src/harness.rs:135-137
/// - `shown_session(pane)`: the session the TUI shows, or `None` on its home screen or
///   with no TUI. It does not reach the server: a TUI keeps its screen while its server
///   is frozen or dead (opencode-pane-spike.md:193-194).
crates/holler-pane-testkit/src/harness.rs:47-54   (the `op` strings)
            HarnessOp::Serve => "harness.serve",            HarnessOp::Health => "harness.health",
            HarnessOp::CreateSession => "harness.create_session", HarnessOp::ListSessions => "harness.list_sessions",
            HarnessOp::Abort => "harness.abort",             HarnessOp::AttachTui => "harness.attach_tui",
            HarnessOp::SelectSession => "harness.select_session", HarnessOp::ShownSession => "harness.shown_session",
```
The three `ASSUMPTION (#642 to confirm)` comments in the fake (`harness.rs:306-307`, `332-335`, `348-350`):
```
// ASSUMPTION (#642 to confirm): abort stops a model turn as it stops a shell command;
// the spike verified only a shell command (opencode-pane-spike.md:179, 267).
// ASSUMPTION (#642 to confirm): select-session switches a TUI whose --dir is another
// project directory than the session's; the spike did not try it (opencode-pane-spike.md:272-273).
// ASSUMPTION (#642 to confirm): the shown session is read from the pane's terminal
// title; the spike read it through tmux, and whether Herdr exposes a pane's terminal
// title is unverified (opencode-pane-spike.md:151-152, 269).
```
What real OpenCode does (spike, verified on 1.18.35 unless marked):
```
docs/research/opencode-pane-spike.md:58-66
opencode serve --pure --port <port> --hostname 127.0.0.1      # cwd = the project directory
GET  /global/health            -> 200 {"healthy":true,"version":"1.18.35"}
POST /session  {"title":"…"}   -> 200 {"id":"ses_…","directory":"<project dir>","title":"…",…}
GET  /session                  -> 200 [ …sessions, most recently updated first… ]
GET  /session/:id              -> 200 session | 404 {"name":"NotFoundError",…}
PATCH /session/:id {"title":…} -> 200 session (renames it)
DELETE /session/:id            -> 200 true | 404 when already gone
GET  /session/status           -> 200 {} when nothing runs; {"ses_…":{"type":"busy"}} while busy
docs/research/opencode-pane-spike.md:93     opencode attach http://127.0.0.1:<port> --dir <project dir> --session <ses_id>
docs/research/opencode-pane-spike.md:100-101  An unknown session id makes `attach` exit with status 1 ... It fails loudly
docs/research/opencode-pane-spike.md:113    `POST /tui/select-session {"sessionID":"ses_…"}` ... → `200 true`.
docs/research/opencode-pane-spike.md:117-118  An unknown id answers `404 NotFoundError` and the TUI stays where it was. A malformed id (not `ses…`) answers `400`.
docs/research/opencode-pane-spike.md:119-124  Caveat 1: broadcast ... Caveat 2: no acknowledgement. On a server with no TUI attached, `select-session` still answers `200 true`.
docs/research/opencode-pane-spike.md:137-148  The TUI sets its terminal title, and tmux reports it as `#{pane_title}` ...
  - `OC | <session title>` while a session is shown. It follows `select-session` (about 110 ms) ...
  - `OpenCode` on the home screen and right after the shown session is deleted.
  - A title longer than 40 characters is cut ...
  `title = len > 40 ? title[0..37] + "…" : title`. The TUI shows `OpenCode` for the home route **and** for a
  session whose title is still the default `New session - …` ... Setting `OPENCODE_DISABLE_TERMINAL_TITLE` turns the title off.
docs/research/opencode-pane-spike.md:150-151  The simplest such title is the id itself: `ses_` plus 26 characters is 30 characters.
docs/research/opencode-pane-spike.md:171-172  **Unknown id:** `200 true`, the same as success. Holler must check `GET /session/:id` (404) first
docs/research/opencode-pane-spike.md:189-192  Frozen (SIGSTOP): a TCP connect still succeeds ... `GET /global/health` gets no answer ...
                                             Killed (SIGKILL): the connection is refused within about 6 ms.
docs/research/opencode-pane-spike.md:198-201  ... 1 or 2 GETs sent at about 570-680 ms were accepted and never answered in 30 s ...
  **Every request to OpenCode needs a client timeout, and none but a health poll may be sent before the first healthy answer.**
docs/research/opencode-pane-spike.md:222   TUI whose session is deleted | verified: it leaves the session within about 110 ms, ... goes to the
  home screen (title `OpenCode`), **stays running**, and creates no replacement session.
scripts/spikes/opencode-api.sh:76-78       POST /session/$SID/shell {"agent":"build","command":"sleep $MARK"}   (busy without a model)
scripts/spikes/opencode-lib.sh             the isolated env (`env -i`, HOME and XDG_* in the scratch dir, OPENCODE_DISABLE_*=1),
                                           the dead-end provider config (`deadend`, base URL http://127.0.0.1:9/v1, built-in
                                           `opencode` provider disabled) and `oc_guard_no_model` (GET /config/providers must be
                                           exactly that, and 127.0.0.1:9 must refuse). Lift them verbatim into the test rig.
```
Where the adapter is wired later (not this story):
```
crates/holler-cli/src/pane/wiring.rs:8-13
//! **Stub (story #670).** `connect` hands out [`Unwired`], whose every method answers
//! `not-implemented`, so a verb that runs before its wiring exists fails loudly and never acts.
//! Story #649 replaces the body of `connect` (and the fields of `Wiring`) with the real stores and
//! adapters, ...
```
Dependencies available without touching the workspace manifest (a shared hot spot, #637 only):
```
Cargo.toml:66-72
# The hub performs the WebSocket server handshake by hand (tokio-tungstenite
# only ships a *client* async handshake). `httparse` parses the upgrade GET;
...
httparse = "1.10.1"
Cargo.toml:161     tempfile = "3"
crates/holler-hub/Cargo.toml:23    httparse = { workspace = true }      (the analogous hand-rolled HTTP)
scripts/lint.sh:54-60   # 5. Dependency features must name their consumer ... (a member `features = [...]` needs a comment)
```
Test conventions:
```
crates/holler-pane-testkit/tests/harness_conformance_test.rs:1
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #684
clippy.toml: too-many-lines-threshold = 100, cognitive-complexity-threshold = 15  (denied in [workspace.lints.clippy])
.github/workflows/ci.yml:21    os: [ubuntu-latest, macos-latest]
.github/workflows/ci.yml:128   run: cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
crates/holler-cli/tests/reconnect_contract_test.rs:15   gated behind `HOLLER_TEST_HOOKS=1`  (the HOLLER_TEST_* opt-in convention)
```

## The public API (fixed here, so T can write RED tests against it)

`src/lib.rs` (names and fields are binding; private helpers are F's choice):
```rust
pub mod http;
pub mod tui;

/// Maps a key to what the adapter needs; an `Err` is returned to the caller as is.
pub type Resolver<K, V> = std::sync::Arc<dyn Fn(&K) -> Result<V, PaneError> + Send + Sync>;

pub enum ProcessEnv {
    /// The child inherits the environment it is started with.
    Inherit,
    /// The child gets exactly these variables (`env -i` semantics). Tests use it for the scratch HOME/XDG_*.
    Isolated(Vec<(String, String)>),
}

pub struct TmuxConfig {
    pub tmux_bin: PathBuf,          // "tmux"
    pub socket: Option<PathBuf>,    // `tmux -S <socket>`; None = the default server (production only; tests always Some)
}

pub struct Timeouts {
    pub call: Duration,      // every port method returns within this (I5); default 10 s
    pub request: Duration,   // one HTTP request; default 5 s (capped by what is left of `call`)
    pub health: Duration,    // one health GET; default 2 s
    pub boot_try: Duration,  // one health GET while `serve` waits for a fresh server; default 500 ms
    pub settle: Duration,    // how long a switch, attach or abort may take to be observed; default 5 s
}
impl Default for Timeouts { /* the defaults above */ }

pub struct OpenCodeConfig {
    pub opencode_bin: PathBuf,                // "opencode"
    pub serve_args: Vec<String>,              // appended after `serve --port P --hostname 127.0.0.1`; tests pass ["--pure"]
    pub env: ProcessEnv,                      // for both `serve` and the TUI's `attach`
    pub tmux: TmuxConfig,
    pub workdir: Resolver<PaneName, PathBuf>, // the project directory a pane's server runs in (#649: from Pane.host.cwd)
    pub tui_target: Resolver<PaneId, String>, // the tmux target of a pane's TUI (#649: from Pane.host.tmux)
    pub timeouts: Timeouts,
}

pub struct OpenCodeHarness { /* private: the config */ }
impl OpenCodeHarness { pub fn new(config: OpenCodeConfig) -> Self; }
impl HarnessPort for OpenCodeHarness { /* below */ }
```
`src/http.rs` (public so the tests reuse it rather than writing a second client):
```rust
pub struct Reply { pub status: u16, pub body: Vec<u8> }
pub enum HttpError {
    Refused,                 // nothing listens (connect refused)
    TimedOut,                // connected, no full answer within the timeout (a frozen server)
    Garbled(String),         // not an HTTP/1.1 response we can read
}
/// One request to http://127.0.0.1:<port><path>, `Connection: close`, JSON body if given.
pub fn request(port: u16, method: &str, path: &str, body: Option<&serde_json::Value>, timeout: Duration)
    -> Result<Reply, HttpError>;
```
`src/tui.rs`:
```rust
pub enum TitleShows { Session(String), Home, Unrecognised }
/// `OC | <id>` with a whole session id (`ses_` then [0-9A-Za-z]+, no `…`) -> Session(id);
/// exactly `OpenCode` -> Home; anything else (the host name tmux shows by default, a truncated or
/// non-id title, empty) -> Unrecognised.
pub fn parse_title(title: &str) -> TitleShows;
/// The loopback port of an `opencode attach http://127.0.0.1:<port> ...` command line, or None
/// when the line is not such an attach (another program, `serve`, a non-loopback URL).
pub fn attach_port(command_line: &str) -> Option<u16>;
```

## Behaviour (what each method does; F implements, T tests)

Every method takes a deadline of `now + timeouts.call` at entry; every request and poll inside it uses the smaller of its own
timeout and what is left. Past the deadline the method answers `timeout` with its `op` (`"harness.<method>"`, the fake's
strings above). HTTP outcomes map the same way everywhere: `Refused` -> `unavailable` ("the harness server on port N");
`TimedOut` -> `timeout { op }`; `Garbled`, or a status the step does not expect -> `unavailable` naming the route and status.
Only `127.0.0.1` is ever contacted (epic decision 3).

- **`serve(name, port)`**: one health GET first. Healthy -> `unavailable` ("port N is in use"), nothing spawned (never adopt
  a server). `TimedOut` -> `timeout { op: "harness.serve" }` (a frozen server holds the port). `Refused` -> resolve
  `workdir(name)` (an `Err` is returned as is), then spawn `<opencode_bin> serve --port P --hostname 127.0.0.1 <serve_args>`
  with the project as its working directory, in a **process group of its own** (`std::os::unix::process::CommandExt::
  process_group(0)`, no `unsafe`), stdin/stdout/stderr null, env per `ProcessEnv`. A binary that cannot be started ->
  `unavailable` naming the binary. Poll only `GET /global/health` (per try `boot_try`, about every 100-200 ms) until healthy,
  and send nothing else before that (spike 198-201). The child exiting first -> `unavailable` with its exit status. The
  deadline passing -> kill the process group it started (e.g. `kill -KILL -- -<pgid>` as a command) and answer
  `timeout { op: "harness.serve" }`. On success return the child's pid (which is also its process group id). The child is
  not killed when the adapter is dropped (it outlives the CLI); reap it on a detached thread so a long-lived caller keeps no
  zombie.
- **`health(port)`**: `GET /global/health` with `timeouts.health`. `Ok(true)` only for status 200 with JSON
  `"healthy": true`; `Ok(false)` for refused, timed out, garbled or anything else. Never `Err`.
- **`create_session(port)`**: `POST /session` with body `{}` -> the id (must start with `ses`); then
  `PATCH /session/<id>` `{"title": "<id>"}` and require the reply's title to equal the id (unique, at most 40 characters,
  maps back to the id; spike 149-151). If the PATCH fails, `DELETE /session/<id>` (best effort) and answer the PATCH's
  error, so no untitled session of record is left behind. Never `--continue`, never "most recent".
- **`list_sessions(port)`**: `GET /session` -> the `id` of every element, in the order given (the suite reads it as a set).
- **`abort(port, session)`**: `GET /session/<id>` (404 -> `session-not-found`, and no abort is sent; spike 171-172), then
  `POST /session/<id>/abort` (200), then poll `GET /session/status` until the id is absent or not `busy`, within `settle`;
  still busy -> `timeout { op: "harness.abort" }`.
- **`attach_tui(pane, port, session)`**: `GET /session/<id>` first: refused -> `unavailable`, 404 -> `session-not-found`;
  in both cases the pane is not touched (cases 6, 11). Take `directory` from that reply. Resolve `tui_target(pane)`; a target
  tmux cannot find -> `unavailable` ("tmux pane <target>"). Set `remain-on-exit on` for that pane, then
  `respawn-pane -k -t <target> -c <directory>` with an **argv of several arguments** (tmux then execs it without a shell):
  `env -i K=V ... <opencode_bin> attach http://127.0.0.1:<port> --dir <directory> --session <id>` under `Isolated`, or
  `env -u OPENCODE_DISABLE_TERMINAL_TITLE <opencode_bin> attach ...` under `Inherit` (the title channel must stay on;
  spike 147, 151). This replaces whatever ran in the pane, as the fake's "replacing any earlier one". Then poll
  `shown_session(pane)` until it is `Some(id)`, within `settle` -> `Ok`. If the pane's process dies first: re-`GET` the
  session; 404 -> `session-not-found`, otherwise `unavailable` ("the TUI in pane P exited with status N"). Deadline ->
  `timeout { op: "harness.attach_tui" }`.
- **`select_session(pane, session)`**: find the pane's TUI first (case 14): resolve `tui_target(pane)` and ask tmux for
  `#{pane_dead}`, `#{pane_start_command}` and `#{pane_title}` in one `display-message -p`; no tmux pane, a dead pane, or a
  start command for which `attach_port` is `None` -> `unavailable` ("no TUI in pane P"). The port comes from `attach_port`.
  Then `GET /session/<id>` on that port (refused -> `unavailable`, 404 -> `session-not-found`, the screen untouched; case 13),
  `POST /tui/select-session` `{"sessionID": "<id>"}` (200; 404 -> `session-not-found`), then poll `shown_session(pane)` until
  `Some(id)` within `settle`; otherwise `timeout { op: "harness.select_session" }`. Because `select-session` reaches every
  TUI of the server, a switch is never trusted until the title confirms it (spike 122-126).
- **`shown_session(pane)`**: resolve `tui_target(pane)` (an `Err` is returned as is) and run the same tmux query. No tmux pane,
  no tmux server on the socket, a dead pane, or a pane not running an `opencode attach` -> `Ok(None)` (case 9). Otherwise
  `parse_title(#{pane_title})`: `Session(id)` -> `Ok(Some(id))`; `Home` and `Unrecognised` -> `Ok(None)`. It never contacts
  the OpenCode server (a TUI keeps its screen while its server is frozen or dead). A tmux binary that cannot be run ->
  `unavailable`.

## Acceptance criteria

Hermetic (in `tests/hermetic_test.rs`, no OpenCode, no tmux; they run in CI on Linux and macOS under
`cargo test --workspace`). The stub server is a `std::net::TcpListener` on `127.0.0.1:0` in a thread, with routes and a
"frozen" mode (accept, read, never answer) set per test, recording every request line it receives.

1. `http::request` reads a `Content-Length` reply and a `Transfer-Encoding: chunked` reply to the same bytes; a closed port is
   `Refused` within 1 s; a frozen stub is `TimedOut` within the timeout plus 300 ms; a non-HTTP reply is `Garbled`.
2. `health` is `Ok(true)` for `{"healthy":true,...}`, and `Ok(false)` for an unbound port, a frozen stub (within
   `timeouts.health` plus 300 ms), a 200 that is not that JSON, and a 500.
3. `create_session` sends `POST /session` then `PATCH /session/<id>` whose body's `title` equals the id, and returns the id;
   when the PATCH answers 500 it returns `unavailable` and the stub has received `DELETE /session/<id>`.
4. `list_sessions` returns the ids of the stub's `GET /session` array.
5. `abort` of an id whose `GET /session/<id>` is 404 is `session-not-found` and the stub received no `POST .../abort`; of a
   known id with `GET /session/status` = `{}` it is `Ok`, and a status that stays `busy` gives `timeout` with
   `op == "harness.abort"`.
6. On an unbound port, `create_session`, `list_sessions`, `abort` and `attach_tui` are `unavailable`; for `attach_tui` the
   `tui_target` resolver is never called (it records calls; the record is empty).
7. On a frozen stub with `Timeouts { call: 1 s, .. }`, `create_session` is `timeout` with `op == "harness.create_session"` and
   returns in under 1.5 s (the call bound caps the request timeout).
8. `serve` on a port where the stub answers healthy is `unavailable` and its message contains the port; `serve` on a free port
   with `opencode_bin` = a path that does not exist is `unavailable` and its message names that path; `serve` on a frozen
   stub is `timeout` with `op == "harness.serve"`.
9. `parse_title`: `"OC | ses_0123456789abcdefABCDEFghij"` -> `Session`; `"OpenCode"` -> `Home`; `"OC | ses_ede8…"`,
   `"OC | New session - 2026-10-09T00:00:00Z"`, `"OC | my notes"`, `"somehost"` and `""` -> `Unrecognised`.
10. `attach_port`: `"env -u OPENCODE_DISABLE_TERMINAL_TITLE /usr/local/bin/opencode attach http://127.0.0.1:48123 --dir /p
    --session ses_1"` -> `Some(48123)`; the same through `env -i A=b ...` -> `Some(48123)`; `"bash"`,
    `"opencode serve --port 48123"` and `"opencode attach http://192.0.2.1:48123"` -> `None`. Include the quoted form tmux
    prints for `#{pane_start_command}` when an argument holds a space.
11. `OpenCodeHarness` is `Send + Sync + 'static` (a compile-time assertion) and `Timeouts::default()` is
    10 s / 5 s / 2 s / 500 ms / 5 s.

Real OpenCode (in `tests/real_opencode_test.rs`): every test is `#[ignore = "needs opencode and tmux: HOLLER_TEST_OPENCODE=1
cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored"]` **and** returns at once, printing why, unless
`HOLLER_TEST_OPENCODE=1`; with it set, a missing `opencode` (`OPENCODE_BIN` or `PATH`) or `tmux` is a test failure, not a skip.
The rig (one per test, and one per conformance case):

- a fresh `tempfile` scratch dir short enough for a tmux socket path (under 100 bytes); `ProcessEnv::Isolated` with `PATH`
  (the opencode binary's dir, `/usr/bin`, `/bin`), `HOME`, `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`,
  `XDG_CACHE_HOME` inside it, `TERM=xterm-256color` and the five `OPENCODE_DISABLE_*=1` of `opencode-lib.sh`; the dead-end
  provider config written verbatim from `opencode-lib.sh`; `serve_args = ["--pure"]`; one scratch project dir that
  `workdir` returns for every pane name;
- two ports picked free from **48100-48199 only** (connect refused, not yet handed out in this process); never any other port;
- a private tmux server (`tmux -S <scratch>/tmux.sock -f /dev/null`) with sessions `demo-c1r1` and `demo-c2r1` running a
  placeholder (`sleep 3600`), and `TmuxConfig.socket` = that socket (never `None` in a test); rig `PaneId`s `w9:p1` and
  `w9:p2`, with `tui_target` mapping them to those sessions and anything else to `pane-not-found`;
- the model guard before any other call: 127.0.0.1:9 must refuse, and after the first `serve` the raw
  `GET /config/providers` must list exactly the `deadend` provider at `http://127.0.0.1:9/v1`; otherwise the test fails
  without going on. No prompt is ever sent;
- the guard: a wrapper `HarnessPort` that delegates to the adapter and records every pid `serve` returns; on drop the guard
  sends SIGKILL to each recorded process group (`kill -KILL -- -<pid>`), kills the private tmux server, then removes the
  scratch dir. It never signals a process it did not start and never touches the default tmux server.

12. `run_harness_conformance(|| rig.fresh())` is `Ok(())`: all 15 cases hold against real OpenCode.
13. Create, list, switch and report: serve; create A and B; both listed; attach pane 0 to A -> `shown_session` = `Some(A)`;
    `select_session(pane0, B)` -> `Some(B)`; `abort(port, A)` is `Ok`.
14. A deleted session under a TUI is reported, not hidden: attach pane 0 to A; raw `DELETE /session/A`; within 2 s
    `shown_session(pane0)` is `Ok(None)`; the tmux pane is still alive (`#{pane_dead}` = 0); `list_sessions` lacks A and has
    no session that was not there before.
15. A frozen server only affects calls to that server: serve ports 0 and 1; attach pane 0 to A on port 0; SIGSTOP port 0's
    process group. Then `health(p0)` is `Ok(false)` within `timeouts.health` plus 500 ms; `create_session(p0)` is `timeout`
    with `op == "harness.create_session"` within `timeouts.call` plus 500 ms; `health(p1)` is `Ok(true)` and
    `create_session(p1)` succeeds; `shown_session(pane0)` is still `Some(A)`. SIGCONT afterwards (the guard kills it anyway).
16. A killed server only affects calls to that server: SIGKILL port 0's group; `health(p0)` is `Ok(false)` and
    `create_session(p0)` is `unavailable`, each within 1 s; `list_sessions(p1)` still lists the sessions created on port 0
    (one shared data directory).
17. Abort of a busy session: make A busy with raw `POST /session/A/shell {"agent":"build","command":"sleep 37.<n>"}` on a
    background thread (`opencode-api.sh:76-78`; no model is called); once raw `GET /session/status` reports A busy,
    `abort(p, A)` is `Ok` within 1 s and the status no longer lists A as busy.
18. Raw OpenCode pins (what the adapter hides; a failure here means OpenCode changed, so re-run the spike): the raw
    `GET /doc` lists the operation ids `global.health`, `session.create`, `session.list`, `session.get`,
    `session.update`, `session.delete`, `session.status`, `session.abort`, `tui.selectSession`, `tui.showToast`; raw
    `POST /tui/select-session` on a server with no TUI answers 200 `true`; raw `POST /session/<unknown>/abort` answers 200
    `true`. Each assertion message says it is a pin.
19. `serve` refuses a port that already serves: serve port 0 for `demo-c1r1`; `serve` port 0 again for `demo-c2r1` and for
    `demo-c1r1` are both `unavailable` (see decision 2).

Gates for the whole change:

20. `cargo test --workspace` passes with the real tests skipped; `HOLLER_TEST_OPENCODE=1 cargo test -p
    holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1` passes on a machine with OpenCode 1.18.x
    and tmux 3.2 or later (record the versions and the run's output summary in `handoff-T-green.md`).
21. `cargo clippy --workspace --all-targets -- -D warnings` is clean; every new `.rs` file passes
    `rustfmt --check --edition 2021` (epic ruling 4); no new `unsafe` (`git diff origin/main | grep -c unsafe` adds 0);
    `bash scripts/lint.sh` passes (no file at 900 lines; any `features = [...]` carries its consumer comment);
    `cargo machete` reports nothing for the crate.
22. `git diff --name-only origin/main` lists only `crates/holler-adapter-opencode/**`, `CHANGELOG.md` and this run's
    `docs/handoffs/` files. No change to `holler-pane`, the test kit, the workspace `Cargo.toml` or ADR-0021.
23. `CHANGELOG.md` `[Unreleased]` has one entry under "Enhancements" that links #642 and epic #633 and says what the adapter
    does and that the real-OpenCode tests are opt-in.
24. Safety: `grep -rn "4700[0-9]\|--continue" crates/holler-adapter-opencode` finds nothing outside comments that forbid them;
    no test reads a real HOME, the default tmux server or a port outside 48100-48199 (A and S check the rig code).
25. The PR body says how each of the fake's three `ASSUMPTION (#642 to confirm)` comments stands after this run: the shown
    session is read through **tmux** `#{pane_title}`, not Herdr, so the Herdr half of that assumption no longer applies;
    cross-directory `select-session` and aborting a model turn remain **unverified** (no test exercises them; both need
    something this run must not do or have). It also says `HarnessPort` is confirmed unchanged by the spike (ADR-0021's
    "Deferred to named stories" item for #642).

## Files

Production (F): `crates/holler-adapter-opencode/Cargo.toml`, `src/lib.rs`, `src/http.rs`, `src/tui.rs`; `CHANGELOG.md`.
Tests (T): `crates/holler-adapter-opencode/tests/hermetic_test.rs`, `tests/real_opencode_test.rs`.

Manifest: `[dependencies]` `holler-pane = { path = "../holler-pane" }`, `serde_json = { workspace = true }`,
`httparse = { workspace = true }`; `[dev-dependencies]` `holler-pane-testkit = { path = "../holler-pane-testkit" }`,
`tempfile = { workspace = true }`. Each with a one-line comment naming its use, as the other manifests do. No feature lists.

### Reuse map (extend, do not duplicate)

- **Relevant code mapped:** `holler-pane/src/ports.rs` (`HarnessPort`), `holler-pane/src/error.rs` (`PaneError`),
  `holler-pane/src/pane.rs` (`PaneName`, `PaneId`); `holler-pane-testkit/src/conformance/harness.rs`
  (`run_harness_conformance`, `HarnessRig`); `holler-pane-testkit/src/harness.rs` (`FakeHarness`, the behaviour to match);
  `scripts/spikes/opencode-lib.sh` (the isolation, the dead-end config, the model guard); `holler-hub/src/ws_handshake.rs`
  (hand-rolled HTTP with `httparse`); `holler-cli/src/pane/wiring.rs` (where #649 will build the adapter).
- **Closest analogous feature:** `FakeHarness` (the same port, the same error shapes and `op` strings) and, for the HTTP
  plumbing, the hub's hand-rolled handshake over `httparse`.
- **Extend-vs-new:**
  - `HarnessPort`, `PaneError`, the codes, `run_harness_conformance`, `HarnessRig`: **reuse as is**; no edit.
  - `OpenCodeHarness`, `OpenCodeConfig`: **new**, justified: the crate is the port's one designated implementation and is
    empty.
  - `http.rs`: **new**, justified: the only OpenCode clients in the repo are async (`reqwest` + `tokio`) inside
    `holler-body` (`http_attach_driver`), a different layer an adapter must not depend on, and the port is synchronous and
    must see "connected but never answered" separately from "refused" (spike 189-192), which a blocking `TcpStream` with
    deadlines gives directly. It reuses the workspace's `httparse` for the response head; it adds no crate and changes no
    workspace feature. `reqwest`'s `blocking` feature was rejected: it would change the feature set of a shared workspace
    dependency for every crate (feature unification) and still hide the connect/answer distinction behind one timeout error.
  - `tui.rs`: **new**, justified: no tmux wrapper exists on `main` (`holler-adapter-host` is still empty; #641 is in flight in
    parallel and owns `HostPort`, keyed by `PaneName`, with no title or start-command query). An adapter must not depend on
    another adapter crate. The overlap is three tmux subcommands (`display-message`, `set-option`, `respawn-pane`); if #641
    lands a reusable tmux helper, consolidating is a later refactor, not this story's.
  - The tests' stub HTTP server: **new**, justified: the existing fake OpenCode servers are private to other crates' test
    trees (`holler-body/tests/http_attach_driver_test/fake_server.rs`, `holler-cli/tests/attach_cli_test/fake_server.rs`),
    async on `tokio`, and cannot be imported; this one must also model a frozen server.
  - `op` strings: the adapter defines its own `"harness.<method>"` constants equal to `HarnessOp::as_str`; production code
    cannot depend on the test kit. AC 7, 8 and 15 pin the equality.

### Forward-compat (the consumers of this API)

| Consumer | Needs | Satisfied |
|---|---|---|
| #644 launch/relaunch | `serve`, `create_session`, `attach_tui`, `shown_session` within I5; a pid to record | yes; `serve` refuses a port in use, so launch checks `health` and the registry first (decision 2) |
| #645 switch/reset | `select_session` confirmed by observation; `create_session`; `abort` | yes |
| #647 reconcile/doctor | `health` as a timed probe (`false`, never a hang); `shown_session` = `None` for home or a deleted session; `list_sessions` | yes |
| #649 wiring | construct the adapter from the registry: `workdir` from `Pane.host.cwd`, `tui_target` from `Pane.host.tmux` (looked up by `herdr.pane_id`) | yes, through the two `Resolver`s; no adapter change needed |
| #667 profile apply on scratch OpenCode | a scratch OpenCode environment | partly: the rig is test code here; #667 lifts it or asks for it to be shared (not a conflict) |

## Decisions made in this brief (MO)

1. **The TUI is reached through tmux, not Herdr.** A pane's TUI runs in its tmux session (`Pane.name` = `host.tmux`,
   ADR-0021:37); the spike read the title through tmux `#{pane_title}`, and Herdr's `PaneInfo.title` is report-metadata, not
   the terminal title (`herdr-api-spike.md:137, 142`). The `PaneId` -> tmux target mapping is injected (`tui_target`), so
   this crate needs no registry access. This resolves the Herdr half of the fake's third assumption.
2. **`serve` never adopts a running server.** A port that already answers is `unavailable`. The fake is more lenient (same
   pane on a running port returns the old pid), but the adapter cannot tell whose server answers or its pid without a
   process-table search, which #641's rules forbid by spirit (no matching processes by name). The conformance suite does not
   test idempotence. #644 must check `health` and the recorded pid before calling `serve`.
3. **`shown_session` answers `None` when it cannot tell** (home, a deleted session, a default or non-id title), per the
   trait's "if it can tell". It never guesses an id from a title. Reconcile treats SHOWN = none as a mismatch to fix at once
   (spike 222), which is right for every case folded into `None`.
4. **The port of a pane's TUI is observed from tmux** (`#{pane_start_command}` of a live pane), not remembered: each CLI
   run is a fresh process (epic ruling 1). If T-green finds `pane_start_command` does not follow a `respawn-pane`, F may read
   `#{pane_pid}`'s command line instead; `attach_port` takes a command line either way.
5. **`attach_tui` sets `remain-on-exit on`** on the pane before respawning it, so an exiting TUI stays observable (dead pane,
   exit status) instead of taking the tmux session with it.
6. **One TUI per server** is a precondition the adapter cannot check (`select-session` broadcasts); the registry gives each
   pane its own port, and case 15 shows a switch reaches only its pane under that rule.
7. **No ADR-0021 change.** The trait is confirmed as merged; the resolvers are wiring (#649), not contract.
8. **No operator step is needed.** Everything runs against scratch OpenCode and a private tmux server.

## Out of scope

- Any change to `holler-pane`, the test kit (including its `ASSUMPTION` comments), the workspace manifest, ADR-0021,
  `holler-cli` (wiring is #649) or `scripts/spikes/*`.
- Password-protected servers (`OPENCODE_SERVER_PASSWORD`, `attach -p/-u`; unverified in the spike), workspaces
  (`?workspace=`), cross-directory `select-session`, aborting a model turn.
- Stopping a server (`HostPort::stop_owned`, #641), restarting a wedged one (#644/#647), the restart, boot-race and load
  probes (stay in `scripts/spikes/`).
- Reading the TUI's screen as a fallback for the title.

## Test plan

**RED first (T).** Both test files compile against the API above with F's code absent, so today they fail to compile
(`holler_adapter_opencode::{OpenCodeHarness, OpenCodeConfig, http, tui}` do not exist). T may add the minimum public stubs
F would otherwise write only if the pipeline requires a runtime RED; if so, every stub method answers
`PaneError::NotImplemented` and AC 1-11 fail on assertions. Record in `handoff-T-red.md` which ACs fail and why. T runs the
real-OpenCode file once with `HOLLER_TEST_OPENCODE=1` to show RED there too, under the rig's rules (scratch dir, private
tmux, ports 48100-48199 only), and confirms that without the variable every real test returns at once.

**GREEN (T verify).** `cargo test -p holler-adapter-opencode` (hermetic); the opt-in run of AC 20 with `--test-threads=1`
(two tests must not race for the scratch port range); clippy, rustfmt, `scripts/lint.sh`, `cargo machete`. Paste the
conformance result and the OpenCode and tmux versions in `handoff-T-green.md`.

## Risks

1. **`GET /session` scope.** Case 5 needs a session created through port 1 to be listed by port 0. The spike verified
   `GET /session/:id` across servers and states the list is shared (spike 75-77); the rig runs both servers in the same
   project dir, so a per-directory filter would still pass. If it fails, the suite is right and the adapter must list with
   `?directory=` of its server; report it, do not weaken the test.
2. **Boot race.** A health GET at about 600 ms can hang (spike 198-201); `boot_try` (500 ms) abandons it and the next try
   goes. A cold first start in a fresh scratch HOME may be slower; the call bound is 10 s, and the rig may raise
   `Timeouts.call` for `serve` only if T-green shows a cold boot near it (record the measurement).
3. **Title timing.** Between `respawn-pane` and the TUI's first title (about 1.6 s), tmux shows the host name, which parses
   as `Unrecognised` -> `None`; `attach_tui` keeps polling until `settle`. The tests never print the raw tmux title
   (it can be the machine's host name); assert on the parsed value.
4. **tmux quoting of `pane_start_command`.** tmux prints the argv re-quoted; `attach_port` must find
   `http://127.0.0.1:<port>` inside quotes too (AC 10 includes a quoted line taken from a real `display-message`).
5. **Leaked processes.** A test that panics before the guard is built could leak a server. Build the guard before the first
   `serve` and record pids through the wrapper so every exit path kills them; the scratch HOME keeps any leak away from
   real state.
6. **macOS CI.** Only the hermetic file runs there; it must not call tmux, `kill`, `/proc` or OpenCode.
