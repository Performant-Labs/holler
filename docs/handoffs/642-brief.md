# Brief: #642b the OpenCode adapter, part 2: the TUI side of `HarnessPort`, the real-OpenCode tests, the ADR-0021 record

Repo: Performant-Labs/holler. Issue: #642 (epic #633, wave 3). Rigor: second-opinion. UI surface: no. Kind: feature (adapter).

**THIS RUN IS 642b ONLY.** 642a (the server side: `serve`, `health`, `create_session`, `list_sessions`, `abort`) merged
as PR #705 (`dc300ab`). 642b builds the three TUI methods (`attach_tui`, `select_session`, `shown_session`), runs the
conformance suite and the opt-in tests against a real `opencode serve`, and records `HarnessPort` as built in ADR-0021 and
the `holler-pane` doc comments. **642b is not the last part of #642:** the issue's 2026-10-09 amendment (apply the pane's
`opencode_agent`) is placed in a **642c** that starts after #700 merges (Decision 16). Nothing in this run reads, names or
applies an agent.

**PR wording (binding on the PR title, the PR body, every commit message and the CHANGELOG):** the PR body says
`Part of #642` and never `Closes #642`. The script opens the PR with `Closes #642.` (Evidence E14), so the run's agent edits
the body with `gh pr edit` before merging. Never put a GitHub closing keyword (`close`, `closes`, `closed`, `fix`, `fixes`,
`fixed`, `resolve`, `resolves`, `resolved`) in front of any `#N` anywhere, and **never write a negated form such as
"does not close #642" or "doesn't fix #N"**: GitHub ignores the negation and closes the issue. Say "Part of #642; 642c
follows" instead. Title: a Conventional subject such as
`feat(adapter-opencode): the TUI side of HarnessPort, attach, switch and the shown session (#642 part 2 of 3)`.

