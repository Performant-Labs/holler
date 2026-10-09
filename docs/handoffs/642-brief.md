# Brief: #642 the OpenCode adapter: `HarnessPort` over OpenCode's HTTP API and a tmux-hosted TUI

Repo: Performant-Labs/holler. Issue: #642 (epic #633, wave 3). Rigor: in-session. UI surface: no. Kind: feature (adapter).

**Branch:** `issue-642-implementation`, based on `9d61c9f` (`origin/main`: #637 slice a, ADR-0021, the OpenCode spike #635,
and every slice of the test kit #638, including slice e #684 with `FakeHarness` and the harness conformance suite).
**Design (D):** N/A (no UI). **Forward-compat:** done, see the table below. **Decision record:** ADR-0021 (sections 5 and 7,
"Deferred to named stories"), the epic's "Skeleton split" rulings, and `docs/research/opencode-pane-spike.md`, which is the
source of every fact about real OpenCode used here. The issue is the source of truth; where this brief turns the issue's
wording into a concrete design, the choice is listed under "Decisions made in this brief".

**Amended 2026-10-09 after A's BLOCK** (`docs/handoffs/642/handoff-A.md`, 2 BLOCK + 9 WARN). Every finding is resolved below;
no earlier decision is reversed except decision 7 (no ADR edit), which B-1 replaces:
B-1 in decision 7, "ADR-0021 and `holler-pane` doc edits", AC 22 and AC 26; B-2 in the API (`tui_session`, the `tui.rs`
builders), Behaviour, decision 9, AC 11a, 11b and 19a; W-1 in decision 2, Out of scope, Forward-compat and Follow-ups; W-2 in
the `Resolver` docs, decision 14 and Forward-compat; W-3 in decision 15, AC 25 and Follow-ups; W-4 in `TmuxSocket`, decision
10 and AC 11c; W-5 in decision 11, Files, the Reuse map and Follow-ups; W-6 in Behaviour, decision 12 and AC 11d; W-7 in
Behaviour, decision 13 and AC 11e; W-8 on `ProcessEnv` and in decision 10; W-9 in Files, AC 22 and Blast radius. The size
check now trips F's file cap; the split it names is proposed there.

## Size check

**Amended: F's file cap now trips; split proposed (the MO decides).** Lines still fit one run, but files do not. One
component family (one crate, `holler-adapter-opencode`), but F now edits by hand **nine** files against a cap of about six:
five crate files (the manifest, `lib.rs`, `http.rs`, `tui.rs`, the new private `exec.rs` of decision 11), `CHANGELOG.md`,
`docs/adr/ADR-0021.md` and two doc-comment-only files in `holler-pane` (B-1), plus the mechanical `Cargo.lock` (W-9). T writes
two test files.

| File | Who | Lines (est.) |
|---|---|---|
| `crates/holler-adapter-opencode/Cargo.toml` | F | ~35 |
| `crates/holler-adapter-opencode/src/lib.rs` (config, `OpenCodeHarness`, the `HarnessPort` impl, serve, crate docs) | F | ~420 |
| `crates/holler-adapter-opencode/src/http.rs` (blocking loopback HTTP/1.1 client) | F | ~200 |
| `crates/holler-adapter-opencode/src/tui.rs` (`TmuxSocket`, the tmux argv builders and escape, title and start-command parsing) | F | ~270 |
| `crates/holler-adapter-opencode/src/exec.rs` (private: timed subprocess runner, tmux stderr classification, the `kill` call) | F | ~150 |
| `crates/holler-adapter-opencode/tests/hermetic_test.rs` (stub server; runs in CI) | T | ~650 |
| `crates/holler-adapter-opencode/tests/real_opencode_test.rs` (opt-in; real OpenCode and tmux) | T | ~600 |
| `CHANGELOG.md` (one entry) | F | ~8 |
| `docs/adr/ADR-0021.md` (close the deferred item; "`HarnessPort` as built (#642)" note) | F | ~25 |
| `crates/holler-pane/src/ports.rs`, `crates/holler-pane/src/lib.rs` (doc comments only) | F | ~10 |
| `Cargo.lock` (mechanical) | F | ~5 |
| **Total** | | **~2,375** (production ~1,075, docs and mechanical ~50, tests ~1,250) |

Up ~+515 from the first brief (~1,860): ~+260 production (the `exec.rs` home, `TmuxSocket`, the argv builders and escape,
the JSON shape checks, the crate docs) plus ~+45 ADR, doc comments and `Cargo.lock`; ~+210 tests (AC 11a-11f, 19a).
For scale: #684 (slice e of the test kit) landed at about 2,500 lines in one passing run.

**Proposed split (the fallback this brief already named), each its own run and PR from `origin/main`:**

| Part | F files | T files | ACs | PR |
|---|---|---|---|---|
| **642a** server side | `Cargo.toml`, `src/lib.rs` (config types, `OpenCodeHarness`, `serve`, `health`, `create_session`, `list_sessions`, `abort`; the three TUI methods answer `PaneError::NotImplemented`), `src/http.rs`, `src/exec.rs` (runner and `kill` only), `src/tui.rs` (the `TmuxSocket` and `TmuxConfig` types only, so the config is final), `CHANGELOG.md`; `Cargo.lock` | `hermetic_test.rs` (AC 1-8, 11, 11d, 11e, without their `attach_tui` clauses) | those, 20 (hermetic half), 21-24 | `Part of #642` |
| **642b** TUI side | `src/tui.rs` (the builders, escape and parsers), `src/lib.rs` (`attach_tui`, `select_session`, `shown_session`), `src/exec.rs` (tmux stderr classification), `CHANGELOG.md`, `docs/adr/ADR-0021.md`, `crates/holler-pane/src/{ports.rs,lib.rs}` (doc comments) | `hermetic_test.rs` (AC 9, 10, 11a-11c, 11f, and the `attach_tui` clauses of AC 6 and 11d), `real_opencode_test.rs` | the rest, including 12 (conformance), 25, 26 | `Closes #642` |

642b carries the ADR and doc edits because their facts (SHOWN through tmux, one TUI per server) hold only once the TUI
half exists, and the conformance suite (AC 12) needs all eight methods. 642b touches seven files, three of them doc-only.
If the MO waives the cap instead (the three doc files total ~35 lines of prose), this brief runs as one story unchanged.

Within either shape: if `lib.rs` nears 600 lines, move `serve` and its boot poll to `src/server.rs` (`mod server;` in
`lib.rs`); if `real_opencode_test.rs` nears 600, move its rig to `tests/real_opencode/rig.rs` (`#[path]` module); if
`hermetic_test.rs` nears 800, move the stub server to `tests/support/stub.rs` (`#[path]` module). No file may reach 900
lines (`scripts/lint.sh` check 4).

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
Added after the plan review: the tmux behaviour the TUI calls rely on, probed by O on tmux 3.7c on a private
`tmux -S <mktemp -d>/s -f /dev/null` server (sessions `demo-c1r1` and `mo-2` running `sleep 3600`), killed and deleted
afterwards; never the default socket. Shell quoting; `<D>` is the scratch dir. (#641's brief, Evidence, has the same
results for `has-session`, `new-session -c` and `new-window`.)
```
display-message -p -t demo '#{session_name}'          -> "demo-c1r1", exit 0     PREFIX MATCH: a bare target is unsafe
display-message -p -t =demo: '[#{session_name}]'      -> "[]", exit 0            A MISSING exact target is NOT an error:
display-message -p -t '=nope:' 'v'                    -> "v", exit 0               the format expands with empty fields
list-panes -t =demo: -F ...                           -> "can't find session: demo", exit 1
display-message ... (no server ever started)          -> "error connecting to <D>/s (No such file or directory)", exit 1
display-message -p -t =demo-c1r1: '#{session_name}|#{pane_dead}|#{pane_start_command}'
                                                      -> "demo-c1r1|0|sleep 3600"; #{pane_title} is the host name until a
                                                         program sets it
set-option -p -t =demo-c1r1: remain-on-exit on        -> exit 0; #{remain-on-exit} reads "on"
respawn-pane -k -t =mo: -c <D> -- env -- sleep 101    -> "can't find session: mo", exit 1; mo-2's pane untouched
respawn-pane -k -t =demo-c1r1: -c "<D>/p#S" -- ...    -> the program's cwd was $HOME: -c is format-expanded ("p#S" became
                                                         "pdemo-c1r1", which does not exist, and tmux fell back)
respawn-pane -k -t =demo-c1r1: -c "<D>/p##S" -- ...   -> cwd "<D>/p#S" exactly
respawn-pane ... -- env -- sh -c '<print args>' f 'x;' rename-session pwned
                                                      -> the program got only "x"; a session was renamed "pwned"
respawn-pane ... -- env -- sh -c '<print args>' f 'x\;' 'a b' rename-session pwned
                                                      -> the program got "x;", "a b", "rename-session", "pwned"; no rename
#{pane_start_command} after that respawn              -> env -- sh -c "printf \"%s\\n\" \"$@\" > \"$0\"; sleep 300" <D>/args2.out "x;" "a b" rename-session pwned
                                                         (it follows respawn-pane; tmux re-quotes elements with spaces,
                                                         ";" or quotes)
TMUX=/nonexistent,1,0 tmux -f /dev/null display-message -p x
                                                      -> "error connecting to /nonexistent ...": with no -S/-L, an inherited
                                                         $TMUX picks the server
```

## The public API (fixed here, so T can write RED tests against it)

`src/lib.rs` (names and fields are binding; private helpers are F's choice):
```rust
pub mod http;
pub mod tui;
mod exec; // private (decision 11)

/// Maps a key to what the adapter needs; an `Err` is returned to the caller as is.
/// Precondition (decision 14): a resolver must answer for a pane that is being launched, before its registry record
/// exists (#644 writes the record after the act). Meeting it is wiring's job (#649), not a registry lookup's.
pub type Resolver<K, V> = std::sync::Arc<dyn Fn(&K) -> Result<V, PaneError> + Send + Sync>;

pub enum ProcessEnv {
    /// The child inherits the environment it is started with.
    Inherit,
    /// The child gets exactly these variables (`env -i` semantics). Tests use it for the scratch HOME/XDG_*.
    /// **Never put a secret here** (W-8): for the TUI every pair goes on its command line (`env -i K=V ...`), which `ps`
    /// shows and tmux reports as `#{pane_start_command}`. No error message echoes a start command or an env value.
    Isolated(Vec<(String, String)>),
}

pub use tui::{TmuxConfig, TmuxSocket}; // defined in tui.rs, below

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
    pub workdir: Resolver<PaneName, PathBuf>,      // the project directory a pane's server runs in (#649: from Pane.host.cwd)
    pub tui_session: Resolver<PaneId, PaneName>,   // the tmux SESSION NAME the pane's TUI runs in (#649: Pane.host.tmux,
                                                   // which equals Pane.name, ADR-0021:37); never a tmux target (B-2)
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
`src/tui.rs` (the builders are public and pure so the hermetic tests pin the exact argv without running tmux):
```rust
/// Which tmux server to address; the same variants as #641's `TmuxSocket`, so #649 configures one value and hands it
/// to both adapters (decision 10). An adapter cannot depend on another adapter crate, hence a mirror, not a re-export.
pub enum TmuxSocket { Default, Name(String), Path(PathBuf) }   // Default: no flag; Name: `-L n`; Path: `-S p`

pub struct TmuxConfig {
    pub tmux_bin: PathBuf,   // "tmux"
    pub socket: TmuxSocket,  // production: Default; tests: always Path (a private server)
}
// No `-f`: this adapter never starts a tmux server (it only addresses sessions that exist), so a config file has no effect.

/// A `Command` for one tmux call: `<tmux_bin> [-S p | -L n] <args...>`, with `TMUX` and `TMUX_PANE` removed from the
/// child's environment (so `Default` means tmux's own default socket, never the server named by an inherited `$TMUX`).
/// Every tmux process the adapter spawns is built here.
pub fn tmux_command(tmux: &TmuxConfig, args: &[String]) -> std::process::Command;
/// `=<session>:` — the only form in which a session reaches tmux (a bare name is prefix-matched).
pub fn exact_target(session: &PaneName) -> String;
/// #641's Decision 13 escape for a value the adapter did not author: a final `;` gets a `\` before it
/// (`x;` -> `x\;`, `y\;` -> `y\\;`, `;` -> `\;`); nothing else changes.
pub fn escape_arg(value: &str) -> String;
/// A start directory for `-c`: every `#` doubled first (tmux format-expands `-c`), then `escape_arg`.
pub fn escape_dir(dir: &str) -> String;
/// The TUI's argv, unescaped: `env -i K=V ... <bin> attach http://127.0.0.1:<port> --dir <dir> --session <id>` under
/// `Isolated`; `env -u OPENCODE_DISABLE_TERMINAL_TITLE <bin> attach ...` under `Inherit`.
pub fn tui_argv(opencode_bin: &str, env: &ProcessEnv, port: u16, dir: &str, session_id: &str) -> Vec<String>;
/// `respawn-pane -k -t =<session>: -c <escape_dir(dir)> -- <escape_arg(e) for e in tui_argv>`.
pub fn respawn_args(session: &PaneName, dir: &str, tui_argv: &[String]) -> Vec<String>;
/// `set-option -p -t =<session>: remain-on-exit on`.
pub fn remain_on_exit_args(session: &PaneName) -> Vec<String>;
/// `display-message -p -t =<session>: <FORMAT>`, where FORMAT is a constant that starts with `#{session_name}` and
/// carries `#{pane_dead}`, `#{pane_start_command}` and `#{pane_title}` (separator and order are F's choice).
pub fn query_args(session: &PaneName) -> Vec<String>;

pub enum TitleShows { Session(String), Home, Unrecognised }
/// `OC | <id>` with a whole session id (`ses_` then [0-9A-Za-z]+, no `…`) -> Session(id);
/// exactly `OpenCode` -> Home; anything else (the host name tmux shows by default, a truncated or
/// non-id title, empty) -> Unrecognised.
pub fn parse_title(title: &str) -> TitleShows;
/// The loopback port of an `opencode attach http://127.0.0.1:<port> ...` command line, or None
/// when the line is not such an attach (another program, `serve`, a non-loopback URL).
pub fn attach_port(command_line: &str) -> Option<u16>;
```
`src/exec.rs` (private, not API; decision 11): run one child with a deadline (stdin null, stdout and stderr drained on
threads, `try_wait` polling, `Child::kill` and `wait` on the deadline) returning status, stdout and stderr or a timeout; a spawn
`NotFound` maps to `unavailable` naming the binary. Classify a failed tmux call's stderr: `can't find session`, `can't find
window`, `can't find pane`, `no server running`, or `error connecting to` with `No such file or directory` or `Connection
refused` = **missing**; anything else = **other**, carrying tmux's first stderr line. And the `kill -s KILL -- -<pgid>` call
(the `kill` binary, no `unsafe`). Shaped like #641's `exec.rs`, so a later consolidation is a move (Follow-ups).

## Behaviour (what each method does; F implements, T tests)

Every method takes a deadline of `now + timeouts.call` at entry; every request and poll inside it uses the smaller of its own
timeout and what is left. Past the deadline the method answers `timeout` with its `op` (`"harness.<method>"`, the fake's
strings above). HTTP outcomes map the same way everywhere: `Refused` -> `unavailable` ("the harness server on port N");
`TimedOut` -> `timeout { op }`; `Garbled`, or a status the step does not expect -> `unavailable` naming the route and status.
Only `127.0.0.1` is ever contacted (epic decision 3).

**Reply shapes are required, not assumed (W-6, decision 12).** OpenCode answers an unknown route with its web app (`200`,
`text/html`; `holler-body/src/http_attach_driver.rs:38-49`), so a `200` alone never counts as success. Each step requires the
JSON it relies on: `GET /global/health` an object with `"healthy": true`; `POST /session` an object whose string `id` starts
with `ses`; `PATCH /session/<id>` an object whose `title` equals the id; `GET /session` an array of objects each with a string
`id`; `GET /session/<id>` an object whose `id` equals the requested id; `POST /session/<id>/abort` and
`POST /tui/select-session` the JSON value `true`; `GET /session/status` an object. Anything else (including a body that is not
JSON) is `unavailable`, with a one-line message that names the route and the status and quotes at most 60 bytes of the body,
control characters replaced (ADR-0021 §9: one-line messages). A session id enters a URL path percent-encoded (every byte
outside `[A-Za-z0-9_-]`), so no id can change the request line; for the existence check `GET /session/<id>`, a `400` (a
malformed id; spike 117-118) reads as `404`, i.e. `session-not-found`.

**Every tmux call (B-2, W-4, decisions 9 and 10)** is built by `tui::tmux_command` from one of the `tui.rs` builders, and its
`-t` is always `exact_target(session)` = `=<session>:`, where `session` is what `tui_session(pane)` returned. The adapter never
passes the resolver's value to tmux any other way and never builds a bare target. Every value the adapter did not author goes
through `escape_arg` (each element of the TUI argv after `--`, including the binary path, the session id and every `K=V` of
`Isolated`) or, for `-c`, `escape_dir` (the directory OpenCode reported). Constant formats and option values the adapter
writes are not escaped. Because `display-message -p -t` on a missing exact target exits 0 with empty fields (Evidence), a
query counts only when its first field equals the session name; otherwise there is **no tmux pane**. A failed call's stderr
is classified by `exec.rs`: missing -> no tmux pane; other -> `unavailable` with tmux's first stderr line (never an argv
element, a directory or an env value).

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
  zombie. `HarnessPort` has no stop: the server is stopped by the host adapter (#641) by this recorded pid (decision 2).
- **`health(port)`**: `GET /global/health` with `timeouts.health`. `Ok(true)` only for status 200 with JSON
  `"healthy": true`; `Ok(false)` for refused, timed out, garbled or anything else. Never `Err`.
- **`create_session(port)`**: `POST /session` with body `{}` -> the id (must start with `ses`); then
  `PATCH /session/<id>` `{"title": "<id>"}` and require the reply's title to equal the id (unique, at most 40 characters,
  maps back to the id; spike 149-151). If the PATCH fails, `DELETE /session/<id>` (best effort) and answer the PATCH's
  error, so no untitled session of record is left behind. Never `--continue`, never "most recent".
- **`list_sessions(port)`**: `GET /session` -> the `id` of every **top-level** element, in the order given (the suite reads
  it as a set). An element whose `parentID` is a string is a child (subagent) session and is left out (decision 13); a
  missing or `null` `parentID` is top-level.
- **`abort(port, session)`**: `GET /session/<id>` (404 -> `session-not-found`, and no abort is sent; spike 171-172; a 200
  whose body is not that session -> `unavailable`, and no abort is sent), then `POST /session/<id>/abort` (200 `true`), then
  poll `GET /session/status` until the id is absent or not `busy`, within `settle`;
  still busy -> `timeout { op: "harness.abort" }`.
- **`attach_tui(pane, port, session)`**: `GET /session/<id>` first: refused -> `unavailable`, 404 -> `session-not-found`;
  a 200 that is not that session -> `unavailable`; in all three cases the pane is not touched (cases 6, 11). Take `directory`
  from that reply. Resolve `tui_session(pane)` (an `Err` is returned as is). Run `remain_on_exit_args(session)`; a missing
  session -> `unavailable` ("no tmux session for pane P"). Then `respawn_args(session, directory, tui_argv(...))`, i.e.
  `respawn-pane -k -t =<session>: -c <directory> -- <TUI argv>`, an **argv of several arguments** (tmux then execs it
  without a shell): `env -i K=V ... <opencode_bin> attach http://127.0.0.1:<port> --dir <directory> --session <id>` under
  `Isolated`, or `env -u OPENCODE_DISABLE_TERMINAL_TITLE <opencode_bin> attach ...` under `Inherit` (the title channel must
  stay on; spike 147, 151), every element escaped as above. An `opencode_bin` that is not valid UTF-8 -> `unavailable`. This
  replaces whatever ran in the pane, as the fake's "replacing any earlier one". Then poll
  `shown_session(pane)` until it is `Some(id)`, within `settle` -> `Ok`. If the pane's process dies first: re-`GET` the
  session; 404 -> `session-not-found`, otherwise `unavailable` ("the TUI in pane P exited with status N"). Deadline ->
  `timeout { op: "harness.attach_tui" }`.
- **`select_session(pane, session)`**: find the pane's TUI first (case 14): resolve `tui_session(pane)` (an `Err` is returned
  as is) and run `query_args(session)` (`#{session_name}`, `#{pane_dead}`, `#{pane_start_command}` and `#{pane_title}` in one
  `display-message -p` on `=<session>:`); no tmux pane (as defined above), a dead pane, or a start command for which
  `attach_port` is `None` -> `unavailable` ("no TUI in pane P"). The port comes from `attach_port`. Then `GET /session/<id>`
  on that port (refused -> `unavailable`, 404 -> `session-not-found`, the screen untouched; case 13), `POST
  /tui/select-session` `{"sessionID": "<id>"}` (200 `true`; 404 -> `session-not-found`), then poll `shown_session(pane)`
  until `Some(id)` within `settle`; otherwise `timeout { op: "harness.select_session" }`. Because `select-session` reaches every
  TUI of the server, a switch is never trusted until the title confirms it (spike 122-126).
