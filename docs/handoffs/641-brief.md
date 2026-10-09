# Brief: #641 host-adapter (tmux sessions, process control, the launcher primitive)

Repo: Performant-Labs/holler. Issue: #641 (epic #633, wave 3). Rigor: second-opinion. UI surface: no. Kind: feature.

**Branch:** `issue-641-implementation` (worktree `.claude/worktrees/0641-host-adapter`, from `origin/main` at `9d61c9f`).
**Review-rigor:** second-opinion (operator, 2026-10-09; the outside model is deepseek-v4-pro; the issue and the epic fix it; the change is one new adapter crate behind a frozen trait and a
conformance suite that already exists).
**Forward-compat:** done, see the table under "Decisions". **Design:** N/A (no UI surface).
**Decision record:** ADR-0021 (`docs/adr/ADR-0021.md`) section 2 and the epic's contract. This brief does **not** edit the ADR:
the design fits the frozen `HostPort` trait, and the one narrowing it makes (which processes are "owned") is recorded in the
adapter's rustdoc and below.
**Handoffs:** `docs/handoffs/641/handoff-<phase>.md` (T-red, F, T-green, A, A-dup, S); the decision journal is
`docs/handoffs/641/decisions.md`.
**Amended 2026-10-09 after A's BLOCK** (`docs/handoffs/641/handoff-A.md`, 3 BLOCK + 9 WARN). Every finding is resolved below
and no earlier decision is reversed: B-1 and B-2 in Decision 13, AC 6d, 6g, 11 and Risks; B-3 in Decisions 1 and 14 and
AC 6h; W-1 in Decisions 6, 7, 12; W-2 in Decisions 7, 8 and AC 6c, 6g; W-3 in Decision 3 and AC 6i; W-4 in AC 7; W-5 in
AC 8 and the Test plan; W-6 in Decision 15 and the forward-compat table; W-7 in the Reuse map and Follow-ups; W-8 in
Decisions 3, 4, 15 and AC 6h; W-9 in the note below and AC 10.
**Amended again 2026-10-09 after A's round-2 BLOCK** (`handoff-A.md` round 2, 2 BLOCK + 4 WARN), applying its Notes for O
items 1-8; additions only, no decision reversed: B-4 in Decisions 4, 5, 10, AC 6e, new AC 12, Risks and Evidence; B-5 in
Decisions 3, 15, AC 6d, 11, new AC 13, the #644 row and Evidence; W-10 in Decision 4 and AC 6h; W-11 in Decisions 4, 15
and AC 6h; W-12 in Decisions 4, 9 and AC 6f, 6h; W-13 in Decision 15 and the #644 row. O re-ran every round-2 probe
(Evidence, "round 2").
**Public repository (W-9):** the issue's title ends with a personal host name. It stays out of the CHANGELOG entry, the PR
title and body, commit messages, rustdoc, test names and test data.

## Problem