**Branch:** `issue-642-implementation`, a fresh worktree from `origin/main` at `dc300ab`. **Design (D):** N/A (no UI).
**Source of truth:** the issue; ADR-0021; `docs/research/opencode-pane-spike.md`; the merged test kit; and the 642a brief
(`git show d6393b2:docs/handoffs/642-brief.md`, 1,106 lines, removed from `main` by the PR step's cleanup), whose 642b
material is carried here verbatim where marked "Carried from 642a". Ids such as B-2, W-4 or NV-7 inside carried text name the
642a review findings that produced them; the facts they rest on are quoted again in Evidence below.

**Dependencies (checked 2026-10-09):**

| Needed | State | For 642b |
|---|---|---|
| #637 skeleton, #638 test kit (incl. `FakeHarness`, `run_harness_conformance`), #635 spike | merged | used as is |
| 642a (PR #705) | merged, `dc300ab` | extended |
| #700 `Pane.opencode_agent`, `AgentKey`, `--agent` | **OPEN, no PR**; `git grep opencode_agent origin/main -- crates` finds nothing | **not needed**: 642c's (Decision 16) |
| #641 host adapter (PR #706) | OPEN, `CONFLICTING`; code on `origin/issue-641-implementation` (`f2ea297`) | mirrored, never depended on (ADR-0021 §5) |
| #640 part 3 (Herdr: ADR-0021 and `holler-pane` doc edits for `HerdrPort`) | not started | merge-order risk only (Risk 8) |

## Size check

One crate plus three doc-only files. F edits five code files (one new) and three doc files; T writes four test files (three
new) and one committed fixture. Estimates at 642a's density:

| File | Who | Lines now -> est. |
|---|---|---|
| `crates/holler-adapter-opencode/src/tui.rs` (builders, escape, `parse_title`, `attach_port`, `parse_query`) | F | 33 -> ~260 |
| `crates/holler-adapter-opencode/src/attach.rs` (**new, private**: the three method bodies, the attach poll) | F | 0 -> ~300 |
| `crates/holler-adapter-opencode/src/exec.rs` (a capturing runner; tmux stderr classification) | F | 77 -> ~190 |
| `crates/holler-adapter-opencode/src/lib.rs` (three one-line delegations, three `op` constants, crate docs) | F | 483 -> ~500 |
| `CHANGELOG.md` (one sentence removed from the part-1 entry, one part-2 entry) | F | ~10 |
| `docs/adr/ADR-0021.md` (§2 paragraph, §2 note, "Deferred" item) | F | ~25 |
| `crates/holler-pane/src/ports.rs`, `crates/holler-pane/src/lib.rs` (doc comments only) | F | ~8 |
| `crates/holler-adapter-opencode/tests/tui_test.rs` (**new**, pure: no stub, no process) | T | ~280 |
| `crates/holler-adapter-opencode/tests/attach_test.rs` (**new**: stub + fake tmux fixture) | T | ~420 |
| `crates/holler-adapter-opencode/tests/fixtures/fake-tmux` (**new**, committed mode 100755, POSIX `sh`) | T | ~30 |
| `crates/holler-adapter-opencode/tests/real_opencode_test.rs` (**new**, opt-in) + `tests/real_opencode/rig.rs` (`#[path]`) | T | ~450 + ~300 |
| `crates/holler-adapter-opencode/tests/hermetic_test.rs` (refused-port rule, comment repoints) | T | 775 -> ~800 |
| **Total new or changed** | | **~2,100** |

No file may reach 900 lines (`scripts/lint.sh` check 4, E12). `lib.rs` stays under 600 (the method bodies live in
`attach.rs`, Decision 17). `hermetic_test.rs` gains no new test cases (only the refused-port helper's use), so it stays under
850. If `attach_test.rs` nears 700, move its fixture helpers to `tests/support/fake_tmux.rs` (`#[path]` module).

## Problem

After 642a, `OpenCodeHarness` answers `not-implemented` for `attach_tui`, `select_session` and `shown_session` (E1). The pane
verbs (#644 launch, #645 switch and reset, #647 reconcile) need them: attach the pane's TUI to exactly the session of record,
switch it without typing (I4), and observe which session it shows (I3: plan, act, observe, record). Raw OpenCode
acknowledges a switch no TUI saw, broadcasts a switch to every TUI of a server, has no API that reports the shown session, and
fails `attach` of an unknown id only by exiting (E6). The TUI is reached through the pane's tmux session, where tmux
prefix-matches bare targets and does not fail a format query on a missing exact target (E7). The adapter must hide all of
that behind the contract the merged conformance suite pins (E4), and ADR-0021 still calls `HarnessPort` provisional and
deferred to this story (E9).

## Evidence (verbatim, as of `dc300ab` unless marked)

**E1. The crate today.** The three methods, the config they read, and the module docs that 642b changes:
```
crates/holler-adapter-opencode/src/lib.rs:7-11
//! **Part 1 of #642: the server side.** `serve`, `health`, `create_session`,
//! `list_sessions` and `abort` are built. `attach_tui`, `select_session` and
//! `shown_session` answer `not-implemented`, the skeleton's answer for a method whose story
//! has not landed, until part 2 adds the TUI, which runs in the pane's tmux session. Until
//! then [`tui`] holds only the tmux configuration. Nothing wires the adapter in yet (#649).
```
```
crates/holler-adapter-opencode/src/lib.rs:37-37
//! What the adapter decides (the decisions of `docs/handoffs/642-brief.md`):
```
```
crates/holler-adapter-opencode/src/lib.rs:149-167
/// How the adapter reaches OpenCode and tmux.
#[derive(Clone)]
pub struct OpenCodeConfig {
    /// The `opencode` binary; production: `"opencode"`, found on `PATH`.
    pub opencode_bin: PathBuf,
    /// Appended after `serve --port P --hostname 127.0.0.1`; the tests pass `["--pure"]`.
    pub serve_args: Vec<String>,
    /// The environment of the server (and, in part 2, of the TUI's `attach`).
    pub env: ProcessEnv,
    /// The tmux server the panes' TUIs run on (part 2).
    pub tmux: TmuxConfig,
    /// The project directory a pane's server runs in (#649: from `Pane.host.cwd`).
    pub workdir: Resolver<PaneName, PathBuf>,
    /// The tmux session name a pane's TUI runs in, never a tmux target (#649:
    /// `Pane.host.tmux`, which equals `Pane.name`, ADR-0021 line 37). Part 2.
    pub tui_session: Resolver<PaneId, PaneName>,
    /// The bounds.
    pub timeouts: Timeouts,
}
```
```
crates/holler-adapter-opencode/src/lib.rs:180-189
    /// A call of the method `op` to the server on `port`, its deadline starting now.
    fn call(&self, port: u16, op: &'static str) -> Call {
        let timeouts = &self.config.timeouts;
        Call {
            port,
            op,
            deadline: deadline_after(timeouts.call),
            request: timeouts.request,
        }
    }
```
```
crates/holler-adapter-opencode/src/lib.rs:234-245
    fn attach_tui(&self, _pane: &PaneId, _port: u16, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented) // part 2 of #642
    }

    fn select_session(&self, _pane: &PaneId, _session: &str) -> Result<(), PaneError> {
        Err(PaneError::NotImplemented) // part 2 of #642
    }

    fn shown_session(&self, _pane: &PaneId) -> Result<Option<String>, PaneError> {
        Err(PaneError::NotImplemented) // part 2 of #642
    }
}
```
```
crates/holler-adapter-opencode/src/lib.rs:372-387
/// The existence check `GET /session/<id>`: `Ok` when the server holds that session. A 404,
/// or a 400 (a malformed id), is `session-not-found`. Any other answer, a 200 that is not
/// that session included, is `unavailable`.
fn known(call: &Call, path: &str, id: &str) -> Result<(), PaneError> {
    let reply = call.send(GET_SESSION, path, None)?;
    if matches!(reply.status, 400 | 404) {
        return Err(PaneError::SessionNotFound {
            what: id.to_owned(),
        });
    }
    if string_at(json_of(&reply).as_ref(), "id") == Some(id) {
        Ok(())
    } else {
        Err(call.unexpected(GET_SESSION, &reply, "that session"))
    }
}
```
```
crates/holler-adapter-opencode/src/tui.rs:1-7
//! The tmux side of the adapter: which tmux server a pane's TUI runs on.
//!
//! In part 1 of #642 this module holds only [`TmuxSocket`] and [`TmuxConfig`], the type of
//! [`crate::OpenCodeConfig`]'s `tmux` field. Part 2 adds the tmux calls of `attach_tui`,
//! `select_session` and `shown_session` here, with their builders and parsers. This adapter
//! never starts a tmux server (it only addresses sessions that exist), so a tmux config
//! file has no effect and there is no `-f`.
```
```
crates/holler-adapter-opencode/src/exec.rs:1-8
//! Running a helper program to its end within a bound (private; decision 11 of the brief).
//!
//! In part 1 of #642 the one helper is `kill`: when `serve`'s deadline passes before the
//! server it started answers, [`kill_group`] sends SIGKILL to that server's process group
//! through the `kill` program, not a raw system call. Part 2 adds what its tmux calls read
//! (their output, and the classification of tmux's stderr) with those calls.
//! The runner is shaped like the host adapter's (#641), so that one runner for the adapters
//! (#696) is a move.
```
```
crates/holler-adapter-opencode/src/exec.rs:45-57
/// Run `command`, with stdin, stdout and stderr null, until it exits or `bound` passes. A
/// program that cannot be started is `unavailable`, naming it. Past `bound` the child is
/// killed and reaped, and the answer is `timeout` with `op`.
fn run(mut command: Command, op: &str, bound: Duration) -> Result<ExitStatus, PaneError> {
    let program = Path::new(command.get_program()).display().to_string();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| PaneError::Unavailable {
            what: crate::one_line(&format!("cannot run {program}: {error}")),
        })?;
```
`wc -l`: `lib.rs` 483, `http.rs` 360, `server.rs` 189, `exec.rs` 77, `tui.rs` 33; `tests/hermetic_test.rs` 775,
`tests/support/stub.rs` 243. Private items of the crate root (`Call`, `known`, `session_path`, `json_of`, `one_line`,
`excerpt`, `deadline_after`, `budget`, `OpenCodeHarness::call`) are visible to a child module such as `attach.rs` without a
visibility change; `server.rs` already calls `crate::one_line`.

**E2. The port** (merged, frozen; no signature changes):
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

**E3. The closed codes** (no new code; `error.rs` is #637's):
```
crates/holler-pane/src/error.rs:404-406
    /// `not-implemented`: the skeleton's answer for a verb or method whose story has
    /// not landed. (#637 defines it; every stub returns it.)
    NotImplemented,
```
```
crates/holler-pane/src/error.rs:454-467
    /// `timeout`: an operation did not return within the bound of I5 (default 10 s);
    /// `op` names it. (#638-#642.)
    Timeout { op: String },
    /// `pane-not-found`: no pane of that name; `what` is the name. (#638-#642.)
    PaneNotFound { what: String },
    /// `session-not-found`: no harness session of that id; `what` is the id.
    /// (#638-#642.)
    SessionNotFound { what: String },
    /// `store-corrupt`: a stored file or record cannot be read back; the store fails
    /// closed rather than dropping state. `what` names the store. (#639/#661.)
    StoreCorrupt { what: String },
    /// `unavailable`: something the verb needs cannot be reached: the hub, the Herdr
    /// socket, a harness. `what` names it. (#638-#642.)
    Unavailable { what: String },
```
```
docs/adr/ADR-0021.md:392-399
  | `pane-not-found` | Refusal (3) | The named pane does not exist. |
  | `session-not-found` | Refusal (3) | The named harness session does not exist. |
  | `generation-conflict` | Failure (1) | A race between writers; running the verb again can succeed. |
  | `profile-conflict` | Failure (1) | The profile moved after the live change (the I8 write order); a race. |
  | `timeout` | Failure (1) | The I5 bound ran out while doing the work. |
  | `unavailable` | Failure (1) | The hub, the Herdr socket or a harness cannot be reached (also a garbled reply). |
  | `store-corrupt` | Failure (1) | Stored state cannot be read back; the store fails closed. |
  | `not-implemented` | Failure (1) | The verb cannot do the work yet; every stub exits 1. |
```

**E4. The suite the adapter must pass** (merged; binding):
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
```
```
crates/holler-pane-testkit/src/conformance/harness.rs:50-59
/// What a harness under test gives the suite for one case: two free ports whose servers
/// share ONE data directory, as the live fleet's do, and two panes in which a TUI can be
/// attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessRig {
    /// Two ports with no server on them yet.
    pub ports: [u16; 2],
    /// Two panes with no TUI in them yet.
    pub panes: [PaneId; 2],
}
```
```
crates/holler-pane-testkit/src/conformance/harness.rs:125-145
/// `fresh` is called once per case. It returns the harness, the rig the case runs on and
/// a guard that the suite keeps alive for that case only; the harness is dropped before
/// its guard. No server may run on the rig's ports yet, and their servers must share one
/// data directory; no TUI may be in its panes yet. How each implementation runs the
/// suite (the test kit cannot name an adapter):
///
/// ```text
/// // the fake:
/// assert_eq!(
///     run_harness_conformance(|| (FakeHarness::new(), HarnessRig::sample(), ())),
///     Ok(())
/// );
/// // holler-adapter-opencode (#642): per case, a scratch dir with the spike's isolated
/// //   env and dead-end provider (opencode-pane-spike.md:16-35), two free ports from
/// //   48100-48199 whose servers share that one data directory, and two panes of a
/// //   private tmux server; the scratch dir and the process groups as the guard.
/// ```
pub fn run_harness_conformance<S, K, F>(mut fresh: F) -> Conformance
where
    S: HarnessPort,
    F: FnMut() -> (S, HarnessRig, K),
```
The 15 case ids (`conformance/harness.rs:78-115`): `health-of-unserved-port-is-false`, `serve-then-healthy`,
`fresh-server-has-no-sessions`, `create-session-is-listed`, `sessions-shared-across-servers`,
`calls-to-unserved-port-are-unavailable`, `abort-known-session`, `abort-unknown-is-session-not-found`,
`shown-without-tui-is-none`, `attach-shows-the-session`, `attach-unknown-is-session-not-found`,
`select-switches-the-shown-session`, `select-unknown-is-session-not-found`, `select-without-tui-fails`,
`select-reaches-only-its-pane`.

**E5. The fake's behaviour and its three assumptions** (the adapter matches the first; AC 25 reports on the rest):
```
crates/holler-pane-testkit/src/harness.rs:135-137
/// - `shown_session(pane)`: the session the TUI shows, or `None` on its home screen or
///   with no TUI. It does not reach the server: a TUI keeps its screen while its server
///   is frozen or dead (opencode-pane-spike.md:193-194).
```
```
crates/holler-pane-testkit/src/harness.rs:306-307
    // ASSUMPTION (#642 to confirm): abort stops a model turn as it stops a shell command;
    // the spike verified only a shell command (opencode-pane-spike.md:179, 267).
```
```
crates/holler-pane-testkit/src/harness.rs:332-335
    // ASSUMPTION (#642 to confirm): select-session switches a TUI whose --dir is another
    // project directory than the session's; the spike did not try it
    // (opencode-pane-spike.md:272-273). The fake has no project directories and always
    // switches.
```
```
crates/holler-pane-testkit/src/harness.rs:348-350
    // ASSUMPTION (#642 to confirm): the shown session is read from the pane's terminal
    // title; the spike read it through tmux, and whether Herdr exposes a pane's terminal
    // title is unverified (opencode-pane-spike.md:151-152, 269).
```

**E6. What real OpenCode does** (spike #635, OpenCode 1.18.35):
```
docs/research/opencode-pane-spike.md:93-93
opencode attach http://127.0.0.1:<port> --dir <project dir> --session <ses_id>     # -s <ses_id> also works
```
```
docs/research/opencode-pane-spike.md:100-101
- An **unknown** session id makes `attach` exit with status 1 and `Error: Session not found: ses_…` on stderr within
  about a second. It fails loudly; it does not fall back to some other session.
```
```
docs/research/opencode-pane-spike.md:113-113
**Call (verified).** `POST /tui/select-session {"sessionID":"ses_…"}` (optionally `?directory=<dir>`) → `200 true`.
```
```
docs/research/opencode-pane-spike.md:117-124
- An unknown id answers `404 NotFoundError` and the TUI stays where it was. A malformed id (not `ses…`) answers
  `400`.
- **Caveat 1: broadcast.** Two TUIs attached to one server *both* switched on one `select-session`. The call is
  not addressed to a TUI. **Read (bundled code):** the server publishes a `tui.session.select` event, and every TUI
  subscribed to that server whose workspace matches navigates.
- **Caveat 2: no acknowledgement.** On a server with **no** TUI attached, `select-session` still answers
  `200 true`. A `true` therefore proves nothing about the screen, and Holler must confirm the switch by observing
  the TUI (capability 4).
```
```
docs/research/opencode-pane-spike.md:137-151
**What works (verified).** The TUI sets its terminal title, and tmux reports it as `#{pane_title}`
(`tmux display-message -p -t <pane> '#{pane_title}'`):
- `OC | <session title>` while a session is shown. It follows `select-session` (about 110 ms) and a `PATCH` of the
  shown session's title (under 10 ms).
- `OpenCode` on the home screen and right after the shown session is deleted.
- A title longer than 40 characters is cut: `OC | hj-c1r1 holler pane title ses_ede8…` (verified).

**Rules (read in the bundled code, consistent with what was observed).**
`title = len > 40 ? title[0..37] + "…" : title`. The TUI shows `OpenCode` for the home route **and** for a
session whose title is still the default `New session - …`, so a default title cannot be told apart from "no
session". Setting `OPENCODE_DISABLE_TERMINAL_TITLE` turns the title off.

**So the caveat is a naming rule Holler must own:** give every session of record a unique, non-default title of
40 characters or fewer that maps back to its id. The simplest such title is the id itself: `ses_` plus 26 characters
is 30 characters. Never set `OPENCODE_DISABLE_TERMINAL_TITLE` in a pane. Whether **Herdr** exposes a pane's
```
```
docs/research/opencode-pane-spike.md:193-194
- **The TUI reports neither.** While its server was frozen or dead, the TUI stayed alive with its title unchanged
  and printed nothing about the connection. It cannot be asked whether its server is healthy.
```
```
docs/research/opencode-pane-spike.md:222-222
| TUI whose session is deleted | verified: it leaves the session within about 110 ms, shows the toast "The current session was deleted", goes to the home screen (title `OpenCode`), **stays running**, and creates no replacement session. `select-session` to a live session brings it back. **Inferred:** anything typed on that home screen would start a new session that no one recorded, so reconcile must treat SHOWN = none as a mismatch to fix at once |
```
```
docs/research/opencode-pane-spike.md:267-273
- Aborting a **model** turn, and whether a set title survives the first prompt (read in the code, not run): both
  need a model call.
- Whether Herdr exposes a pane's terminal title (#636); everything here read it through tmux.
- A wedge that lasts days, and memory growth over time (only the SIGSTOP stand-in and a short load were run).
- Basic-auth servers (`OPENCODE_SERVER_PASSWORD`) and `attach -p/-u`.
- `select-session` to a session from **another project directory** than the TUI's `--dir`. Workspaces
  (`?workspace=`) were not exercised; the bundled code filters `tui.session.select` by workspace.
```
`opencode attach --help` on the installed 1.18.35 (run by O, 2026-10-09):
```
opencode attach <url>
      --dir           directory to run in                                                   [string]
  -s, --session       session id to continue                                                [string]
```
The id alphabet (642a's NV-7, read from the 1.18.35 bundle): `ses_`, 12 hex digits, then 14 characters of
`[0-9A-Za-z]`, so every character after `ses_` is in `[0-9A-Za-z]` and there are 26 of them (spike 150-151 above).

**E7. tmux behaviour the TUI calls rely on**, re-probed by O on 2026-10-09 on tmux 3.7c, on a private server
(`env -u TMUX -u TMUX_PANE tmux -L hlr642bprobe<pid> -f /dev/null`, session `demo-c1r1` running `sleep 3600`), killed
afterwards; never the default socket. The title field, which is the machine's host name until a program sets it, is shown
as `<host>`.
```
display-message -p -t demo '#{session_name}'                -> "demo-c1r1", exit 0      PREFIX MATCH: a bare target is unsafe
display-message -p -t '=demo:' '[#{session_name}]'          -> "[]", exit 0             a MISSING exact target is not an error
set-option -p -t '=nope:' remain-on-exit on                 -> "no such pane: =nope:", exit 1
respawn-pane -k -t '=nope:' -- sleep 1                      -> "can't find session: nope", exit 1
set-option -p -t '=demo-c1r1:' remain-on-exit on; respawn-pane -k -t '=demo-c1r1:' -- sh -c 'exit 7'
display-message -p -t '=demo-c1r1:' '[#{session_name}|#{pane_dead}|#{pane_dead_status}|#{pane_start_command}]'
                                                            -> "[demo-c1r1|1|7|sh -c "exit 7"]", exit 0  a dead pane still answers
respawn-pane -k -t '=demo-c1r1:' -c /tmp -- env -u OPENCODE_DISABLE_TERMINAL_TITLE sleep 3601 'a b' 'x\;'
  (the same query)                                          -> "[demo-c1r1|1|1|env -u OPENCODE_DISABLE_TERMINAL_TITLE sleep 3601 "a b" "x;"]"
                                                               (sleep refused its arguments and exited 1; tmux re-quotes an
                                                               element with a space or `;`, and `x\;` reached it as `x;`)
display-message -p -t '=demo-c1r1:' "<the five fields of Decision 23, joined by TAB>"
                                                            -> "demo-c1r1\t0\t\t"sleep 3600"\t<host>\n", exit 0
the same on '=nope:'                                        -> "\t\t\t\t\n", exit 0     (four TABs: every field empty)
after kill-server: display-message -p -t '=demo-c1r1:' x    -> "no server running on <socket path>"
```
642a's probe (642a brief, Evidence, tmux 3.7c, same method) also recorded: `respawn-pane ... -c "<D>/p#S"` ran the program
in `$HOME` because `-c` is format-expanded, while `-c "<D>/p##S"` gave the cwd `<D>/p#S` exactly; an element `x;` after `--`
ended the tmux command (a following `rename-session pwned` ran), while `x\;` did not; and with no `-S`/`-L`, an inherited
`TMUX=/nonexistent,1,0` made tmux connect to `/nonexistent`.

**E8. #641's rules, which `tui.rs` mirrors by the same names** (unmerged: PR #706, `origin/issue-641-implementation` at
`f2ea297`, `crates/holler-adapter-host/src/tmux.rs`):
```
crates/holler-adapter-host/src/tmux.rs (at f2ea297):155-171
/// A value the adapter did not write, made safe for tmux's command line (Decision 13).
/// tmux ends a command at every argument that ends in `;`, after `--` too, and reads a
/// trailing `\;` as a literal `;` (`man tmux`, PARSING SYNTAX; E1, R1). So a value that
/// ends in `;` gets a `\` before that `;`, and nothing else changes: `x;` is `x\;`, `y\;`
/// is `y\\;`, and `;` is `\;`.
pub(crate) fn escape(value: &str) -> String {
    match value.strip_suffix(';') {
        Some(head) => format!("{head}\\;"),
        None => value.to_owned(),
    }
}

/// A start directory for `new-session -c` (Decision 13). tmux format-expands it, and
/// `#(...)` there runs a shell command (E3, E4), so every `#` is doubled (`##` is a
/// literal `#`) before [`escape`].
pub(crate) fn escape_cwd(cwd: &str) -> String {
    escape(&cwd.replace('#', "##"))
```
```
crates/holler-adapter-host/src/tmux.rs (at f2ea297):303-319
/// Classify a failed call by its stderr. tmux's messages are not localized.
pub(crate) fn classify(stderr: &str) -> Refusal {
    let no_socket = stderr.contains("error connecting to")
        && (stderr.contains("No such file or directory") || stderr.contains("Connection refused"));
    if stderr.contains("can't find window") || stderr.contains("no such window") {
        Refusal::WindowGone
    } else if no_socket
        || stderr.contains("can't find session")
        || stderr.contains("no server running")
    {
        Refusal::Missing
    } else if stderr.contains("duplicate session") {
        Refusal::Duplicate
    } else {
        Refusal::Other
    }
}
```

**E9. The sentences 642b edits in place** (ADR-0021 and `holler-pane` docs; Decision 7, AC 26):
```
docs/adr/ADR-0021.md:37-37
| `name` | `PaneName` | A newtype over `holler_proto::vocab::SessionName`, so a pane name obeys the [ADR 0005](ADR-0005.md) grammar. It is also the tmux session name. Names such as `hj-c1r2` are names, not positions. |
```
```
docs/adr/ADR-0021.md:100-103
`HerdrPort` and `HarnessPort`, and the data types they take and return, stay **provisional** until the spikes #636 (Herdr)
and #635 (OpenCode) report. If a spike contradicts them, the contract and this ADR are amended first, in their own PR (the
epic's amend-first rule). `holler-pane-testkit` (#638) fakes every port and ships a conformance suite each real
implementation runs against itself.
```
```
docs/adr/ADR-0021.md:528-537
## Deferred to named stories

- The operation id and the executor of long work: #644, after a contract amendment (section 12).
- The roster's `--json` shape beside the envelope: #648.
- How the hub's DRIVEN session follows `session_of_record`: #649 and #654.
- The exact file layouts of `panes.json` and `profiles.json`, the long-poll window, and the feed's retained window: #639 and
  #661.
- Which closed failure code a mismatch observed after `act` carries (I3 says the verb exits 1, and under section 9 an open
  code is a refusal, exit 3): #644 and #645.
- `HerdrPort` and `HarnessPort` in their final form: #636 and #635, then #640 and #642.
```
```
crates/holler-pane/src/ports.rs:12-15
//! **Frozen when #637 merges**, after which a change goes through the epic's
//! amend-first rule. [`HerdrPort`] and [`HarnessPort`], and the minimal data types
//! they take and return, stay provisional until the spikes #636 (Herdr) and #635
//! (OpenCode) report.
```
```
crates/holler-pane/src/lib.rs:31-33
//! **Frozen when #637 merges.** After that a change to a signature or a code goes
//! through the epic's amend-first rule. `HerdrPort` and `HarnessPort` stay
//! provisional until the spikes #636 and #635 report.
```
```
CHANGELOG.md:214-216
  is `unavailable`. Attaching, switching and reading a pane's TUI answer `not-implemented` until part 2, which also
  brings the opt-in tests against a real OpenCode. Nothing a user runs changes yet: #649 wires the adapter in
  ([#642](https://github.com/Performant-Labs/holler/issues/642)).
```
ADR-0021's invariants this story serves (unchanged):
```
docs/adr/ADR-0021.md:163-163
| I3 | Every verb is plan, act, observe, record; a mismatch fails loudly and records nothing. | Make the fake TUI show a different session after `act`: the verb exits 1 with a code, and the stored record and its generation are unchanged. |
```
```
docs/adr/ADR-0021.md:168-168
| I8 | An edit through `--profile` and the live change are one transaction. | Fail `act` in `edit_spec` on the fake: P's specs are equal to before and no pane record changed; succeed it: P holds the edit and the pane record names P. |
```

**E10. Platform lessons from sibling runs** (they set the test rules of AC 32):
```
git show 9ebedcc (fix(adapter-herdr), merged with #702), commit message:
macOS refuses SO_RCVTIMEO/SO_SNDTIMEO with EINVAL on a socket whose peer has
already closed it (inferred from PR #702's macOS CI job). The transport set a
read timeout before every read, so a reply ended by EOF without a newline came
back as `unavailable` instead of the reply. A timeout refused with
`InvalidInput` now lets the read or write go ahead, and its own result is the
answer; every other error keeps its mapping, and the deadline is unchanged.

The worker-lifetime test tolerates the same refusal from `set_nonblocking` and
`set_read_timeout` on a kept stream whose worker has closed it.
```
```
crates/holler-adapter-opencode/tests/support/stub.rs:148-157
/// A loopback port nothing listens on: bind one, then drop the listener.
pub fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a throwaway port");
    let port = listener
        .local_addr()
        .expect("a bound listener has an address")
        .port();
    drop(listener);
    port
}
```
`closed_port()` frees its port before the test uses it. Any other bind of `127.0.0.1:0`, in this test binary (every `Stub`
binds one) or in another process on the machine (CI's self-hosted runner and this machine run several pipelines at once), can
be handed that port, so a test that expects "refused" can meet a listener; 642a's tests showed this flake. The OS hands out
ephemeral ports from this range, which contains the rig's 48100-48199 (Linux, this machine):
```
/proc/sys/net/ipv4/ip_local_port_range -> 32768	60999
```
The same timeout-setting pattern as the Herdr bug, in this crate's production client (not changed by 642b; Follow-ups):
```
crates/holler-adapter-opencode/src/http.rs:152-158
fn send(stream: &mut TcpStream, bytes: &[u8], deadline: Instant) -> Result<(), HttpError> {
    stream
        .set_write_timeout(Some(left(deadline)?))
        .map_err(|error| failed("set a send timeout", &error))?;
    stream
        .write_all(bytes)
        .map_err(|error| failed("send the request", &error))
```
```
crates/holler-adapter-opencode/src/http.rs:248-250
            self.stream
                .set_read_timeout(Some(left(self.deadline)?))
                .map_err(|error| failed("set a read timeout", &error))?;
```

**E11. The real rig's isolation**, lifted from the spike's library (642a brief AC rig, carried below):
```
scripts/spikes/opencode-lib.sh:64-67
  if (exec 3<>/dev/tcp/127.0.0.1/9) 2>/dev/null; then
    echo "127.0.0.1:9 accepts connections; the dead-end provider would not be dead. Refusing." >&2
    exit 2
  fi
```
```
scripts/spikes/opencode-lib.sh:98-111
  env -i \
    PATH="$(dirname "$OC_BIN"):/usr/bin:/bin" \
    HOME="$OC_DIR/home" \
    XDG_DATA_HOME="$OC_DIR/data" \
    XDG_CONFIG_HOME="$OC_DIR/config" \
    XDG_STATE_HOME="$OC_DIR/state" \
    XDG_CACHE_HOME="$OC_DIR/cache" \
    TERM="${OC_TERM:-xterm-256color}" \
    OPENCODE_DISABLE_AUTOUPDATE=1 \
    OPENCODE_DISABLE_MODELS_FETCH=1 \
    OPENCODE_DISABLE_CLAUDE_CODE=1 \
    OPENCODE_DISABLE_LSP_DOWNLOAD=1 \
    OPENCODE_DISABLE_SHARE=1 \
    "$@"
```
```
scripts/spikes/opencode-lib.sh:189-194
oc_guard_no_model() { # port
  oc_http GET "$1" /config/providers
  local got
  got=$(printf '%s' "$HTTP_BODY" | jq -c '[.providers[] | {id, url: .options.baseURL}]' 2>/dev/null || echo "?")
  if [ "$got" = '[{"id":"deadend","url":"http://127.0.0.1:9/v1"}]' ] \
    && ! (exec 3<>/dev/tcp/127.0.0.1/9) 2>/dev/null; then
```
(`opencode-lib.sh:68-85` writes the dead-end provider config: `enabled_providers: ["deadend"]`, `disabled_providers:
["opencode"]`, `model` and `small_model` `deadend/none`, the provider's `baseURL` `http://127.0.0.1:9/v1`. Lift it verbatim.)

**E12. Gates and CI:**
```
scripts/lint.sh:43-47
# 4. File-size gate: warn at 600 lines, fail at 900 (skill rule: never let a
#    file cross 1k unnoticed).
while read -r n f; do
  if [ "$n" -ge 900 ]; then
    echo "lint: $f is $n lines — decompose before merging"
```
```
.github/workflows/ci.yml:21-21
        os: [ubuntu-latest, macos-latest]
```

**E13. The issue's amendment and #700** (`gh issue view 642`, `gh issue view 700`, 2026-10-09):
```
#642 body: (amended 2026-10-09, agent; confirmed by the operator, decision 8 of the epic #633, ...) the pane's OpenCode agent
  (`Pane.opencode_agent`, an agent key; none means the server's default) reaches the harness through this adapter: applied as
  the server's `default_agent` when it is started, or per prompt if the OpenCode API takes an agent per message ...
  One extra conformance case: with an agent set, the first assistant message of a delivered prompt reports that agent; with
  none set, the server's default.
#642 body: Depends on: #637, #638, #635, #700 (the agent part: the `Pane.opencode_agent` field and the `--agent` flag).
#700: state OPEN. "Merges first among the stories that use the field: #642, #644 and #647 start when it has merged."
```

**E14. The pipeline's PR step** (`$WORKFLOW_ROOT/workflow/coding-pipeline.workflow.mjs`):
```
coding-pipeline.workflow.mjs:4818-4822
  const prResult = await runPrCreateAndVerify({
    repoPath: args.repoPath,
    branch: `issue-${args.issueNumber}-implementation`,
    title: `Implements #${args.issueNumber}`,
    body: `Closes #${args.issueNumber}.`,
```

## The public API (carried from 642a, then amended here)

Unchanged and reused as merged (E1): `OpenCodeConfig`, `Timeouts`, `ProcessEnv`, `Resolver`, `TmuxSocket`, `TmuxConfig`,
`OpenCodeHarness::new`, `http`. **Carried from 642a** (`d6393b2:docs/handoffs/642-brief.md:564-616`), the `tui.rs` surface
and the `exec.rs` plan:

`src/tui.rs` (the builders are public and pure so the hermetic tests pin the exact argv without running tmux):
```rust
/// Which tmux server to address; the same variants as #641's `TmuxSocket` (641-brief.md:368-369 and 262-264 at `dd5e99b`,
/// Evidence), so #649 configures one value and hands it to both adapters (decision 10). An adapter cannot depend on
/// another adapter crate, hence a mirror, not a re-export.
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
/// carries `#{pane_dead}`, `#{pane_dead_status}`, `#{pane_start_command}` and `#{pane_title}` (separator and order are
/// F's choice; `#{pane_dead_status}` is the exit status of a dead pane and empty for a live one, Evidence, W-5).
pub fn query_args(session: &PaneName) -> Vec<String>;

pub enum TitleShows { Session(String), Home, Unrecognised }
/// `OC | <id>` with a whole session id (`ses_` then [0-9A-Za-z]+, no `…`; OpenCode's own ids are 26 such characters,
/// Evidence, NV-7) -> Session(id);
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
window`, `can't find pane`, `no such pane` (what `set-option -p` prints for a missing exact target, Evidence, W-6),
`no server running`, or `error connecting to` with `No such file or directory` or `Connection refused` = **missing**;
anything else = **other**, carrying tmux's first stderr line. And the `kill -s KILL -- -<pgid>` call
(the `kill` binary, no `unsafe`). Shaped like #641's `exec.rs`, so a later consolidation is a move (Follow-ups).

**Amendments made here** (Decision 23; they replace "separator and order are F's choice" above):
```rust
/// The five fields, in this order, joined by one TAB (U+0009) each:
/// #{session_name}, #{pane_dead}, #{pane_dead_status}, #{pane_start_command}, #{pane_title}.
pub const QUERY_FORMAT: &str = "#{session_name}\t#{pane_dead}\t#{pane_dead_status}\t#{pane_start_command}\t#{pane_title}";

pub enum Query {
    /// The first field is not `session` (a missing exact target expands to empty fields, E7): no tmux pane.
    NoPane,
    /// `#{pane_dead}` is `1`; the exit status from `#{pane_dead_status}` when it parses as an integer.
    Dead(Option<i32>),
    /// A live pane: its start command and its title (the title is everything after the fourth TAB, kept whole).
    Live { start_command: String, title: String },
}
/// Read `query_args(session)`'s stdout (one trailing newline removed). Fewer than five fields, or a `#{pane_dead}`
/// other than `0` or `1`, is `NoPane` (fail closed: no TUI is assumed).
pub fn parse_query(session: &PaneName, stdout: &str) -> Query;
```
`query_args(session)` is `["display-message", "-p", "-t", "=<session>:", QUERY_FORMAT]`. `exec.rs` gains (private, F names
it): run a `Command` to its exit within a deadline with stdout and stderr captured on threads (bounded, e.g. 64 KiB each),
returning status, stdout and stderr, or `timeout { op }` past the deadline (child killed and reaped), or `unavailable` naming
the program when it cannot be spawned; and `classify(stderr) -> Missing | Other(first line)` with the carried rule above.
`kill_group` is unchanged.

## Behaviour (carried from 642a, verbatim; F implements, T tests)

General (`642-brief.md:620-647` at `d6393b2`):

Every method takes a deadline of `now + timeouts.call` at entry; every request and poll inside it uses the smaller of its own
timeout and what is left. Past the deadline the method answers `timeout` with its `op` (`"harness.<method>"`, the fake's
strings above). HTTP outcomes map the same way everywhere: `Refused` -> `unavailable` ("the harness server on port N");
`TimedOut` -> `timeout { op }`; `Garbled`, or a status the step does not expect -> `unavailable` naming the route and status.
Only `127.0.0.1` is ever contacted (epic decision 3).

**Reply shapes are required, not assumed (W-6, decision 12).** OpenCode answers an unknown route with its web app (`200`,
`text/html`; `holler-body/src/http_attach_driver.rs:39-48`, Evidence), so a `200` alone never counts as success. Each step requires the
JSON it relies on: `GET /global/health` an object with `"healthy": true`; `POST /session` an object whose string `id` starts
with `ses`; `PATCH /session/<id>` an object whose `title` equals the id; `GET /session` an array of objects each with a string
`id`; `GET /session/<id>` an object whose `id` equals the requested id; `POST /session/<id>/abort` and
`POST /tui/select-session` the JSON value `true`; `GET /session/status` an object. Anything else (including a body that is not
JSON) is `unavailable`, with a one-line message that names the route and the status and quotes at most 60 bytes of the body,
control characters replaced (ADR-0021 §9: one-line messages). A session id enters a URL path percent-encoded (every byte
outside `[A-Za-z0-9_-]`), so no id can change the request line; for the existence check `GET /session/<id>`, a `400` (a
malformed id; spike 117-118) reads as `404`, i.e. `session-not-found`. (The spike's `400` was observed for
`POST /tui/select-session`; what `GET /session/<malformed>` answers is unrecorded (Evidence, NV-8). The rule holds either
way: a `400` or a `404` from the existence check means no session of that id, and the step that follows is never sent.)

**Every tmux call (B-2, W-4, decisions 9 and 10)** is built by `tui::tmux_command` from one of the `tui.rs` builders, and its
`-t` is always `exact_target(session)` = `=<session>:`, where `session` is what `tui_session(pane)` returned. The adapter never
passes the resolver's value to tmux any other way and never builds a bare target. Every value the adapter did not author goes
through `escape_arg` (each element of the TUI argv after `--`, including the binary path, the session id and every `K=V` of
`Isolated`) or, for `-c`, `escape_dir` (the directory OpenCode reported). Constant formats and option values the adapter
writes are not escaped. Because `display-message -p -t` on a missing exact target exits 0 with empty fields (Evidence), a
query counts only when its first field equals the session name; otherwise there is **no tmux pane**. A failed call's stderr
is classified by `exec.rs`: missing -> no tmux pane; other -> `unavailable` with tmux's first stderr line (never an argv
element, a directory or an env value).

The three methods (`642-brief.md:678-710` at `d6393b2`):

- **`attach_tui(pane, port, session)`**: `GET /session/<id>` first: refused -> `unavailable`, 404 -> `session-not-found`;
  a 200 that is not that session -> `unavailable`; in all three cases the pane is not touched (cases 6, 11). Take `directory`
  from that reply. Resolve `tui_session(pane)` (an `Err` is returned as is). Run `remain_on_exit_args(session)`; a missing
  session -> `unavailable` ("no tmux session for pane P"). (On a missing exact target `set-option -p` exits 1 with `no such
  pane: =<session>:`, which `exec.rs` classifies as missing. A dead pane, kept by an earlier `remain-on-exit on`, is still a
  pane: `set-option` succeeds on it and the `respawn-pane -k` below revives it, so a dead pane is respawned, never reported
  as "no tmux session". Evidence, W-6.) Then `respawn_args(session, directory, tui_argv(...))`, i.e.
  `respawn-pane -k -t =<session>: -c <directory> -- <TUI argv>`, an **argv of several arguments** (tmux then execs it
  without a shell): `env -i K=V ... <opencode_bin> attach http://127.0.0.1:<port> --dir <directory> --session <id>` under
  `Isolated`, or `env -u OPENCODE_DISABLE_TERMINAL_TITLE <opencode_bin> attach ...` under `Inherit` (the title channel must
  stay on; spike 147, 151), every element escaped as above. An `opencode_bin` that is not valid UTF-8 -> `unavailable`. This
  replaces whatever ran in the pane, as the fake's "replacing any earlier one". Then poll
  `shown_session(pane)` until it is `Some(id)`, within `settle` -> `Ok`. If the pane's process dies first: re-`GET` the
  session; 404 -> `session-not-found`, otherwise `unavailable` ("the TUI in pane P exited with status N"). Deadline ->
  `timeout { op: "harness.attach_tui" }`.
  **How the poll sees a death (W-5, W-7):** each poll runs `query_args(session)` once and reads the reply itself rather than
  calling `shown_session` (whose `Ok(None)` cannot tell "not started yet" from "dead"). A reply whose first field is not the
  session name -> `unavailable` ("no tmux session for pane P"); `#{pane_dead}` = `1` -> the death branch above, with N taken
  from `#{pane_dead_status}` (the pane stays because of `remain-on-exit on`, decision 5); otherwise `parse_title(#{pane_title})`
  as `shown_session` reads it: `Session(id)` for the requested id -> `Ok`, anything else -> poll again.
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

**Added by 642b:**
- The `op` constants are `OP_ATTACH_TUI = "harness.attach_tui"`, `OP_SELECT_SESSION = "harness.select_session"` and
  `OP_SHOWN_SESSION = "harness.shown_session"`, equal to the test kit's `HarnessOp` strings (AC 30 pins them). Every tmux call
  runs within the method's deadline; past it the answer is `timeout` with the method's `op`.
- `select_session` builds its `Call` with the port `attach_port` returned and reuses `known` (E1) for `GET /session/:id`.
  `POST /tui/select-session` is a new `Route` (label `/tui/select-session`); its body is `{"sessionID": "<id>"}` and its
  reply must be the JSON `true` (a `404` is `session-not-found`; anything else is `unavailable` through `Call::unexpected`).
- `attach_tui` takes `directory` from the `GET /session/:id` reply's string field `directory`; a reply without one is
  `unavailable` (the pane is not touched). The `--dir` element of the TUI argv is that directory, passed through
  `escape_arg`; the `-c` value is the same directory through `escape_dir`.
- A message never echoes an argv element, a directory, an env value or the raw title (Risk 3); it may name the pane id,
  the port, the route, the status, and tmux's first stderr line.

## Acceptance criteria

**This run's ACs (642b), canonical for T, F and S:** AC 6 (642b clause), 9, 10, 11a, 11b, 11c, 11d (642b clause), 11f,
12-19a, 20, 21-26 as restated below, and the new 27-33. AC 1-5, 7, 8, 11, 11e and the 642a clauses are 642a's, merged, and
must keep passing unedited except for the refused-port helper of AC 31.

Hermetic tests run in CI on Linux and macOS under `cargo test --workspace`: `tests/tui_test.rs` (pure: AC 9, 10, 11a-11c,
11f, 27) and `tests/attach_test.rs` (the 642a stub through `#[path = "support/stub.rs"] mod stub;`, plus the fake tmux
fixture: AC 6, 11d, 28-30).

**Carried from 642a** (`642-brief.md:755-757, 765-772, 778-803` at `d6393b2`; the 642a-only clauses are already merged):

6. On an unbound port, `create_session`, `list_sessions` and `abort` are `unavailable` (the 642a clause). **642b's clause
   (not written or run in 642a):** on an unbound port `attach_tui` is `unavailable` and the `tui_session` resolver is never
   called (it records calls; the record is empty).
9. `parse_title`: `"OC | ses_0123456789abcdefABCDEFghij"` -> `Session`; `"OpenCode"` -> `Home`; `"OC | ses_ede8…"`,
   `"OC | New session - 2026-10-09T00:00:00Z"`, `"OC | my notes"`, `"somehost"` and `""` -> `Unrecognised`.
10. `attach_port`: `"env -u OPENCODE_DISABLE_TERMINAL_TITLE /usr/local/bin/opencode attach http://127.0.0.1:48123 --dir /p
    --session ses_1"` -> `Some(48123)`; the same through `env -i A=b ...` -> `Some(48123)`; `"bash"`,
    `"opencode serve --port 48123"` and `"opencode attach http://192.0.2.1:48123"` -> `None`. Include the quoted form tmux
    prints for `#{pane_start_command}` when an argument holds a space. Also `"/bin/oc attach http://127.0.0.1:48123"` (no
    `--dir` or `--session`) and the same with a flag after the URL (`... attach http://127.0.0.1:48123 --session ses_1
    --dir /p`) -> `Some(48123)`, so the parser does not depend on the flags that follow (NIT-2).
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
     `abort` is `unavailable` and the stub received no `POST .../abort`; (**642b's clause, not written or run in 642a:**
     `attach_tui` is `unavailable` and the `tui_session` resolver is never called). A `GET /session/<id>` whose object's `id` differs is `unavailable` the same way. With a valid
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

**Real OpenCode, carried from 642a** (`642-brief.md:805-853` at `d6393b2`), with one rule added for the ports (AC 31):

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

**New in 642b (hermetic; no tmux, no OpenCode, no `kill`):**

The **fake tmux** is a committed POSIX `sh` script, `tests/fixtures/fake-tmux` (git mode `100755`), used as `tmux_bin` with
`TmuxSocket::Path(<scratch>/sock)`. Its contract: it expects `-S <path>` as its first two arguments; it appends one line per
call to `<path>.calls` (the remaining arguments, each followed by U+001F); and for the subcommand in its third argument it
prints `<path>.<subcommand>.out` to stdout and `<path>.<subcommand>.err` to stderr when they exist, and exits with the
number in `<path>.<subcommand>.code` (default 0). The tests write only those data files, never a script (Decision 18).

27. **`parse_query`** (in the strings below `\t` is one TAB character). For `demo-c1r1`: `"demo-c1r1\t0\t\tenv -u OPENCODE_DISABLE_TERMINAL_TITLE /bin/oc attach
    http://127.0.0.1:48123 --dir /p --session ses_1\tOC | ses_1\n"` is `Live` with that start command and title `OC | ses_1`;
    `"demo-c1r1\t1\t7\tsh -c \"exit 7\"\t<anything>"` is `Dead(Some(7))`; `"\t\t\t\t\n"` (E7's missing target) is `NoPane`;
    a first field `demo` is `NoPane`; three fields is `NoPane`; a title holding a TAB stays whole. `query_args(demo-c1r1)` is
    `["display-message", "-p", "-t", "=demo-c1r1:", QUERY_FORMAT]`.
28. **`shown_session` through the fixture.** `tui_session` maps `w9:p1` to `demo-c1r1` and anything else to
    `pane-not-found`. (a) With a `Live` reply titled `OC | <id>` and a start command from `tui_argv(..)`, the answer is
    `Ok(Some(id))` and `<path>.calls` holds exactly one line, `display-message`, `-p`, `-t`, `=demo-c1r1:`, `QUERY_FORMAT`
    (the `-S <path>` pair is consumed by the fixture). (b) Titles `OpenCode`, `somehost`, `OC | ses_ab…`; a `Dead` reply; a
    start command `sleep 3600`; the four-TAB reply; and the fixture exiting 1 with stderr `no server running on /x/sock`,
    `can't find session: demo-c1r1` or `error connecting to /x/sock (No such file or directory)` each give `Ok(None)`. (c) The
    fixture exiting 1 with stderr `protocol version mismatch (client 8, server 7)` gives `unavailable` whose message holds
    that line and does not hold `QUERY_FORMAT` or `=demo-c1r1:`. (d) A `tmux_bin` that does not exist gives `unavailable`
    naming it. (e) `shown_session(w9:p2)` is the resolver's `pane-not-found`, unchanged, and `<path>.calls` does not exist.
29. **`select_session` through the fixture and the stub.** The fixture's reply is `Live` with the start command
    `tui_argv("/bin/oc", &Inherit, <stub port>, "/p", "ses_A")` joined by spaces. (a) Title `OC | ses_B` and the stub knowing
    `ses_B` with `POST /tui/select-session` answering `true`: `Ok(())`, and the stub's request lines are, in order,
    `GET /session/ses_B HTTP/1.1` and `POST /tui/select-session HTTP/1.1`, the second with body `{"sessionID":"ses_B"}`.
    (b) `GET /session/ses_B` answering 404: `session-not-found`, and no `POST /tui/select-session` was received. (c) A start
    command `sleep 3600` (no TUI): `unavailable`, its message names the pane, and the stub received nothing (case 14's order).
    (d) The title staying `OC | ses_A`: `timeout` with `op == "harness.select_session"` within `settle` plus 500 ms. (e) A
    `POST /tui/select-session` answering 200 HTML: `unavailable` (a 200 is not enough).
30. **`attach_tui` through the fixture and the stub.** The stub answers `GET /session/ses_A` with
    `{"id":"ses_A","directory":"/p#S;"}`. (a) With the display-message reply `Live` and titled `OC | ses_A`: `Ok(())`, and
    `<path>.calls` starts with exactly `set-option -p -t =demo-c1r1: remain-on-exit on`, then
    `respawn_args(demo-c1r1, "/p#S;", &tui_argv(<bin>, &env, <stub port>, "/p#S;", "ses_A"))`, whose `-c` value is
    `/p##S\;` and whose `--dir` element is `/p#S\;`, then one or more `display-message` calls. (b) The `set-option` step
    exiting 1 with `no such pane: =demo-c1r1:`: `unavailable` naming the pane, and no `respawn-pane` line was recorded. (c) A
    `Dead(Some(3))` reply: `unavailable` whose message holds `3`. (d) A `GET /session/ses_A` reply with no `directory`:
    `unavailable`, and `<path>.calls` does not exist. (e) The three `op` strings of 642b equal
    `HarnessOp::{AttachTui, SelectSession, ShownSession}.as_str()`.
31. **No test relies on a just-freed port being refused (E10).** One helper (in `tests/support/stub.rs`) gives every test
    that needs a refused port its port, replacing the direct uses of `closed_port()` in `hermetic_test.rs` and serving AC 6's
    642b clause. It (1) picks a candidate by bind-then-drop, (2) checks that a connect to it is refused immediately before the
    call under test, and (3) when the call's answer is not the expected one **and** a connect to the port now succeeds,
    discards the attempt and repeats it with a new candidate, at most 3 attempts in all; only an attempt that saw no listener
    before and after may fail the test. The real rig treats `serve`'s "port N is in use" the same way: it picks another port
    in 48100-48199 and does not fail on it.
32. **Platform-safe tests (E10; A and S check the test code; both CI legs run them).** No new test code calls
    `set_nodelay`, `set_linger`, `set_ttl`, `set_nonblocking` or any other socket option, except `set_read_timeout` and
    `set_write_timeout`, whose `ErrorKind::InvalidInput` (macOS's `EINVAL` on a socket whose peer closed) is treated as "go
    ahead and let the read or write answer", as 9ebedcc does. Hermetic tests run no `tmux`, `opencode`, `kill`, `ps` or
    `/proc`; the only programs they start are the committed fixture (through `/bin/sh`) and what 642a's tests already start.
    No test writes a file and then executes it (a concurrent `fork` can make that `exec` fail with "Text file busy"). No
    test synchronises with a fixed sleep (bounded polls only), and every timing bound is an upper bound with at least 300 ms
    of slack.
33. **The fixture runs anywhere the tests do.** `tests/fixtures/fake-tmux` starts with `#!/bin/sh`, uses only `printf`,
    `cat`, `test` and shell built-ins, and its git mode is `100755` (`git ls-files -s` shows it). A test checks, before
    using it, that the file is executable, and fails with a message that names `git update-index --chmod=+x` if not.

**Gates (restated for 642b):**

20. `cargo test --workspace` passes on both CI legs with the real tests skipped; and `HOLLER_TEST_OPENCODE=1 cargo test -p
    holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1` passes on a machine with OpenCode 1.18.x
    and tmux 3.2 or later (record the versions and the run's summary in `handoff-T-green.md`).
21. `cargo clippy --workspace --all-targets -- -D warnings` is clean; every new `.rs` file passes `rustfmt --check --edition
    2021` (epic ruling 4); no new `unsafe`; `bash scripts/lint.sh` passes (no file at 900 lines); `cargo machete` is clean
    (a `tempfile` dev-dependency is declared only if a test uses it).
22. `git diff --name-only origin/main` lists only `crates/holler-adapter-opencode/**`, `CHANGELOG.md`, `docs/adr/ADR-0021.md`,
    `crates/holler-pane/src/ports.rs`, `crates/holler-pane/src/lib.rs`, this run's `docs/handoffs/` files, and `Cargo.lock`
    only if a dev-dependency was added (then it changes only `holler-adapter-opencode`'s list). Every added or removed line of
    `git diff origin/main -- crates/holler-pane` is a `//!` or `///` line. No change to the test kit, the workspace
    `Cargo.toml`, `src/http.rs` or `src/server.rs`.
23. `CHANGELOG.md` `[Unreleased]`: the part-1 entry loses exactly the sentence "Attaching, switching and reading a pane's TUI
    answer `not-implemented` until part 2, which also brings the opt-in tests against a real OpenCode." (E9), and one new
    Enhancements entry "OpenCode adapter, part 2: the TUI side" links #642 and epic #633, says what the three methods do, that
    the real-OpenCode tests are opt-in, and that applying a pane's OpenCode agent is still to come (#700). No closing keyword.
24. Safety: `grep -rn "4700[0-9]\|--continue" crates/holler-adapter-opencode` finds nothing outside comments that forbid
    them; no test reads a real HOME, the default tmux server or, in the real rig, a port outside 48100-48199; no test prints
    the raw `#{pane_title}`, and assertions use the parsed value (A and S check).
25. The PR body: `Part of #642` (with the wording rule above); the AI disclosure (`CONTRIBUTING.md`); how each of the fake's
    three `ASSUMPTION (#642 to confirm)` comments stands (the shown session is read through **tmux** `#{pane_title}`, so the
    Herdr half no longer applies; cross-directory `select-session` and aborting a model turn remain **unverified**);
    `HarnessPort` confirmed unchanged and now recorded as built in ADR-0021; the two `FakeHarness` divergences (Decision 15)
    and their alignment follow-up (cite its issue number if the operator has filed it, P-3); and the open remainder: 642c, the agent, after #700.
26. **ADR-0021 and the port docs say what was built (Decision 7, edited in place, E9).**
    - **ADR-0021 §2, the paragraph at lines 100-103**, first sentence: "`HerdrPort` and `HarnessPort`, and the data types
      they take and return, stay **provisional** until the spikes #636 (Herdr) and #635 (OpenCode) report." becomes
      "`HerdrPort`, and the data types it takes and returns, stay **provisional** until the spike #636 (Herdr) reports.
      `HarnessPort` is final: the spike #635 confirmed it unchanged and #642 built it (see "`HarnessPort` as built (#642)"
      below)." The paragraph's other two sentences are unchanged.
    - **ADR-0021 §2, a new note** "`HarnessPort` as built (#642)" right after that paragraph, with the six facts of the
      carried list below and a seventh: "The pane's OpenCode agent (`Pane.opencode_agent`, #700) is not applied yet; see
      "Deferred to named stories"."
    - **ADR-0021 "Deferred to named stories", the item at line 537** "`HerdrPort` and `HarnessPort` in their final form:
      #636 and #635, then #640 and #642." becomes two items: "`HerdrPort` in its final form: #636, then #640." and "Applying
      `Pane.opencode_agent` in the OpenCode adapter: #642's last part, after #700."
    - **`ports.rs:171`**: "**Provisional** until spike #635 reports." becomes a sentence saying the spike #635 confirmed it and
      pointing at ADR-0021 §2's note. **`ports.rs:13-15`** and **`lib.rs:32-33`**: the sentence keeps `HerdrPort` provisional
      as now and says `HarnessPort` is final (built by #642). `HerdrPort`'s own wording (`ports.rs:119`) is untouched.
    - **Repoint the dangling pointers** in files this run edits: `lib.rs:37` ("the decisions of `docs/handoffs/642-brief.md`")
      and `exec.rs:1` ("decision 11 of the brief") cite ADR-0021 §2's note instead; T repoints the outside-gate ids in
      `hermetic_test.rs:243, 265, 510` to a plain statement of the rule. `lib.rs:7-11` and `tui.rs:1-7` describe part 2 as
      built and name 642c's agent as the remaining part.

The six facts of the note, **carried from 642a** (`642-brief.md:912-926` at `d6393b2`):

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

## Files

Production (F): `crates/holler-adapter-opencode/src/tui.rs`, `src/attach.rs` (new, private: `mod attach;`), `src/exec.rs`,
`src/lib.rs`; `CHANGELOG.md`. Docs (F): `docs/adr/ADR-0021.md`; doc comments only in `crates/holler-pane/src/ports.rs` and
`crates/holler-pane/src/lib.rs`. Tests (T): `tests/tui_test.rs`, `tests/attach_test.rs`, `tests/fixtures/fake-tmux`,
`tests/real_opencode_test.rs`, `tests/real_opencode/rig.rs`, `tests/hermetic_test.rs` and `tests/support/stub.rs` (helpers
only). Manifest: unchanged, unless the rig declares `tempfile = { workspace = true }` under `[dev-dependencies]` with a
one-line comment naming its use.

**Blast radius:** `crates/holler-adapter-opencode/**` (the issue's), plus `CHANGELOG.md`, `docs/adr/ADR-0021.md` (#634's file,
amended in the change that settles a contract point, as #639, #676 and #692 did; 642a's A ruled it in, B-1) and the doc
comments of two `holler-pane` files (no compiled change). Nothing depends on `holler-adapter-opencode` yet (`cargo tree -i`
shows no dependent), so no other test can break.

### Reuse map (extend the 642a objects; do not duplicate)

- **Reuse as merged, no edit:** `HarnessPort`, `PaneError` and its codes, `run_harness_conformance`, `HarnessRig`, `HarnessOp`
  (dev-dependency, `op` pins only); `OpenCodeConfig`, `Timeouts`, `ProcessEnv`, `Resolver`, `TmuxSocket`, `TmuxConfig`.
- **Reuse from the crate root, unchanged (E1):** `OpenCodeHarness::call` and `Call::{send, send_by, unexpected, timeout}` for
  every HTTP step; `known` for the existence check of `attach_tui` and `select_session`; `session_path`, `json_of`,
  `string_at`, `one_line`, `excerpt`, `deadline_after`, `budget`. A new `Route` constant for `POST /tui/select-session`
  beside the others.
- **Extend:** `exec.rs`'s `run` gains a capturing sibling (stdout and stderr on threads), sharing its deadline loop, and
  `classify`; `kill_group` stays as is. `tui.rs` grows from the two types to the pure builders and parsers of the API.
  `tests/support/stub.rs` (the HTTP stub, frozen mode and request record) is reused by `attach_test.rs` through `#[path]`;
  if a helper is unused there, the `mod stub;` line carries `#[allow(dead_code)] // #642: shared with hermetic_test.rs`.
- **New, justified:** `src/attach.rs` (642a's A, N-2: `lib.rs` is 483 lines and the three bodies with the attach poll would
  take it past 600; it mirrors `server.rs`, to which `serve` delegates in one line); the fake tmux fixture (no tmux double
  exists in the repo, and a committed script avoids writing an executable at test time, AC 32); the real rig (the spike's
  shell rig in Rust, E11; #667 may lift it later).
- **Mirrored, not shared:** `escape_arg`/`escape_dir`/`classify` follow #641's `escape`/`escape_cwd`/`classify` (E8) by rule
  and name, because an adapter cannot depend on another adapter crate (ADR-0021 §5); one shared home is #696.

## Decisions carried over verbatim (642a brief, `d6393b2`)

1, 3, 4, 5, 6, 9 and 10 (`642-brief.md:984-989, 1000-1010, 1016-1026`):

1. **The TUI is reached through tmux, not Herdr.** A pane's TUI runs in its tmux session (`Pane.name` = `host.tmux`,
   ADR-0021:37); the spike read the title through tmux `#{pane_title}`, and Herdr's `PaneInfo.title` is report-metadata, not
   the terminal title (`herdr-api-spike.md:137, 142`). The `PaneId` -> tmux **session name** mapping is injected
   (`tui_session`, typed `PaneName`, so only a valid name can reach a target); the adapter builds every tmux target itself
   (decision 9), so this crate needs no registry access and holler-cli never writes tmux syntax. This resolves the Herdr half
   of the fake's third assumption.
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

15 (`642-brief.md:1040-1044`):

15. **Two divergences from `FakeHarness`, recorded, not hidden (W-3).** (a) `serve` on a port already serving the same pane:
    the fake returns that server's pid, the adapter answers `unavailable` (decision 2). (b) A `PaneId` the resolver does not
    know: the fake gives `shown_session = Ok(None)` and `select_session = unavailable` (exit 1); the adapter returns the
    resolver's error, `pane-not-found`, a refusal (exit 3), from both. Neither is pinned by the suite. Both go in the crate
    docs, the ADR note and the PR body (AC 25); the test-kit follow-up aligns the fake.
Decision 7 (ADR-0021 updated in this change) is carried as AC 26. Decisions 2, 11, 12, 13 and 14 are built in 642a; 14 is
restated as Decision 21.

## Decisions made in this brief (MO)

16. **The agent amendment is 642c, after #700; 642b is `Part of #642`.** #700 is open with no PR (E13), and the agent part is
    the only part of #642 that depends on it (the issue scopes its `#700` dependency to "the agent part"). 642b reads no agent
    field and changes no signature the agent part needs, so it starts now. 642c will add a third resolver
    (`Resolver<PaneName, Option<AgentKey>>`) beside `workdir`, applied by `serve` after its health check; settle the route by
    a probe on scratch OpenCode (642a's A, N-1: the 1.18.35 bundle holds a `default_agent` config key and an
    `OPENCODE_CONFIG_CONTENT` overlay, both unverified behaviour; a per-prompt agent is the hub's `send_prompt` and
    `holler-body`'s driver, not `HarnessPort`); state the rule for an operator's own `OPENCODE_CONFIG_CONTENT` under
    `Inherit`; and add the crate-local real test that stands for the issue's "one extra conformance case" (it sends a prompt,
    so it relaxes the rig's no-prompt rule for one test against the dead-end provider). 642c is the part that completes #642. **PROPOSED for the
    operator** (P-1): the alternative is to hold 642b until #700 merges and build both together.
17. **Homes.** The three TUI bodies, the attach poll and the query reading live in `src/attach.rs`; `lib.rs`'s impl
    delegates in one line each, as `serve` does to `server.rs`. Pure builders and parsers live in `tui.rs`. Tests: pure ones
    in `tests/tui_test.rs`, process and stub ones in `tests/attach_test.rs`; `hermetic_test.rs` gains no case (642a's A, N-2).
18. **A committed fake tmux.** Hermetic tests reach the tmux code paths through `tests/fixtures/fake-tmux`, driven by data
    files, never by a script written at test time (AC 32-33). Real tmux runs only in the opt-in rig.
19. **Refused ports are re-checked, not assumed (AC 31).** Bind-then-drop picks a candidate only; interference is detected
    and retried, never asserted through. This also fixes 642a's flake in `hermetic_test.rs`.
20. **Socket options in tests follow 9ebedcc (AC 32).** The same latent pattern in `src/http.rs` (E10) is not changed here
    (no failure has been seen on this crate's macOS leg, and `http.rs` is outside this run's files); it is a follow-up.
21. **Decision 14, restated (642a's A, W-1).** The resolvers' precondition (answer for a pane being launched, before its
    record exists) is met by wiring (#649), as `lib.rs:100-105` says and #644's brief (C-9) agrees: the frozen methods give
    #644 no channel to pass a session name or directory. 642a's prose "#644 passes the session name (and directory)" is
    withdrawn. For `tui_session`, #649 maps the `PaneId` it got from `HerdrPort::ensure_pane` in the same act to the pane's
    tmux session name (`Pane.name`, ADR-0021:37).
22. **Error codes stay closed.** The three methods answer only `session-not-found`, `pane-not-found` (the resolver's, as is),
    `unavailable` and `timeout` (E3); none of them answers `not-implemented` once built. No new code, no new variant.
23. **The query is fixed now** (amends 642a's "separator and order are F's choice"): `QUERY_FORMAT`, `Query` and
    `parse_query` (API above), so T can write the canned replies of AC 27-30 before F's code exists. TAB is safe as the
    separator for the first four fields (E7 shows a TAB-joined reply, and a missing target as four TABs); the title, last,
    is kept whole.

**Invariants.** I3: `attach_tui` and `select_session` answer `Ok` only once the TUI's title shows the requested session
(observe before the verb records); `shown_session` never guesses (Decision 3). I8: the adapter holds no state and writes no
record, so a failed `act` leaves nothing of the adapter's to compensate; a failed `attach_tui` may leave the pane's TUI
replaced or dead, which the verb (#644) reports and the doctor (#647) observes. I4: no keystroke is ever sent; the TUI is
switched only over the API.

## Forward-compat (the consumers of this API)

| Consumer | Needs | Satisfied |
|---|---|---|
| #644 launch/relaunch | `attach_tui` confirmed by observation; `shown_session` | yes; the resolvers' precondition is #649's (Decision 21) |
| #645 switch/reset | `select_session` confirmed by observation | yes; a switch is trusted only when the title confirms it |
| #647 reconcile/doctor | `shown_session = None` for home, deleted, dead or unknown; never a hang | yes; it never contacts the server |
| #649 wiring | build `tui_session` from the act's `PaneId` and `Pane.name`; one `TmuxSocket` shared with #641 | yes (Decision 21); the Herdr-pane-to-tmux-session link (#644's C-8) is #640/#649's, not this crate's |
| 642c (agent) | a per-pane value applied at `serve` | yes: one more resolver, additive; nothing outside the crate constructs `OpenCodeConfig` yet |
| #640 part 3 | its own `HerdrPort` wording in the same ADR paragraph and doc sentences | whichever merges second rebases and keeps the other's wording (Risk 8) |
| #696 one runner | `exec.rs` shaped like #641's | yes; differences (stderr captured here, `LC_ALL=C` there) recorded on #696 |

## Out of scope

- The agent amendment (642c, Decision 16); a per-prompt agent (the hub and `holler-body`, not this crate).
- `src/http.rs` and `src/server.rs` (the EINVAL hardening and 642a's A-dup D-1 comment fixes are follow-ups), the test kit
  (including its `ASSUMPTION` comments and `FakeHarness` parity), the workspace manifest, `holler-cli`, #641's crate.
- Stopping a server (#695); consolidating the subprocess and tmux plumbing (#696); the Herdr-pane-to-tmux-session link.
- Password-protected servers, workspaces (`?workspace=`), cross-directory `select-session`, aborting a model turn, reading
  the TUI's screen as a fallback for the title.

## Follow-ups (the orchestrator files them; none is this run's work)

- **`http.rs` on macOS:** give `send`'s `set_write_timeout` and `fill`'s `set_read_timeout` (E10) 9ebedcc's `InvalidInput`
  tolerance. **PROPOSED** (P-2): file it as its own small issue.
- **`FakeHarness` parity** (642a's S, advisory 3, marked required and still unfiled: `gh issue list --search FakeHarness`
  finds only #684 and #638): align `serve`'s "never adopt", the unknown-pane answer (Decision 15), and retire the three
  `ASSUMPTION` comments. **PROPOSED** (P-3).
- 642a's A-dup D-1 (the comments at `http.rs:9-10` and `stub.rs:5-6`) and D-2 (record the two `kill` runners' differences on
  #696). If T edits `stub.rs` anyway, T fixes its D-1 comment there.

## Test plan

**RED first (T).** `tests/tui_test.rs` and `tests/attach_test.rs` call `tui::{tmux_command, exact_target, escape_arg,
escape_dir, tui_argv, respawn_args, remain_on_exit_args, query_args, QUERY_FORMAT, Query, parse_query, TitleShows, parse_title,
attach_port}`, which do not exist, so they fail to compile today. As in 642a, T may land the minimum public stubs (each pure
function returns an empty vector, an empty string, `None`, `NoPane` or `Unrecognised`; the three methods keep
`NotImplemented`) so RED is a runtime assertion failure; record in `handoff-T-red.md` which ACs fail and why. AC 31's helper
lands in RED too, and every 642a test still passes with it. T runs the real file once with `HOLLER_TEST_OPENCODE=1` to show
RED (the three methods answer `not-implemented`; AC 12 lists the failing cases), and confirms that without the variable every
real test returns at once.

**GREEN (T verify).** `cargo test -p holler-adapter-opencode` (several concurrent runs, to shake out port and timing
flakes); the opt-in run of AC 20 with `--test-threads=1`; clippy, rustfmt, `scripts/lint.sh`, `cargo machete`. Paste the
conformance result and the OpenCode and tmux versions into `handoff-T-green.md`.

## Risks

1. **Title timing.** Between `respawn-pane` and the TUI's first title (about 1.6 s) tmux shows the host name, which parses as
   `Unrecognised`; `attach_tui` keeps polling until `settle`. Tests never print the raw title.
2. **tmux quoting of `pane_start_command`.** tmux re-quotes elements (E7); `attach_port` must find `http://127.0.0.1:<port>`
   inside quotes (AC 10). If T-green finds `pane_start_command` does not follow `respawn-pane` on another tmux, F may read
   `#{pane_pid}`'s command line instead (Decision 4).
3. **Leaked processes in the real rig.** Build the guard before the first `serve`; record pids through the wrapper; the
   scratch HOME keeps any leak away from real state.
4. **macOS CI.** Only the hermetic files run there. The fixture is POSIX `sh`; nothing runs tmux, `kill`, `ps` or `/proc`
   (AC 32); socket options follow 9ebedcc.
5. **The fixture's executable bit** can be lost by a checkout with `core.fileMode=false`; AC 33's check fails loudly.
6. **A refactor that bypasses the builders** (a bare `-t`, an unescaped element, a tmux `Command` not made by `tmux_command`)
   reopens the prefix-match and injection holes. AC 11a-11c and 30(a) pin them; A-dup and S check every tmux spawn goes
   through `tmux_command` and every `-t` through `exact_target`.
7. **Broadcast.** `select-session` reaches every TUI of a server; one TUI per server is the caller's precondition (Decision
   6); case 15 and AC 19a show a switch reaching only its pane under it.
8. **Merge order with #640 part 3.** Both edit ADR-0021 §2's provisional paragraph, the "Deferred" item at line 537 and the
   same `holler-pane` doc sentences, each for its own port; the second to merge rebases and keeps the other's wording.

## PROPOSED items for the operator

- **P-1** Decision 16: 642b runs now as `Part of #642`, and the agent amendment becomes 642c after #700 merges (the epic's
  wave table says #642 needs #700; this reads it as "#642's agent part needs #700").
- **P-2** File the `http.rs` macOS socket-timeout follow-up (E10).
- **P-3** File the `FakeHarness` parity follow-up (642a's S required it; it is still unfiled).