- **`shown_session(pane)`**: resolve `tui_session(pane)` (an `Err` is returned as is) and run the same tmux query. No tmux pane,
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
   `tui_session` resolver is never called (it records calls; the record is empty).
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

Added by the amendment (pure builders and stub cases; still no tmux and no OpenCode):

11a. **Exact targets (B-2).** `exact_target(demo-c1r1)` is `=demo-c1r1:`. `respawn_args(demo-c1r1, "/p", &tui_argv(...))`
     equals exactly `["respawn-pane", "-k", "-t", "=demo-c1r1:", "-c", "/p", "--", <the TUI argv>]`;
     `remain_on_exit_args(demo-c1r1)` is `["set-option", "-p", "-t", "=demo-c1r1:", "remain-on-exit", "on"]`;
     `query_args(demo-c1r1)` is `["display-message", "-p", "-t", "=demo-c1r1:", <a format starting with #{session_name}>]`.
     For the session `demo` every builder's `-t` value is `=demo:`, and no vector any builder returns holds a bare session
     name as a `-t` value.
11b. **Escaping (B-2, #641 Decision 13).** `escape_arg`: `x;` -> `x\;`, `y\;` -> `y\\;`, `;` -> `\;` (raw values, not
     Rust literals), and `a b`, `#{x}`, `{`, `}`, `~`, `-t`, `""` unchanged. `escape_dir("/p#S;")` is `/p##S\;`.
     `respawn_args` applies `escape_dir` to the directory and `escape_arg` to every TUI element: with `ProcessEnv::Isolated([("K", "v;")])` and a
     directory `/p#S`, the vector holds `-c`, `/p##S` and the element `K=v\;`.