`HostPort` (ensure a tmux session, run an argv in it, stop what it owns, list its pids) has a fake and a conformance suite
(#684) but no real implementation: `holler-adapter-host` is an empty skeleton. Without it no `holler pane` verb can create
the tmux session of record or start a harness in it, and launches stay keystroke-and-sleep (type a command into a shell, wait,
hope). This story builds `TmuxHost`, the real `HostPort`, which talks to a local tmux server (epic decision 3: no ssh), never
types into a shell, bounds every call, and stops only the processes it started.

There is **no launcher code in this repository to extend**: the keystroke-and-sleep launcher is the operator's external
tooling (pfleet), and the `launch` verb that composes these primitives is #644. In this repo `tmux` appears only in the
pane-control contract, the test kit and the spike scripts (`scripts/spikes/*.sh`, which drive Herdr, not tmux, with
`send-keys`). "The launcher" in the issue title is this adapter's `run`: it starts an argv in the session and returns once
tmux reports the new process, with no typing and no sleep.

## Evidence (verbatim, as of `9d61c9f`)

The port (frozen by #637):
```
crates/holler-pane/src/ports.rs:150-168
/// The machine a pane lives on: tmux sessions and the processes in them (the adapter
/// is `holler-adapter-host`, #641).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
pub trait HostPort: Send + Sync {
    /// Make the tmux session `name` exist, working in `cwd`.
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;

    /// Run `argv` (never through a shell) in the session `name`.
    fn run(&self, name: &PaneName, argv: &Argv) -> Result<(), PaneError>;

    /// Stop the processes the session `name` owns.
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;

    /// The process ids running in the session `name`.
    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>;
}
```

The conformance suite the adapter must pass, and the readings it fixes (#684):
```
crates/holler-pane-testkit/src/conformance/host.rs:4-20
//! The cases pin what the tmux adapter (#641) must keep: `run` starts a process that
//! `ps` then lists, an empty argv is refused, `ensure_session` is idempotent, and
//! `stop_owned` stops every process of its session and none of another session's (no
//! broad kill). Two readings of the port are decided here (#684), and they bind the
//! adapter:
//!
//! - A missing session is `pane-not-found` for `run` and `ps`, which is how this suite
//!   reads #641's "a missing session is a typed error". The tmux session is named by the
//!   pane's name, so the pane's code fits; `session-not-found` stays the code of a
//!   harness session. An empty argv is `usage`.
//! - `stop_owned` of a missing session is `Ok`: nothing is owned, so nothing is
//!   stopped, and a `relaunch` or `close` after a crash does not fail on it (case 7).
//!
//! The cases assert only what a real tmux session can also satisfy. A real session may
//! hold a shell process, so no case asserts that a fresh session's `ps` is empty; and it
//! may end with its last process, so a case ensures the session again before it reads
//! `ps` after `stop_owned`. Every list of pids is read as a set: the port fixes no order.
```
```
crates/holler-pane-testkit/src/conformance/host.rs:75-93
/// Run every case of [`host_cases`], in order, each against a fresh host with no
/// session, and return every case that did not hold.
///
/// `fresh` is called once per case. It returns the host and a guard that the suite
/// keeps alive for that case only; the host is dropped before its guard. How each
/// implementation runs the suite (the test kit cannot name an adapter):
///
/// ```text
/// // the fake:
/// assert_eq!(run_host_conformance(|| (FakeHost::new(), ())), Ok(()));
/// // holler-adapter-host (#641): a private tmux server per case
/// //   (`tmux -S <tempdir>/tmux.sock`), the adapter on it as the host, and the tempdir
/// //   and the tmux server's handle as the guard, so dropping the guard kills every
/// //   process the case started.
/// ```
pub fn run_host_conformance<S, K, F>(fresh: F) -> Conformance
where
    S: HostPort,
    F: FnMut() -> (S, K),
```
The nine case ids (`host_cases()`), every one run with the argv `["sleep", "30"]`, sessions `demo-c1r1`/`demo-c2r1`, cwd
`std::env::temp_dir()`: `ps-of-missing-session-is-pane-not-found`, `run-in-missing-session-is-pane-not-found`,
`run-adds-a-process` (the pid `run` added must appear in the very next `ps`), `ensure-session-is-idempotent`,
`run-empty-argv-is-usage`, `stop-owned-stops-every-owned-process` (re-ensures the session, then `ps` must lack both pids),
`stop-owned-of-missing-session-is-ok`, `stop-owned-leaves-other-sessions`, `ps-lists-only-its-session`.

The fake's rules, which the adapter must match on every input (the verbs are tested on the fake):
```
crates/holler-pane-testkit/src/host.rs:52-61
/// - `ensure_session` creates a missing session working in `cwd`; on an existing one it
///   is `Ok` and changes nothing, neither the cwd nor the processes.
/// - `run` in a missing session is `pane-not-found`, and an empty argv is `usage`.
///   Otherwise it starts one process in the session and records the argv exactly as
///   given: an element is never joined to another or re-split, and nothing goes
///   through a shell.
/// - `stop_owned` stops every process of the session and of no other session. On a
///   missing session it is `Ok`: nothing is owned, so nothing is stopped.
/// - `ps` of a missing session is `pane-not-found`; otherwise the session's pids, in
///   the order they started.
crates/holler-pane-testkit/src/host.rs:208-214   (order: the missing session is checked BEFORE the empty argv)
    fn start(&mut self, name: &PaneName, argv: &Argv) -> Result<(), PaneError> {
        let session = self.sessions.get_mut(name).ok_or_else(|| not_found(name))?;
        if argv.as_slice().is_empty() {
            return Err(PaneError::Usage {
                message: "an empty argv has no program to run".to_owned(),
            });
        }
```

The error variants used (closed set, not edited here):
```
crates/holler-pane/src/error.rs:409, 456, 458, 467
    Usage { message: String },
    Timeout { op: String },
    PaneNotFound { what: String },
    Unavailable { what: String },
```

The crate today (both files replaced/extended by this story):
```
crates/holler-adapter-host/Cargo.toml:12-15
# Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI
# otherwise). The crate is an empty skeleton; its owning story adds the
# dependencies it uses.
[dependencies]
crates/holler-adapter-host/src/lib.rs:1-5
//! `holler_adapter_host` — the host adapter: it implements `holler_pane::HostPort`
//! (tmux sessions and the processes in them) for the machine the hub runs on
//! (epic #633).
//!
//! Empty skeleton (story #637); story #641 fills it.
```

The consumer seam (owned by #649, **not touched here**): `crates/holler-cli/src/pane/wiring.rs:25-34` builds `Wiring` with
the `Unwired` stub whose `impl HostPort for Unwired` (line 151) answers `not-implemented`. #649 will replace it with this
crate's `TmuxHost`.

Workspace rules that bind the code: `Cargo.toml` `[workspace.lints]` denies `unwrap_used`, `expect_used`, `panic`,
`unreachable`, `cognitive_complexity` (threshold 15), `too_many_lines` (threshold 100 per fn) and `dead_code`; test files open
with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` (as `holler-pane-testkit/tests/*.rs` do). `libc` is
in the workspace only for test harnesses that use `unsafe`; the epic forbids new `unsafe`. `tempfile = "3"` is a workspace dep.

tmux behaviour this design relies on, observed by O on tmux 3.7c against a private `-L` socket (never the default socket):
```
has-session -t demo            (session "demo-c1r1" exists)  -> exit 0      PREFIX MATCH: the bare target is unsafe
has-session -t =demo                                         -> "can't find session: demo", exit 1
list-panes -s -t =nope ...     (server up, no such session)  -> "can't find window: nope", exit 1
                               (a WINDOW lookup: without ":" the target is not an exact session; see round 2, B-4)
new-window -t =nope: -- ...                                  -> "can't find session: nope", exit 1
has-session / list-panes, no server ever started             -> "error connecting to <sock> (No such file or directory)", exit 1
list-panes after kill-server (stale socket file)             -> "no server running on <sock>", exit 1
socket path longer than sun_path                             -> "error connecting to <sock> (File name too long)", exit 1
new-session -d -s demo-c1r1 (exists)                         -> "duplicate session: demo-c1r1", exit 1, nothing changed
new-window -d -P -F '#{pane_pid} #{window_id}' -t =demo-c1r1: -- sleep 30
                                                             -> "1505489 @1"; ps: PID=PGID=SID=1505489, comm sleep (no shell)
new-window ... -- 'sleep 31'   (ONE argument)                -> runs "sh -c sleep 31": tmux hands a single argument to a shell
new-window ... -- env -- sleep 32                            -> pane_pid's comm is sleep, PGID == pid (env execs in place)
new-window ... -- env -- 'sleep 33'                          -> exits at once (no program "sleep 33"): no shell involved
set-option -w -t @1 @holler-pid 1505489; list-panes -F '#{pane_pid} #{@holler-pid}' -> "1505489 1505489"
kill -s TERM -- -1505489                                     -> the window closes; list-panes no longer lists it
kill-server                                                  -> leaves the socket FILE behind (a guard must delete its dir)
```

Added after the plan review: A's probes (handoff-A.md "Probe evidence", E1-E5, R1, C1-C2, K2) and O's re-probe, all on
tmux 3.7c on private `-S` servers under `/tmp`, each killed and deleted afterwards. Shell quoting below; `\;` at the shell
is one argv element `;`.
```
new-window ... -- env -- sh -c '<print args>' f 'x;' rename-session pwned
                                    -> the program got only "x"; the session was renamed "pwned" (E1)
                                       (an element ending in ";" ends the tmux command, even after "--"; man tmux PARSING SYNTAX)
elements 'x\;' 'y\\;' '\;'          -> arrive as x;  y\;  ;    ('{' '}' '#{session_name}' '~' '-t' 'a b' '' arrive unchanged) (R1)
new-session -c "<dir>/p#S"          -> session_path "<dir>/p": the start directory is format-expanded (E3)
new-session -c "<dir>/#(touch <dir>/job-ran)"   (an existing dir)
                                    -> <dir>/job-ran created: #(...) ran through the server's shell (E4)
new-session -c "<dir>/p##S"         -> session_path "<dir>/p#S" exactly; no job runs (C1, C2)
new-session -c "<dir>/q\;"          -> session_path "<dir>/q;" exactly (O)
new-window ... \; set-option -w -F @holler-pid '#{pane_pid}'   (one invocation)
                                    -> exit 0, no window tagged: keep new-window and the tag as two invocations (K2)
set-option -w -t @2 @holler-pid N \; set-option -w -t @2 remain-on-exit off   (one invocation, literal values)
                                    -> both options set on @2 (O)
global remain-on-exit on; kill -s TERM -- -<pid>
                                    -> a window without its own remain-on-exit stays listed with pane_dead=1;
                                       the window with remain-on-exit off is gone (O)
```

Added after the plan review, round 2: A's probes (handoff-A.md round 2, P1-P7, Q1-Q5, R1-R2), re-run by O with the
transcript below (A's P1 is P2's shape with a shorter prefix; A's Q1-Q4 checks of `has-session -t =NAME`, `new-window -t
=NAME:` and `list-panes -t =NAME:` are the second to fourth commands of P2 and P4). Setup (reproducible): tmux 3.7c, procps-ng 4.0.4; `unset TMUX TMUX_PANE`; `D=$(mktemp -d
/tmp/hlr-o641-XXXXXX)`; every call is `tmux -S $D/s -f /dev/null ...`; dirs `$D/A`, `$D/B`, `$D/C`; the probing shell's
cwd is `$D/C`; an exit trap ran `kill-server` and `rm -rf $D`. `<dir>` is `$D`, `<pid>` a pid tmux printed. Output verbatim:
```
## P2 prefix: only demo-c1r10 exists (new-session -d -s demo-c1r10 -c $D/A)
$ tmux list-panes -s -t =demo-c1r1 -F #{session_name} #{pane_dead}
demo-c1r10 0                                     -> exit 0   (the WRONG session: prefix match)
$ tmux list-panes -s -t =demo-c1r1: -F #{session_name} #{pane_dead}
can't find session: demo-c1r1                    -> exit 1
$ tmux has-session -t =demo-c1r1
can't find session: demo-c1r1                    -> exit 1
$ tmux new-window -d -t =demo-c1r1: -- env -- true
can't find session: demo-c1r1                    -> exit 1
## P3 digits: session 1 (made first), then the newest session zz with windows 0 and 1
$ tmux list-panes -s -t =1 -F #{session_name}:#{window_index}
zz:0
zz:1                                             -> exit 0   (window index 1 of the current session)
$ tmux list-panes -s -t =1: -F #{session_name}:#{window_index}
1:0                                              -> exit 0
## P4 window name: no session demo-c2r1; new-window -d -n demo-c2r1 -t =zz:
$ tmux list-panes -s -t =demo-c2r1 -F #{session_name}:#{window_name}
zz:tmux
zz:tmux
zz:demo-c2r1                                     -> exit 0   (the window named demo-c2r1 in zz)
$ tmux list-panes -s -t =demo-c2r1: -F #{session_name}:#{window_name}
can't find session: demo-c2r1                    -> exit 1
$ tmux has-session -t =demo-c2r1
can't find session: demo-c2r1                    -> exit 1
$ tmux new-window -d -t =demo-c2r1: -- env -- true
can't find session: demo-c2r1                    -> exit 1
## Q5 list-panes -s -t =demo-c1r10: -F '[#{pane_pid} #{pane_dead} #{@holler-pid}]', after one run + tag
[<pid> 0 ]                                       (the untagged shell pane: the tag field is EMPTY)
[<pid> 0 <pid>]
## P5 cwd: session made in <dir>/A, client cwd <dir>/C; new-window -d -P -F '#{pane_pid}' [-c ...] -t =demo-c1r10: -- env -- sleep 30
no -c:                   cwd <dir>/C             (readlink /proc/<pid>/cwd: the CLIENT's cwd)
-c '#{session_path}':    cwd <dir>/A
## P6 sessions made with -c "$D/p##S" and -c "$D/##(touch $D/ran2)"; then run with -c '#{session_path}'; sleep 1
demo-c3r1 session_path: <dir>/p#S
demo-c3r1 run cwd:      <dir>/p#S
demo-c4r1 session_path: <dir>/#(touch <dir>/ran2)
demo-c4r1 run cwd:      <dir>/#(touch <dir>/ran2)
ran2 does not exist                              (#{session_path} is expanded once, not again)
## R2 set-option -g remain-on-exit on; new-window ... -- env -- true; sleep 0.5; the tag invocation of Decision 3
tag exit 0
[<pid> 1 <pid>]                                  (list-panes -t <window_id>: a DEAD pane, still tagged)
/usr/bin/kill: (<pid>): No such process          (kill -s 0 -- <pid>: the pid is free for reuse)
## P7 LC_ALL=C /usr/bin/kill -s 0 -- -<pgid of a reaped child>
/usr/bin/kill: (-<pgid>): No such process        -> exit 1
## R1 kill-server; HLR_PROBE_VAR=from-first-client tmux ... new-session -d -s demo-c1r1;
##    then a client WITHOUT the variable: new-window -- sh -c 'printf "%s\n" "${HLR_PROBE_VAR-unset}" > "$0"' $D/env.out
pane printed: from-first-client                  (a tmux client's environment reaches every later pane)
```
Afterwards no `/tmp/hlr-o641-*` dir and no probe server was left.

Why a prefix or an all-digit pane name is legal (so `hj-c1r1` beside `hj-c1r10`, or a session `1`, can exist):
```
crates/holler-proto/src/vocab.rs:24        pub const MAX_SEGMENT_LEN: usize = 32;
crates/holler-proto/src/vocab.rs:212-223   for &c in seg { if !(is_word(c) || c == b'-') { return Err(NameError::BadCharacter); } }
                                           ... fn is_word(b: u8) -> bool { matches!(b, b'0'..=b'9' | b'a'..=b'z') }
docs/adr/ADR-0021.md:37                    ... It is also the tmux session name. Names such as `hj-c1r2` are names, not positions.
```
Why `run` must work in the session's directory (B-5):
```
crates/holler-pane/src/ports.rs:157        /// Make the tmux session `name` exist, working in `cwd`.
docs/adr/ADR-0021.md:136                   | `--project DIR` | `host.cwd` |
docs/adr/ADR-0021.md:483                   | the launcher's port and directory table | ... | `harness.port`, `host.cwd`, `command` ...
epic #633 body                             host:    { cwd }                      // project directory or worktree
```

## Acceptance criteria

All run from the worktree. "Real-tmux" tests are `#[ignore]` (the issue's opt-in) and are run explicitly; each one returns
early with an `eprintln!("skipped: tmux not found")` when `tmux -V` cannot run, so `-- --ignored` on a machine without tmux
passes and says why.

1. **Conformance (real tmux).** `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored
   the_tmux_host_passes_the_host_conformance_suite` passes: `run_host_conformance(fresh)` returns `Ok(())`, where `fresh`
   builds a `TmuxHost` on a private server per case (AC 9) and returns the guard. Every one of the nine cases holds, which
   fixes: `ps`/`run` on a missing session is `pane-not-found`; `stop_owned` on a missing session is `Ok`; `run` of an empty
   argv in an existing session is `usage` and starts nothing; `run` in a missing session with an empty argv is
   `pane-not-found` (the fake's order).
2. **Ensure twice makes one session (real tmux, ignored):** `ensure_session_twice_makes_one_session` calls `ensure_session`
   twice (the second with a different cwd) and asserts `tmux -S <sock> list-sessions -F '#{session_name}'` prints exactly one
   line, `demo-c1r1`, and the session's `#{session_path}` is still the first cwd. A third call, with a cwd that is not an
   existing directory, returns `Ok` on the real server (Decision 6: the session exists) and leaves the same one session and
   `#{session_path}` (outside review round 1, W-3).
3. **Stop kills only the owned process (real tmux, ignored):** `stop_owned_kills_only_the_owned_process` runs `sleep 30` in
   `demo-c1r1` and in `demo-c2r1`, and the test itself spawns a third `sleep 30` outside tmux (`std::process::Command`, killed
   by the test at the end). After `stop_owned(demo-c1r1)`: the c1r1 run's pid is gone from `ps` and from the OS (`kill -0`
   fails, checked with the `kill` binary); the session `demo-c1r1` still exists and its shell pane (the pane `ensure_session`
   created) is still in `ps`; the c2r1 pid and the outside `sleep` (same program name) are alive.
4. **Escalation (real tmux, ignored):** `stop_owned_escalates_to_kill` runs `["sh", "-c", "trap '' TERM HUP; sleep 30"]`, sets
   the stop grace to 200 ms, and asserts `stop_owned` returns `Ok` within the bound and the pid is gone.
5. **Missing session is a typed error (real tmux, ignored):** `missing_session_is_typed` on a private socket where **no
   server was ever started**: `ps` and `run` are `PaneError::PaneNotFound`, `stop_owned` is `Ok` (proves the "error
   connecting ... No such file or directory" path maps like "can't find session").
6. **No tmux needed by default.** `cargo test -p holler-adapter-host` (no `--ignored`) passes on a machine without tmux. Its
   tests drive `TmuxHost` against a **fake tmux binary**: a `/bin/sh` script the test writes into a tempdir, which appends its
   arguments to a calls file and answers with a canned stdout, stderr and exit status, either one answer for every call or
   call by call from a numbered queue (answer N for the N-th call, the last answer repeating). Every test that can reach a
   signal (any `stop_owned` whose listing has a tagged pane, any `run` that reaches the cleanup of Decision 3) also sets
   `with_kill_binary` to a **fake `kill`**: a script that only appends its arguments to a kill record and answers with a canned
   stderr and exit status. Decision 14 is the rule. They assert:
   - a. **Bounded:** with a fake that runs `exec sleep 30`, each of the four methods returns `PaneError::Timeout { op }` with
     `op` equal to `host.ensure_session` / `host.run` / `host.stop_owned` / `host.ps` (the `HostOp::as_str` names of the
     test kit) within the configured timeout (200 ms) plus 1 s, and the hung fake process is reaped (its pid, written by the
     script to a file, no longer exists).
   - b. **Missing binary:** a tmux path that does not exist gives `PaneError::Unavailable` from all four methods (including
     `stop_owned`: a missing binary is not a missing session).
   - c. **Error mapping:** stderr `can't find session: x`, `can't find window: x`, `no server running on /s`, `error connecting
     to /s (No such file or directory)` (exit 1) each map to `PaneNotFound` for `ps` and `run` and to `Ok` for `stop_owned`;
     stderr `error connecting to /s (Permission denied)` or `(File name too long)` and any other non-zero exit map to
     `Unavailable` whose `what` carries tmux's first stderr line (never an argv element or the cwd: with a sentinel string in
     the argv and the cwd, `what` lacks it).
   - d. **Argv passed exactly, never through a shell:** `run(demo-c1r1, ["prog", "a b", "$(id);x", "-t"])` makes the fake
     record, as separate arguments and in this order, `-S <sock>` first, then `new-window -d -P -F #{pane_pid} #{window_id}
     -c #{session_path} -t =demo-c1r1: -- env -- prog "a b" "$(id);x" -t` (`-c` and the literal, unescaped `#{session_path}`
     at exactly that position, B-5); a one-element argv `["prog"]` is also prefixed with `env --` (so tmux
     never sees a single argument, which it would give to a shell). The record shows one tmux call after it, `set-option -w
     -t <window_id> @holler-pid <pid> ; set-option -w -t <window_id> remain-on-exit off` (the `;` a separate element the
     adapter authors), with the pid and window id the fake printed. Precisely (B-2, outside review round 1; Decision 3): the
     calls file holds exactly **two** entries for this `run`, one per spawn of the fake: the `new-window` entry above, then a
     second, separate entry whose arguments after the socket flags are that `set-option ... ; set-option ...` vector. The
     test asserts there are two entries, that the first holds no `set-option` and the second no `new-window` (this is what
     tells the K2-failing form, the tag chained onto `new-window` in one spawn, apart from the probed one). **Trailing `;` escaped (B-1, Decision 13):**
     `run(demo-c1r1, ["prog", "x;", "y\\;", ";", "kill-server"])` (Rust literals, so the third element is `y\;`) makes the
     fake record, after `env --`, these separate arguments in order: `prog`, `x\;`, `y\\;`, `\;`, `kill-server`.
   - e. **Exact targets (B-4, Decision 10):** across `ensure_session`, `run` (both the plain and the `has-session` paths),
     `ps` and `stop_owned` (its first listing and its polls), `has-session` records `-t =demo-c1r1`, and `new-window` and
     every `list-panes` record `-t =demo-c1r1:`. No recorded call has the bare name, and no `new-window` or `list-panes` has
     `=demo-c1r1` without the colon. A case here that drives `stop_owned` with a tagged pane uses the fake `kill`.
   - f. **Socket flags:** `TmuxSocket::Path(p)` puts `-S p` and `TmuxSocket::Name(n)` puts `-L n` before the subcommand;
     `TmuxSocket::Default` puts neither; a configured config file adds `-f <path>`. Every spawned tmux has `TMUX` and
     `TMUX_PANE` removed from its environment: the test re-runs its own test binary (`std::env::current_exe()`) as a child
     with `TMUX=/nonexistent,1,0` and `TMUX_PANE=%99` set on that child `Command` and a filter naming one helper test (which
     returns at once unless a marker variable is set), and with `LC_ALL` removed from that child (`env_remove`); the helper
     drives the adapter against the fake, the fake writes `${TMUX-unset} ${TMUX_PANE-unset} ${LC_ALL-unset}` to a file, and
     the parent asserts the file reads `unset unset unset` (W-12: nothing is added to a tmux subprocess's environment). No
     `set_var`, no `unsafe`.
   - g. **Bad input refused, values escaped (Decisions 6, 7, 13):**
     - `ensure_session` with a `cwd` that is not an existing directory asks `has-session -t =demo-c1r1` and nothing else:
       with the fake answering `can't find session: demo-c1r1` (exit 1) it is `usage`; with the fake answering exit 0 it is
       `Ok` (the session exists, nothing changes). The calls file never holds `new-session`.
     - `ensure_session` with an existing directory `<tmp>/p#S` records `-c <tmp>/p##S`; with an existing directory
       `<tmp>/q;` it records `-c <tmp>/q\;`.
     - `run` whose `argv[0]` contains `=` (the element `HLR_SENTINEL_641=s3cr3t`), with the fake answering `has-session`
       exit 0, is `usage`; the calls file holds `has-session` and no `new-window`; the message names the rule and contains
       neither `HLR_SENTINEL_641` nor `s3cr3t` (W-2). With the fake answering `can't find session` it is `pane-not-found`.
   - h. **Stop by ownership, through the kill seam (Decision 4, B-3):** `with_stop_grace(200 ms)`, a fake tmux answering
     `list-panes` call by call and a fake `kill` that also appends `${LC_ALL-unset}` to a separate env file per call.
     The first listing holds four panes (format `<pane_pid> <pane_dead> <@holler-pid>`, an untagged pane's third field empty, Q5): `101 0 101`,
     `202 0 ` (untagged), `303 0 999`, and `404 1 404` (tagged but dead, W-11). The kill record holds `-s TERM -- -101`
     only; `202`, `303` and `404` are never signalled. Every line of the env file reads `C` (W-12). Then:
     - the listing still shows `101` live (`pane_dead` 0) after the grace: the kill record gains `-s KILL -- -101`, and
       `stop_owned` is `Ok` once a later listing lacks it;
     - a listing that shows `101` with `pane_dead` 1 counts as gone: `Ok`, no `KILL` (W-8);
     - the fake `kill` answering the procps-ng 4 text `/usr/bin/kill: (-101): No such process` (exit 1; Evidence round 2,
       P7) counts as gone: `Ok`; any other `kill` stderr (exit 1) is `Unavailable`;
     - **one shared grace (W-10):** two owned panes `101 0 101` and `505 0 505`, both still live after the grace: the kill
       record holds both `TERM` entries before any `KILL`, then exactly one `-s KILL -- -101` and one `-s KILL -- -505`;
       `stop_owned` is `Ok` once a later listing lacks both, within the grace plus 1 s.
   - i. **No untagged orphan (Decision 3, W-3):** the fake answers `new-window` with `4242 @7` and then fails `set-option`:
     with `can't find window: @7` (exit 1) `run` is `Ok` and the kill record is empty; with any other stderr (exit 1) `run`
     is `Unavailable` and the kill record holds `-s KILL -- -4242`. A malformed `new-window` answer (`abc @7`, `4242 7`,
     `4242 @7x`, empty) is `Unavailable`, with no `set-option` call and no signal.
7. **No broad kill:** `grep -rnE 'pkill|killall|pgrep|pidof' crates/holler-adapter-host/src | grep -vE ':[0-9]+:\s*//'`
   prints nothing, and a default-run test `no_broad_kill_in_source` asserts the same by walking `CARGO_MANIFEST_DIR/src` at
   runtime (every `.rs` file, so a file added later is covered) and skipping `//` comment text, as
   `crates/holler-hub/tests/logging_guard_test.rs` and `crates/holler-cli/tests/hold_single_path_test.rs` do (W-4). The scope
   is production source only (`src/`); the fake tmux and fake `kill` scripts stay under `tests/` and never move into `src/`
   (outside review round 1, W-4). Signals
   go only to a process group whose leader pid tmux reports for a live window the adapter tagged (Decisions 3, 4), or to the pid
   `run` just started when tagging it failed (Decision 3).
8. **Quality gates** (the tester overlay's Tier 1, as CI runs them, W-5): `bash scripts/lint.sh` (every `#[allow]` and
   `#![allow]`, test files included, carries a trailing `// #641`; no file at 900 lines), `bash scripts/changelog-check.sh`,
   `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `rustfmt --check --edition 2021` on
   every new `.rs` file, and `cargo test --workspace` all pass; `cargo machete` finds no unused dependency; no `unsafe` in
   the crate (`grep -rn unsafe crates/holler-adapter-host` prints nothing); every touched file is under 900 lines and every
   function under clippy's 100-line threshold.
9. **Isolation of real-tmux tests:** every real-tmux test builds its host only through one test helper that creates
   `tempfile::Builder::new().prefix("hlr-tmux-").tempdir_in("/tmp")`, uses `TmuxSocket::Path(<dir>/s)` (asserting the path is
   under 100 bytes, the `sun_path` limit), and a config file in that dir containing `set -g default-shell /bin/sh` (no user
   rc). Its guard's `Drop` runs `tmux -S <dir>/s kill-server` (ignoring "no server") and then removes the dir.
   `grep -rn 'TmuxSocket::Default\|TmuxSocket::Name' crates/holler-adapter-host/tests` prints nothing. No test names a session
   outside `demo-*`.
10. **CHANGELOG:** `CHANGELOG.md` `## [Unreleased]` / `### Enhancements` gains one entry for the host adapter linking
    `[#641](https://github.com/Performant-Labs/holler/issues/641)`. It does not name a host (W-9).
11. **Argv and cwd pass exactly (real tmux, ignored; B-1, B-2):** `argv_and_cwd_pass_exactly`, built through the AC 9 helper
    (private server only):
    - `run(demo-c1r1, ...)` starts a program that writes each of its arguments to a file (for example `["sh", "-c",
      "printf '%s\\n' \"$@\" > \"$0\"", <file>, ...]`), with the argv elements `x;`, `y\;`, `;`, `rename-session`, `pwned`,
      `#{session_name}`. The file holds exactly those six elements, in order, and `list-sessions -F '#{session_name}'` still
      prints `demo-c1r1`.
    - `ensure_session` is called with `<dir>/p#S` (session `demo-c1r1`) and with `<dir>/#(touch <dir>/ran)` (session
      `demo-c2r1`), both made first with `create_dir_all`. Each session's `#{session_path}` equals its cwd exactly, and
      `<dir>/ran` does not exist after a bounded wait (poll for 1 s).
    - Each of those two sessions then gets a `run` of `["sh", "-c", "pwd -P > \"$0\"", <out file>]` (B-5); each output file
      holds exactly `fs::canonicalize` of that session's cwd (plus the newline), and `<dir>/ran` still does not exist.
12. **Targets are exact (real tmux, ignored; B-4):** `targets_are_exact`, built through the AC 9 helper:
    - ensure `demo-c1r10` and `run` `sleep 30` in it (pid P). With no session `demo-c1r1`: `ps(demo-c1r1)` is
      `PaneNotFound`, `stop_owned(demo-c1r1)` is `Ok`, and P is still in `ps(demo-c1r10)` and alive (`kill -0` with the
      `kill` binary);
    - then ensure `demo-c2r1`, so it is the newest session, and give it a window named `demo-c1r1` with `tmux -S <sock>
      new-window -d -n demo-c1r1 -t =demo-c2r1:`. `ps(demo-c1r1)` is still `PaneNotFound`, `stop_owned(demo-c1r1)` is still
      `Ok`, and P is still alive.
    - The all-digit case (P3) would need a session name outside `demo-*`, which AC 9 forbids; the same colon closes it
      (Evidence round 2, P3) and AC 6e's pin covers it.
13. **`run` works in the session's cwd (real tmux, ignored; B-5):** `run_works_in_the_session_cwd`, built through the AC 9
    helper, calls `ensure_session(demo-c1r1, <dir>/A)`, where `<dir>/A` (made with `create_dir_all`) is not the test
    process's cwd (asserted). Then `run(demo-c1r1, ["sh", "-c", "pwd -P > \"$0\"", "<dir>/cwd.out"])`; after a bounded wait
    (poll for 1 s) `<dir>/cwd.out` holds exactly `fs::canonicalize(<dir>/A)` plus the newline.

## Files

Production (crate `crates/holler-adapter-host/`):
- `Cargo.toml` (edit): `[dependencies] holler-pane = { path = "../holler-pane" }`; `[dev-dependencies] holler-pane-testkit
  = { path = "../holler-pane-testkit" }`, `tempfile = { workspace = true }`; each with the `# for ...` consumer comment the
  workspace uses. Update `description` (drop "empty in the workspace skeleton").
- `src/lib.rs` (rewrite, ~290 lines): crate docs (the error table with the three deliberate narrowings of Decision 12, the
  ownership rule, the escaping rule of Decision 13, the facts and tmux defaults of Decision 15), `TmuxSocket`, `TmuxHost`
  with its builder (including `with_kill_binary`), `impl HostPort for TmuxHost`, `run`'s cleanup on a failed tag.
- `src/exec.rs` (new, ~155 lines): run one child process with a deadline (stdin null, stdout and stderr drained on threads,
  `try_wait` polling, `Child::kill` + `wait` on the deadline), returning status/stdout/stderr or `Timeout`; spawn `NotFound`
  maps to `Unavailable`. Also the `kill -s <SIG> -- -<pgid>` call, through the configured kill binary.
- `src/tmux.rs` (new, ~215 lines): the argument lists for each tmux call (pure functions), the one escape function of
  Decision 13, parsing and validating `list-panes` / `new-window -P` output, and classifying a failed call's stderr into
  Missing / WindowGone / Other.

Tests:
- `tests/fake_tmux_test.rs` (new, ~505 lines): AC 6 (a-i) and AC 7, default run; the fake tmux (with its numbered answer
  queue) and the fake `kill` live in this file.
- `tests/real_tmux_test.rs` (new, ~385 lines): AC 1-5, AC 11-13 and the AC 9 helper, all `#[ignore]`.

Outside the crate (mechanical): `CHANGELOG.md` (AC 10) and `Cargo.lock` (the new path deps).

**Size estimate (amended):** ~660 production lines, ~795 test lines, ~1,455 in all, up ~+255 from the first brief (~+80
production: the escape, `with_kill_binary`, the cwd fallback, the tag cleanup, strict parsing, the docs; ~+175 tests:
AC 6d/6g extensions, 6h, 6i, the answer queue and fake `kill`, AC 11). Round 2 adds ~10 production lines (the colon targets,
`-c '#{session_path}'`, the shared grace, `LC_ALL=C` on `kill`) and ~95 test lines (AC 6e/6f/6h extensions, AC 11's cwd
check, AC 12, AC 13): ~670 production, ~890 tests, **~1,560 in all**. Still six crate files plus two mechanical ones, no new
file, the largest (`fake_tmux_test.rs`) ~505 lines, well under 900. One component family (one crate behind one trait), at
F's ~6-file cap and not over it: **fits one run**, no split. If F finds the fake-tmux helpers push `fake_tmux_test.rs` past
~800 lines, the fallback is a seventh file, `tests/support/fake.rs`, still one family; F journals it rather than splitting
the story.

**Blast radius:** `crates/holler-adapter-host/**` (the issue's), plus `CHANGELOG.md` and `Cargo.lock`. No other crate
changes; nothing depends on `holler-adapter-host` yet, so no existing test can break.

**Reuse map (extend, do not duplicate):**

| Object | Use | Extend or new |
|---|---|---|
| `holler_pane::HostPort` (`ports.rs:156`) | implemented as is; signature frozen | extend (implement) |
| `holler_pane_testkit::conformance::host::run_host_conformance` | the acceptance test, with a real-tmux `fresh` and guard | reuse, no copy of its cases |
| `holler_pane_testkit::host::HostOp::as_str` names (`host.run`, ...) | the `op` of every `Timeout` | reuse the strings (the adapter does not depend on the test kit at runtime; it uses the same literals, and AC 6a pins them equal by comparing with `HostOp::as_str` in the test) |
| `holler_pane::{PaneError, PaneName, Argv}` | errors, names, argv | reuse; no new error variant |
| `holler-adapter-host` skeleton | filled | extend |
| `holler-cli/src/pane/wiring.rs` | **not touched** (#649 owns it) | n/a |
| `run_probe` (`probe.rs`), a stub until #663 | not reused: its body is #663's, and its contract (a `ProbeResult`, no `PaneError`) does not fit; `exec.rs` stays private to this crate. Follow-up (W-7): #663 should expose its bounded runner from `holler-pane` (or say why it cannot), so this adapter can switch `exec.rs` to it and the two runners do not drift; the orchestrator files that follow-up | new, justified |
| existing launcher | none in this repo (see Problem) | n/a |

## Decisions already made (O)

1. **Type and constructor.** `pub struct TmuxHost` (plain data, so `Send + Sync`), `pub enum TmuxSocket { Default,
   Name(String), Path(PathBuf) }`, `TmuxHost::new(socket)`; builder methods `with_tmux_binary(PathBuf)` (default `tmux`
   from `PATH`), `with_kill_binary(PathBuf)` (default `kill` from `PATH`; every signal the adapter sends goes through it, B-3),
   `with_config(PathBuf)` (adds `-f`), `with_timeout(Duration)` (default 10 s, I5), `with_stop_grace(Duration)`
   (default 2 s). #649 will call `TmuxHost::new(TmuxSocket::Default)`; #667 uses `Path`.
2. **One deadline per method.** Each port call computes one deadline (`now + timeout`) and every tmux or `kill` subprocess it
   spawns gets the time remaining; on expiry the child is killed and reaped and the method returns `Timeout { op:
   "host.<method>" }`. A hung call is reported, not waited on. The one exception is the cleanup `kill` of Decision 3, which
   gets at least 250 ms.
3. **Ownership = what `run` started, by recorded pid.** `run` executes `new-window -d -P -F '#{pane_pid} #{window_id}' -c
   '#{session_path}' -t =NAME: -- env -- <argv...>` (each element escaped, Decision 13). `-c '#{session_path}'` is a
   constant format the adapter authors (Decision 13: not escaped). tmux expands it once, against the target session, and
   does not expand the result again (P6). Without it, the program works in the tmux client's cwd, which is the CLI's or the
   hub's, not `host.cwd` (P5; B-5). `run` parses its output strictly (`<u32> @<digits>`, else
   `unavailable` with no further call), then runs one tmux invocation `set-option -w -t <window_id> @holler-pid <pid> ;
   set-option -w -t <window_id> remain-on-exit off` (the `;` is its own element, authored by the adapter; both values are
   literal; probed, see Evidence). The two invocations stay separate: chaining the tag onto `new-window` tagged nothing (K2).
   Spelled out (outside review, round 1, B-2): a successful `run` spawns exactly **two** tmux subprocesses, (1) `new-window`
   and (2) the tag. The tag is one subprocess whose argv chains its two `set-option` commands with the `;` element; that
   form, with literal values, is the one O probed as setting both options (Evidence, "(O)" line under K2). What K2 showed
   failing is a different form: the tag chained onto `new-window` in the same subprocess, reading `#{pane_pid}` by format.
   So the tag is never merged into the `new-window` subprocess, and its two `set-option` commands are one subprocess, not two.
   `remain-on-exit off` on the tagged window means a stopped run never lingers as a dead pane, whatever the operator's global
   setting (W-8). The record lives in tmux (not in a file,
   I6; the adapter is rebuilt per CLI run so it cannot keep memory). `stop_owned` stops exactly the panes of the session whose
   `#{@holler-pid}` equals their live `#{pane_pid}`; the shell window `ensure_session` created, and anything a person opened, is
   never signalled, so the session (and a Herdr pane attached to it) survives a stop. If the window is already gone when
   `set-option` runs (`can't find window`: the program exited at once), `run` still returns `Ok`. **Any other `set-option`
   failure** (another stderr, or `Timeout` because the deadline ran out between the two calls) first sends `kill -s KILL --
   -<pid>` to the pid `new-window` printed, through the kill binary, then returns the error, so no untagged process that
   `stop_owned` could never stop is left behind and #644's rollback and retry do not start a second copy (W-3). That cleanup
   `kill` gets the time remaining or 250 ms, whichever is longer, so a method may overrun its timeout by at most 250 ms
   (inside AC 6a's 1 s slack).
4. **How it stops.** `stop_owned` lists `list-panes -s -t =NAME: -F '#{pane_pid} #{pane_dead} #{@holler-pid}'` (B-4: the
   colon; every poll below uses the same target; an untagged pane's third field is empty, Q5). A pane is **owned** when its
   `@holler-pid` equals its `pane_pid` **and** `pane_dead` is 0. A tagged pane listed with `pane_dead` 1 is not owned and is
   never signalled: its pid may already belong to another process (W-11, R2). The stop runs in two phases inside Decision
   2's one deadline (W-10), never one cycle per pid:
   - send `kill -s TERM -- -<pid>` to **every** owned group first (the pane child is a session and group leader, PGID ==
     pid, so its children go too);
   - poll `list-panes` every 50 ms until every one is gone or the **one shared** grace ends;
   - send `kill -s KILL -- -<pid>` to every survivor, then poll until all are gone or the deadline passes (`Timeout`).

   A pane counts as **gone** when it is no longer listed **or** is listed with `pane_dead` 1 (W-8: an operator's
   `remain-on-exit on` must not make every stop wait out the grace). `kill` is spawned with `LC_ALL=C` (procps-ng `kill`
   is localized; tmux's messages are not) and only its stderr containing `No such process` counts as gone (the procps-ng 4
   text is `/usr/bin/kill: (-<pgid>): No such process`, P7); any other `kill` failure is `unavailable` (W-12). The `kill`
   binary (Decision 1's `with_kill_binary`) is used instead of `libc::kill` so the crate stays free of `unsafe` and of new
   dependencies. Nothing matches by name.
5. **`ps`** = `list-panes -s -t =NAME: -F '#{pane_pid} #{pane_dead}'` (B-4: the colon makes the session target exact;
   P2-P4): the leader pid of every live pane of the session
   (the shell included; descendants not listed), as the port fixes no order. Untagged panes count: `ps` reports what runs
   there; `stop_owned` decides what to stop.
6. **`ensure_session`.** With a `cwd` that is an existing directory (checked on the unescaped value): `new-session -d -s NAME
   -c <cwd escaped per Decision 13>`; `duplicate session` is `Ok` and changes nothing (one call, race-free). With a `cwd` that
   is not an existing directory, the cwd matters only if the session would be created (W-1a): the adapter asks `has-session
   -t =NAME`; an existing session is `Ok` and changes nothing (as the fake does, so a launch onto a leftover session, Decision
   15, does not exit 2); a missing session (or no server) is `usage`, and `new-session` is never called (tmux would start the
   session in a missing directory with exit 0, E5).
7. **`run` order** matches the fake: a missing session first (`pane-not-found`), then an empty argv (`usage`). With an empty
   argv, or an `argv[0]` containing `=`, the adapter asks `has-session -t =NAME` to tell the cases apart; otherwise
   `new-window` itself reports the missing session. `argv[0]` containing `=` in an existing session is `usage` (env would
   treat it as an assignment). The message names the rule ("argv[0] contains '='") and never echoes the element, which has
   the shape a `NAME=value` secret arrives in (W-2; as `holler-pane/src/argv.rs` does).
8. **Error mapping** (the crate docs carry this table): stderr containing `can't find session`, `can't find window`, `no
   server running`, or `error connecting to` with `No such file or directory` or `Connection refused` means **missing**
   (`pane-not-found` for `run`/`ps`, `Ok` for `stop_owned`); a spawn `NotFound` or any other failure is `unavailable` with the
   first stderr line. `Unavailable.what` is tmux's stderr line (or, for a spawn failure, the binary's path), never an argv
   element or the cwd (W-2).
9. **Environment.** Every spawned `tmux` has `TMUX` and `TMUX_PANE` removed, so a hub or test running inside a tmux pane never
   addresses that pane's server by inheritance; only the configured socket is used (with `TmuxSocket::Default` that means
   tmux's own default socket, never `$TMUX`'s). AC 6f checks it with a re-executed child test, not `set_var`. **No variable
   is ever added to a tmux subprocess's environment** (W-12): a tmux client's environment reaches every pane started later
   (R1), so removing `TMUX` and `TMUX_PANE` stays the only change. In particular `LC_ALL` is never set on tmux; only the
   `kill` subprocess gets `LC_ALL=C` (Decision 4).
10. **Exact targets (B-4).** Every target that names the session is exact, in one of two forms set by the command's target
    type. `has-session` takes `-t =NAME` (a session target). Every other command takes `-t =NAME:`: `new-window`, every
    `list-panes` (in `ps` and in `stop_owned`'s listings and polls), and any command added later. A `=NAME` without the
    colon, given to a window or pane target, is not exact: tmux tries a window index, then a window name in the current
    session, then the session with prefix matching (Evidence round 2, P2-P4). A bare name is never used (tmux
    prefix-matches it: `has-session -t demo` succeeded for `demo-c1r1`). Pane names are `[a-z0-9-]` (ADR 0005), so no
    further escaping of names is needed (values
    the adapter did not author are escaped per Decision 13).
11. **Tests, not CI.** Real-tmux tests are `#[ignore]` per the issue and are run by T-green and S with `-- --ignored` on a
    machine that has tmux (the pipeline host has 3.7c). Adding them to CI needs an edit to `.github/workflows/ci.yml`, outside
    this blast radius: a follow-up for the operator, not this story.
12. **ADR-0021 unchanged; three deliberate narrowings.** None changes a type or method; each is recorded in the crate docs'
    error table and in `decisions.md` (W-1):
    - ownership (3) narrows "stop the processes the session owns" (also in the CHANGELOG);
    - a `cwd` that is not an existing directory is `usage` when the session would be created; the fake would create it (6);
    - an `argv[0]` containing `=` is `usage`; the fake would run it (7).
    `FakeHost` is not changed (outside this blast radius); parity is a follow-up only if the operator wants it.
13. **Escaping at tmux's command line (B-1, B-2).** tmux splits its argument vector into commands at every element that ends
    in `;`, before option parsing and after `--` too, and turns a trailing `\;` into `;` (E1, R1; `man tmux`, PARSING
    SYNTAX). It also format-expands the `-c` start directory (`#S`, `#{...}`, and `#(cmd)`, which runs `cmd` through the
    server's shell; E3, E4). So:
    - `tmux.rs` passes every value the adapter did not author through **one pure escape function** before it enters a tmux
      argument vector: each argv element after `--`, and the cwd. A value that ends in `;` gets a `\` inserted before that
      final `;` (`x;` -> `x\;`, `y\;` -> `y\\;`, `;` -> `\;`); nothing else changes (R1: `{`, `}`, `#{...}`, `~`, `-t`, `a b`,
      `""` pass through literally).
    - The cwd first has every `#` doubled (`##` is a literal `#`), then gets the same escape. Decision 6's directory check
      runs on the unescaped value.
    - Constant formats the adapter writes (`-F '#{pane_pid} #{window_id}'`, `run`'s `-c '#{session_path}'`, the list-panes
      formats) and the `;` it authors
      between its own commands (Decision 3) are not escaped.
    - Values read back from tmux and reused in a command (the pid and the window id) are parsed and validated first: the pid
      as `u32`, the window id as `@` followed by one or more digits.
14. **Test safety: no signal to a stranger (B-3).** No default-run test lets the adapter signal a pid the test did not spawn.
    A test that can reach a signal uses a fake `kill` (via `with_kill_binary`) that only records its arguments. The pipeline
    host runs the live fleet; a canned `list-panes` answer names whatever process group happens to hold that number there.
    Real-tmux tests (`#[ignore]`) use the real `kill`, but only on processes the test started on its private server (AC 9).
15. **Facts consumers must know; tmux defaults assumed (W-6, W-8).** Recorded in the crate docs and `decisions.md`:
    - `run` creates its window detached (`-d`), so a client attached to the session keeps showing the shell window.
    - The session and its shell window survive `stop_owned`, and `HostPort` has no call that ends a session. So `close`
      (#646) leaves the tmux session behind (the next launch of that name reuses it, Decision 6), and `ps` is never empty
      while the session exists.
    - Processes the external launcher started before cutover are untagged, so `stop_owned` never stops them (#650, #654:
      the first relaunch of an imported pane).
    - tmux defaults the design assumes: `exit-unattached off` (else a server that `new-session -d` just started ends at
      once), `exit-empty` irrelevant while a session exists, and `remain-on-exit` handled per window (Decisions 3, 4).
    - A session keeps the directory it was made in (B-5). `ensure_session(name, B)` on a session made in A changes nothing
      (Decision 6, as the fake), so a later `run` still works in A. A relaunch after `host.cwd` changes runs in the old
      directory until the session is ended, and no port call ends one (#644, #646).
    - A program that exits before its tag lands, under an operator's global `remain-on-exit on`, leaves a dead tagged window
      (R2): its own `remain-on-exit off` does not close a pane that is already dead. It is never signalled (Decision 4) and
      stays until its session ends (W-11).
    - After `run` returns `Timeout` from `new-window` itself, tmux may still create the window and start the program after
      the adapter killed its client: an **untagged** process may then run in the session. `ps` lists it and `stop_owned`
      never stops it; the adapter has no pid for it and never matches by name (W-13).

Forward-compat (consumers of this crate):

| Consumer | Needs | Satisfied |
|---|---|---|
| #644 launch/relaunch | `ensure_session` then `run(command argv)`, the harness working in `host.cwd` (`-c '#{session_path}'`, B-5); relaunch = `stop_owned` then `run` without killing the session; a `run` whose tag fails leaves no untagged process for a rollback to miss | yes (3, 6), with two limits documented in 15: a session keeps the directory it was made in, so a relaunch after `host.cwd` changes runs in the old one until the session is ended; and a `Timeout` of `new-window` itself may leave an untagged process that `stop_owned` never stops (W-13) |
| #646 close | `stop_owned` that is `Ok` on a crashed/missing session | yes (8) |
| #646 close | to know the tmux session and its shell survive (no port call ends a session), and `ps` is not empty after a close | documented (15); ending the session is #646's call to make |
| #650 / #654 import, first relaunch of an imported pane | to know that processes started before cutover are untagged, so `stop_owned` does not stop them | documented (15); handling them is #650/#654's |
| #647 reconcile/doctor | `ps` as observed state, `pane-not-found` for a session gone | yes (5, 8) |
| #649 wiring | a constructor with no I/O, `Send + Sync`, the default socket | yes (1) |
| #667 apply scenario | the adapter on a scratch tmux server | yes (`TmuxSocket::Path`/`Name`) |
| #642 OpenCode adapter | none: `HarnessPort::serve` returns its own pid; the host adapter does not stop by port | n/a |

## Out of scope

- Wiring the adapter into the CLI (`wiring.rs`, #649); any verb (#644, #646, #647).
- Stopping by harness port (the issue's "pid and port"): `HostPort::stop_owned` takes only a name, and the server's port is
  `HarnessPort`'s (#642). Recorded pids are the only handle here.
- A tmux version check, remote hosts, editing ADR-0021 or the test kit, CI changes.
- Touching any live tmux server, the default socket, the operator's sessions, Herdr or OpenCode.

## Follow-ups (not this story)

- #663: expose its bounded subprocess runner from `holler-pane` (or record why it cannot), so this adapter's `exec.rs` can
  switch to it (W-7). The orchestrator files this as an issue.
- CI for the real-tmux tests (Decision 11), and `FakeHost` parity for the two narrowings (Decision 12), both the operator's
  call.

## Test plan

RED (W-5; a compile error is not RED, `docs/agent-overlays/tester.md`): **the allowed approach** is that T, before writing the
tests, lands stub files with the exact public signatures of Decision 1 and no logic: `src/lib.rs` with `TmuxSocket`,
`TmuxHost`, `new` and every `with_*` builder method (returning `self`), and `impl HostPort for TmuxHost` whose four methods
return `PaneError::NotImplemented`, plus the `Cargo.toml` dependencies. Then every default-run test fails on its assertion
and every real-tmux test fails on `not-implemented`. F replaces the stubs. T journals the RED run with the exact failures.
GREEN: `cargo test -p holler-adapter-host` (default) and `cargo test -p holler-adapter-host -- --ignored` (real tmux,
private sockets), then the AC 8 gates. T-green also confirms no `hlr-tmux-*` dir is left in `/tmp` and no `demo-*` session
exists on the default socket (`tmux list-sessions` on the default socket is read-only and must not list one; if no default
server runs, that is fine).

## Risks

- **tmux re-parses its own command line (B-1, B-2).** An argv element ending in `;` ends the tmux command, and the elements
  after it run as tmux commands: on tmux 3.7c, `run`'s exact command line renamed the session (E1), and `run-shell` in that
  position would run a shell. A `#(...)` in an existing cwd ran a shell command through the tmux server (E4). Decision 13's
  escape closes both; AC 6d and 6g pin the escaped vectors and AC 11 proves them on real tmux. A refactor that builds a tmux
  vector without the escape function reopens both.
- **Signals in default-run tests (B-3).** A fake `list-panes` answer names a real process group on the machine running the
  tests. Decision 14 and `with_kill_binary` keep every default-run signal inside a recording fake.
- **tmux single-argument shell path.** tmux runs a one-argument command through `sh -c`; the `env --` prefix (AC 6d) is what
  keeps "never through a shell" true. A refactor that drops it reopens injection; AC 6d pins it.
- **Prefix matching** would let `stop_owned(demo)` hit `demo-c1r1`. A `=NAME` without the colon on a window target is not
  exact either (B-4): `stop_owned(hj-c1r1)` after that session crashed would signal `hj-c1r10`'s harness. AC 6e pins both
  the `=` and the colon per command, and AC 12 proves the prefix and window-name cases on real tmux.
- **Working directory (B-5).** Without `-c '#{session_path}'` every harness would start in the CLI's or hub's cwd. AC 6d
  pins the flag and AC 11 and 13 read the started process's `pwd -P`.
- **Socket path length:** `sun_path` is ~104-108 bytes; a long `TMPDIR` gives "File name too long". The test helper uses
  `/tmp` and asserts the length (AC 9).
- **Pid reuse** between `list-panes` and `kill` is a window of milliseconds; tmux's live `pane_pid` plus the matching
  `@holler-pid` makes a stale target unlikely. That holds only because a dead tagged pane (`pane_dead` 1, whose pid may be
  reused for as long as its window lingers, R2) is never signalled (Decision 4, AC 6h pane `404`). Accepted.
- **`kill` binary portability** (`kill -s TERM -- -PGID`): verified on Linux procps; macOS BSD `kill` accepts the same form.
  Real-tmux tests are opt-in, so a macOS difference cannot break CI.
- **Shell rc in sessions:** production sessions use the operator's default shell; tests force `/bin/sh` through the config
  file so they do not depend on a user's rc files.