11c. **One tmux server, never `$TMUX`'s (W-4).** `tmux_command` with `TmuxSocket::Path(p)` has program `tmux_bin` and
     arguments `-S p` then the call's; with `Name(n)`, `-L n` then the call's; with `Default`, the call's alone. For all
     three, `Command::get_envs()` holds `("TMUX", None)` and `("TMUX_PANE", None)` (both removed).
11d. **A 200 is not enough (W-6).** With the stub answering `200` `text/html` (`<!doctype html>...`) for `GET /session/<id>`:
     `abort` is `unavailable` and the stub received no `POST .../abort`; `attach_tui` is `unavailable` and the `tui_session`
     resolver is never called. A `GET /session/<id>` whose object's `id` differs is `unavailable` the same way. With a valid
     `GET /session/<id>` and `POST /session/<id>/abort` answering `200` HTML, `abort` is `unavailable`. `list_sessions`
     against a `200` HTML `GET /session` is `unavailable`. Each such message is one line, names the route and the status,
     and is at most 200 bytes. `abort` of the id `ses x/?` sends a request line whose path is
     `/session/ses%20x%2F%3F`.
11e. **Top-level sessions only (W-7).** With `GET /session` = `[{"id":"ses_a"},{"id":"ses_b","parentID":"ses_a"},
     {"id":"ses_c","parentID":null}]`, `list_sessions` is `["ses_a", "ses_c"]`.
11f. **The TUI argv.** `tui_argv("/bin/oc", &Inherit, 48123, "/p", "ses_1")` is `["env", "-u",
     "OPENCODE_DISABLE_TERMINAL_TITLE", "/bin/oc", "attach", "http://127.0.0.1:48123", "--dir", "/p", "--session",
     "ses_1"]`; under `Isolated([("A", "b")])` it is `["env", "-i", "A=b", "/bin/oc", "attach", ...]` with the same tail.
     `attach_port` of each, joined as tmux would print it, is `Some(48123)`.

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
  `w9:p2`, with `tui_session` mapping them to those session names and anything else to `pane-not-found`;
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
19a. **A prefix never reaches another pane (B-2).** The rig as above; in this test only, a second `OpenCodeHarness` is built
    from the same config but with a `tui_session` that also maps `w9:p3` to the session name `demo` (a prefix of
    `demo-c1r1`; no session `demo` exists). Attach pane 0 to A and read pane 0's `#{pane_pid}` (raw tmux on the private
    socket). Then `shown_session(w9:p3)` is `Ok(None)`, `select_session(w9:p3, B)` and `attach_tui(w9:p3, p0, B)` are
    `unavailable`, and afterwards pane 0 still shows A with the same `#{pane_pid}`.

Gates for the whole change:

20. `cargo test --workspace` passes with the real tests skipped; `HOLLER_TEST_OPENCODE=1 cargo test -p
    holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1` passes on a machine with OpenCode 1.18.x
    and tmux 3.2 or later (record the versions and the run's output summary in `handoff-T-green.md`).
21. `cargo clippy --workspace --all-targets -- -D warnings` is clean; every new `.rs` file passes
    `rustfmt --check --edition 2021` (epic ruling 4); no new `unsafe` (`git diff origin/main | grep -c unsafe` adds 0);
    `bash scripts/lint.sh` passes (no file at 900 lines; any `features = [...]` carries its consumer comment);
    `cargo machete` reports nothing for the crate.
22. `git diff --name-only origin/main` lists only `crates/holler-adapter-opencode/**`, `CHANGELOG.md`, `Cargo.lock`,
    `docs/adr/ADR-0021.md`, `crates/holler-pane/src/ports.rs`, `crates/holler-pane/src/lib.rs` and this run's
    `docs/handoffs/` files (B-1, W-9). The two `holler-pane` files change doc comments only: every added or removed line of
    `git diff origin/main -- crates/holler-pane` is a `//!` or `///` line. `Cargo.lock` changes only
    `holler-adapter-opencode`'s dependency list. No change to the test kit or the workspace `Cargo.toml`.
23. `CHANGELOG.md` `[Unreleased]` has one entry under "Enhancements" that links #642 and epic #633 and says what the adapter
    does and that the real-OpenCode tests are opt-in.
24. Safety: `grep -rn "4700[0-9]\|--continue" crates/holler-adapter-opencode` finds nothing outside comments that forbid them;
    no test reads a real HOME, the default tmux server or a port outside 48100-48199 (A and S check the rig code).
25. The PR body says how each of the fake's three `ASSUMPTION (#642 to confirm)` comments stands after this run: the shown
    session is read through **tmux** `#{pane_title}`, not Herdr, so the Herdr half of that assumption no longer applies;
    cross-directory `select-session` and aborting a model turn remain **unverified** (no test exercises them; both need
    something this run must not do or have). It also says `HarnessPort` is confirmed unchanged by the spike and that
    ADR-0021 now records it as built (AC 26), and it lists the two adapter-vs-`FakeHarness` divergences of decision 15 with
    the test-kit follow-up that will align them.
26. **ADR-0021 and the port docs say what was built (B-1).** In `docs/adr/ADR-0021.md`: the "Deferred to named stories" line
    no longer lists `HarnessPort` as deferred (it reads, for `HarnessPort`: confirmed unchanged by #635, built by #642, see
    section 2), and section 2 holds a "`HarnessPort` as built (#642)" note with the facts listed under "ADR-0021 and
    `holler-pane` doc edits" below. `HarnessPort`'s doc comment (`ports.rs:170-171`) and the crate docs (`lib.rs:32-33`,
    `ports.rs:13-15`) no longer call `HarnessPort` provisional; `HerdrPort`'s wording is left as it is (#640 owns it).

## Files

Production (F): `crates/holler-adapter-opencode/Cargo.toml`, `src/lib.rs`, `src/http.rs`, `src/tui.rs`, `src/exec.rs`
(private, decision 11); `CHANGELOG.md`.
Docs (F, B-1): `docs/adr/ADR-0021.md`; doc comments only in `crates/holler-pane/src/ports.rs` and `crates/holler-pane/src/lib.rs`.
Mechanical (W-9): `Cargo.lock` (the crate's new dependencies).
Tests (T): `crates/holler-adapter-opencode/tests/hermetic_test.rs`, `tests/real_opencode_test.rs`.

**Blast radius:** `crates/holler-adapter-opencode/**` (the issue's), plus `CHANGELOG.md`, `Cargo.lock`, `docs/adr/ADR-0021.md`
and the doc comments of `crates/holler-pane/src/{ports.rs,lib.rs}` (no code, so no compiled change in `holler-pane`). Nothing
depends on `holler-adapter-opencode` yet, so no existing test can break. The ADR lines and the `holler-pane` doc lines
overlap with #640 part 3's planned edits for `HerdrPort` (640-brief.md:35, 589-594); whichever merges second rebases, and
each edits only its own port's wording.

### ADR-0021 and `holler-pane` doc edits (F, B-1)

Precedent: #639 (2a6f349), #676 (3f9fbf2) and #692 (316b8e3) each amended ADR-0021 in the change that settled a contract
point; the #640 brief does the same for `HerdrPort`. About 20-30 lines in all:

- **ADR-0021 "Deferred to named stories"** (line 530): the item becomes `HerdrPort` alone (#636, then #640); `HarnessPort` is
  recorded as confirmed unchanged by spike #635 and built by #642 (section 2).
- **ADR-0021 section 2**, the "provisional" paragraph (lines 100-103): `HarnessPort` is no longer provisional (spike #635
  confirmed it; #642 implements it). The `HerdrPort` half is left as it is for #640.
- **ADR-0021 section 2, a short note "`HarnessPort` as built (#642)"** stating, one sentence each:
  1. SHOWN is read from the TUI's terminal title through tmux (`#{pane_title}` of the pane's tmux session, addressed by its
     exact name), not through Herdr; the session of record is titled with its own id so the title maps back to it
     (decision 1).
  2. `serve` never adopts a running server: a port that already answers is `unavailable` (unlike `FakeHarness`, which
     re-serves its own pane's port). `HarnessPort` has no stop; the host adapter (#641) stops the server by the recorded pid,
     its process group and descendants (decision 2).
  3. `shown_session` is `None` whenever it cannot tell: the home screen, a deleted session, a title that is not a whole id,
     no TUI; it never guesses (decision 3).
  4. One TUI per server is the caller's precondition (`select-session` reaches every TUI of a server); the registry gives
     each pane its own port (decision 6).
  5. `list_sessions` lists top-level sessions only; child (subagent) sessions are left out (decision 13).
  6. The adapter's two lookups (a pane's project directory, a pane's tmux session name) are supplied by wiring (#649) and must
     answer for a pane being launched, before its record exists (decision 14); an unknown `PaneId` is the lookup's own error
     (`pane-not-found`), where `FakeHarness` answers `None` or `unavailable` (decision 15).
- **`crates/holler-pane/src/ports.rs:170-171`** (`HarnessPort`'s doc): drop "**Provisional** until spike #635 reports"; say
  the spike confirmed it and point at ADR-0021 section 2's note. **`ports.rs:13-15`** and **`lib.rs:32-33`**: the sentence
  keeps `HerdrPort` provisional as now and says `HarnessPort` is final (built by #642). Doc comments only.

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
    another adapter crate (ADR-0021 §5, line 182). The overlap with #641's `tmux.rs` is the socket flags, the `TMUX`/`TMUX_PANE`
    removal, the exact target and the escape function of its Decision 13; this crate mirrors them **by the same rules and
    names** (`TmuxSocket`'s variants, `=<session>:`, the trailing-`;` escape, `#` doubling for `-c`) so the consolidation
    follow-up is a move. Its own calls are three subcommands (`display-message`, `set-option`, `respawn-pane`).
  - `exec.rs` (private): **new**, justified the same way (W-5): the timed subprocess runner, the tmux stderr classification
    and the `kill` call have one home in this crate, shaped like #641's `exec.rs` (one deadline per method, `Timeout { op }`
    when it passes, spawn `NotFound` -> `unavailable`). #663's `run_probe` will be a third runner; the follow-up covers all
    three.
  - The tests' stub HTTP server: **new**, justified: the existing fake OpenCode servers are private to other crates' test
    trees (`holler-body/tests/http_attach_driver_test/fake_server.rs`, `holler-cli/tests/attach_cli_test/fake_server.rs`),
    async on `tokio`, and cannot be imported; this one must also model a frozen server.
  - `op` strings: the adapter defines its own `"harness.<method>"` constants equal to `HarnessOp::as_str`; production code
    cannot depend on the test kit. AC 7, 8 and 15 pin the equality.

### Forward-compat (the consumers of this API)

| Consumer | Needs | Satisfied |
|---|---|---|
| #644 launch/relaunch | `serve`, `create_session`, `attach_tui`, `shown_session` within I5; a pid to record; a way to stop the old server before a relaunch | yes, with two preconditions on #644: `serve` refuses a port in use, so launch checks `health` and the recorded pid first and relaunch stops the old server first, through the host adapter (#641) by the recorded pid (decision 2; follow-up); and the resolvers must answer for the pane being launched before its record exists, so #644 passes the session name (and directory) explicitly, from its plan, rather than relying on a registry lookup (decision 14) |
| #645 switch/reset | `select_session` confirmed by observation; `create_session`; `abort` | yes |
| #647 reconcile/doctor | `health` as a timed probe (`false`, never a hang); `shown_session` = `None` for home or a deleted session; `list_sessions` without subagent sessions | yes; `list_sessions` is top-level only, so a working agent's child sessions are not strays (decision 13) |
| #649 wiring | construct the adapter: `workdir` (a pane's project directory, `Pane.host.cwd`), `tui_session` (a `PaneId`'s tmux session name, `Pane.host.tmux` looked up by `herdr.pane_id`), and one `TmuxSocket` value shared with the host adapter | yes, through the two `Resolver`s and `TmuxSocket` (decision 10); precondition: both resolvers also answer for a pane being launched, before its record exists, from the verb's plan (decision 14) |
| #638 test kit (follow-up) | `FakeHarness` agreeing with the adapter | no, two divergences recorded (decision 15); the test-kit follow-up aligns them |
| #641 host adapter (follow-up) | to stop the harness server `serve` started | not yet: the follow-up gives `stop_owned` that job (decision 2); #641's brief is unchanged by this amendment |
| #667 profile apply on scratch OpenCode | a scratch OpenCode environment | partly: the rig is test code here; #667 lifts it or asks for it to be shared (not a conflict) |

## Decisions made in this brief (MO)

1. **The TUI is reached through tmux, not Herdr.** A pane's TUI runs in its tmux session (`Pane.name` = `host.tmux`,
   ADR-0021:37); the spike read the title through tmux `#{pane_title}`, and Herdr's `PaneInfo.title` is report-metadata, not
   the terminal title (`herdr-api-spike.md:137, 142`). The `PaneId` -> tmux **session name** mapping is injected
   (`tui_session`, typed `PaneName`, so only a valid name can reach a target); the adapter builds every tmux target itself
   (decision 9), so this crate needs no registry access and holler-cli never writes tmux syntax. This resolves the Herdr half
   of the fake's third assumption.
2. **`serve` never adopts a running server.** A port that already answers is `unavailable`. The fake is more lenient (same
   pane on a running port returns the old pid), but the adapter cannot tell whose server answers or its pid without a
   process-table search, which #641's rules forbid by spirit (no matching processes by name). The conformance suite does not
   test idempotence. #644 must check `health` and the recorded pid before calling `serve`.
   **Who stops the server `serve` starts (W-1; MO decision):** the host adapter (#641), by the recorded pid, killing the
   server's process group **and its descendants** (OpenCode runs shell commands in sessions of their own, spike 175-177,
   244, so a group kill alone misses them). `serve` already returns a pid that is the server's process-group id. `HarnessPort`
   gains no stop method. This is a forward-compat fact for #641 and #644; the orchestrator files a follow-up issue for it
   (no number yet), and #641's brief is not changed by this amendment. Until it lands, a relaunch onto a running server
   fails at `serve` with "port N is in use", loudly, never by adopting it.
3. **`shown_session` answers `None` when it cannot tell** (home, a deleted session, a default or non-id title), per the
   trait's "if it can tell". It never guesses an id from a title. Reconcile treats SHOWN = none as a mismatch to fix at once
   (spike 222), which is right for every case folded into `None`.
4. **The port of a pane's TUI is observed from tmux** (`#{pane_start_command}` of a live pane), not remembered: each CLI
   run is a fresh process (epic ruling 1). O's probe on tmux 3.7c (Evidence) shows `pane_start_command` follows a
   `respawn-pane`; if T-green finds otherwise on another tmux, F may read `#{pane_pid}`'s command line instead;
   `attach_port` takes a command line either way.
5. **`attach_tui` sets `remain-on-exit on`** on the pane before respawning it, so an exiting TUI stays observable (dead pane,
   exit status) instead of taking the tmux session with it.
6. **One TUI per server** is a precondition the adapter cannot check (`select-session` broadcasts); the registry gives each
   pane its own port, and case 15 shows a switch reaches only its pane under that rule.
7. **ADR-0021 is updated in this change (B-1; replaces "no ADR change").** The trait's signatures are confirmed as merged,
   but ADR-0021 defers `HarnessPort`'s final form to this story by name, and decisions 1, 2, 3, 6, 13, 14 and 15 are facts the
   verb stories rely on. F closes the deferred item, adds the "`HarnessPort` as built (#642)" note and drops "provisional"
   from `HarnessPort`'s doc comments, as listed under "ADR-0021 and `holler-pane` doc edits".
8. **No operator step is needed.** Everything runs against scratch OpenCode and a private tmux server.
9. **Exact tmux targets, escaped values (B-2).** tmux prefix-matches a bare target (`display-message -t demo` answered for
   `demo-c1r1`, Evidence; #641's `has-session` probe). So the resolver returns a session name, and `tui.rs` alone turns it
   into `=<session>:` (`exact_target`) for every call. A missing exact target is not an error for `display-message` (exit 0,
   empty fields), so a query counts only when its `#{session_name}` field equals the name. Values the adapter did not author
   are escaped by #641's Decision 13 rule: a final `;` gets a `\` (an element ending in `;` ends the tmux command even after
   `--`; Evidence), and the `-c` directory has every `#` doubled first (`-c` is format-expanded; `#(...)` would run a shell
   command, #641's E4). Names need no escape: a `PaneName` is `[a-z0-9-]` (ADR 0005).
10. **One tmux server, chosen once (W-4, W-8).** `TmuxSocket { Default, Name, Path }` mirrors #641's, so #649 builds one value
    for both adapters. Every tmux child has `TMUX` and `TMUX_PANE` removed (`tmux_command`), so `Default` means tmux's own
    default socket, never the server an inherited `$TMUX` names (Evidence). `ProcessEnv::Isolated` values are on the TUI's
    command line and in `#{pane_start_command}`; they must never hold a secret, and no message echoes them.
11. **One private home for subprocess plumbing (W-5).** `src/exec.rs`: the timed runner, the tmux stderr classification and
    the `kill` call, shaped like #641's `exec.rs`. Depending on #641's crate is ruled out (ADR-0021 §5), so the copy is
    deliberate and temporary: a consolidation follow-up once #641 and #642 have both merged (Follow-ups).
12. **Reply shapes are checked (W-6).** OpenCode's web app answers unknown routes with `200` HTML, so every step requires the
    JSON it relies on (Behaviour); anything else is `unavailable` with a one-line message naming route and status.
13. **`list_sessions` is top-level only (W-7).** Elements with a string `parentID` (subagent sessions; `holler-body`'s
    `connection.rs:70-77, 305-325`, issue #382) are left out. The port's `Vec<String>` cannot carry parentage, and reconcile
    (#647) treats every listed session other than the session of record as a stray (ADR-0021:478), so listing children would
    flag a working agent's subagents.
14. **Resolver precondition (W-2).** #644 writes the pane record once, after the act (ADR-0021 §8 step 4, line 292), so a
    registry-backed resolver cannot answer for a new pane during launch. Each resolver must answer for a pane being launched
    before its record exists; #644 passes the session name (and directory) explicitly from its plan, and #649 builds
    resolvers that consult it. Recorded in the `Resolver` doc, the crate docs and the ADR note; the API does not change.
15. **Two divergences from `FakeHarness`, recorded, not hidden (W-3).** (a) `serve` on a port already serving the same pane:
    the fake returns that server's pid, the adapter answers `unavailable` (decision 2). (b) A `PaneId` the resolver does not
    know: the fake gives `shown_session = Ok(None)` and `select_session = unavailable` (exit 1); the adapter returns the
    resolver's error, `pane-not-found`, a refusal (exit 3), from both. Neither is pinned by the suite. Both go in the crate
    docs, the ADR note and the PR body (AC 25); the test-kit follow-up aligns the fake.

## Out of scope

- Any change to `holler-pane` beyond the doc comments above, the test kit (including its `ASSUMPTION` comments and
  `FakeHarness`'s two divergences: a follow-up), the workspace manifest, `holler-cli` (wiring is #649), #641's crate or brief,
  or `scripts/spikes/*`.
- Password-protected servers (`OPENCODE_SERVER_PASSWORD`, `attach -p/-u`; unverified in the spike), workspaces
  (`?workspace=`), cross-directory `select-session`, aborting a model turn.
- Stopping a server: owned by the host adapter (#641) by the recorded pid, its process group and descendants (decision 2;
  a follow-up, not this story). Restarting a wedged one (#644/#647), the restart, boot-race and load probes (stay in
  `scripts/spikes/`).
- Reading the TUI's screen as a fallback for the title.

## Follow-ups (not this story; the orchestrator files them)

- **#641 / #644: stop the harness server.** `stop_owned` (or #644's relaunch through it) stops the pane's harness server by the
  pid `serve` returned, killing its process group and its descendants (decision 2). How #641 learns the pid (for example a
  wiring resolver over `Pane.harness.pid`) is for #641 and #649.
- **Test kit (#638's files): `FakeHarness` parity.** Align `FakeHarness::serve` with "never adopt", settle the unknown-pane
  answer (decision 15), and retire the three `ASSUMPTION (#642 to confirm)` comments with this run's outcome (AC 25).
- **Consolidate the subprocess and tmux plumbing** of this crate's `exec.rs`/`tui.rs` and #641's `exec.rs`/`tmux.rs` (and
  #663's runner) once #641 and #642 have both merged.

## Test plan

**RED first (T).** Both test files compile against the API above with F's code absent, so today they fail to compile
(`holler_adapter_opencode::{OpenCodeHarness, OpenCodeConfig, http, tui}` do not exist). T may add the minimum public stubs
F would otherwise write only if the pipeline requires a runtime RED; if so, every stub method answers
`PaneError::NotImplemented` (and every pure `tui.rs` function returns an empty or `None` value) and AC 1-11f fail on
assertions. Record in `handoff-T-red.md` which ACs fail and why. T runs the
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
6. **macOS CI.** Only the hermetic file runs there; it must not call tmux, `kill`, `/proc` or OpenCode (AC 11a-11c build
   `Command`s and argv vectors without running them).
7. **A refactor that bypasses the builders** (a bare `-t`, an unescaped element, a tmux `Command` not made by
   `tmux_command`) reopens B-2 or W-4. AC 11a-11c pin the builders; A-dup and S check that every tmux spawn goes through
   `tmux_command` and every `-t` through `exact_target`.
8. **Merge order with #640 part 3.** Both edit ADR-0021 section 2 and the same `holler-pane` doc lines for their own port;
   the second to merge rebases and keeps the other's wording.
