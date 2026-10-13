# Claude Code pane spike: session of record, delivery, interrupt, observation (issue #830)

Verify-first spike for epic [#633](https://github.com/Performant-Labs/holler/issues/633) (decision 9, amended
2026-10-11), story [#830](https://github.com/Performant-Labs/holler/issues/830). The question is whether a Claude Code CLI
session in a pane can be launched with a chosen session, driven by the hub without anyone typing into the pane,
interrupted, and observed, so that `holler pane` can launch and register Claude Code panes in the same registry as
OpenCode panes. The note ends with a recommended design, the contract changes it needs, and the test-kit fakes. No
contract changes are made here (amend-first rule); story #831 carries them.

- **Claude Code version probed:** `claude --version` → `2.1.296` (Linux x86-64), probed 2026-10-11. Subscription login
  (`claude.ai`), smallest model (`haiku`).
- **Probes:** `scripts/spikes/claude-session.sh` (questions 1, 5, 6), `claude-drive.sh` (2), `claude-interrupt.sh` (3),
  `claude-observe.sh` (4), with shared helpers in `claude-lib.sh` and three small helpers they start: `claude-channel.mjs`
  (a hand-written channel MCP server), `claude-stream.py` (a stream-json driver), `claude-hook.sh` and
  `claude-abort-hook.sh` (hook commands). Each probe prints one `PASS` / `FAIL` / `INFO` line per observation and
  exits 1 on any `FAIL`, so story #831 can lift them into contract tests. Re-run them whenever Claude Code is upgraded:
  most of the delivery path below rests on behaviour the client does not document.
- **Needs:** `claude`, `jq`, `tmux`, `setsid`, `timeout`, `uuidgen`, `python3`, `node` (Linux). `CLAUDE_BIN` picks another
  binary, `CL_MODEL` another model. A run makes about 20 one-line model turns in total.

## How the probes stay away from a real fleet and a real login

Unlike the OpenCode spike, these probes cannot give the process a scratch `HOME`: the subscription login lives under
the config directory in `HOME`, and a scratch `HOME` has no login. So the guarantees are different, and these are the
ones that hold (all in `claude-lib.sh`):

- **The login is never read, copied, printed or moved.** The only thing asked of it is `claude auth status`, and only its
  `authMethod` field is looked at. The login file's *name* and mode are all a probe says about it.
- Every Claude process runs under `env -i` with only `HOME`, `PATH`, `TERM` and `LANG`. No `CLAUDE_CODE_*` variable, no
  token and no API key from the caller reaches it. Where a probe shows what a variable does, it sets that one variable
  by name to a **dummy** value (`authMethod` is read back; nothing is sent).
- The working directory is a fresh `mktemp -d` directory, so each session's transcript lands in a project directory of
  its own, and the `EXIT` trap removes exactly that project with `claude purge <scratch dir>`.
- `--setting-sources project,local` keeps the operator's user settings (hooks, plugins, MCP servers, permission mode) out
  of every scratch session. A probe's own hooks and MCP config come in through `--settings` / `--mcp-config`.
- Every listing of running sessions is filtered with `--cwd <scratch dir>` (and then by the pane's pid), so a probe never
  reads, counts or prints anybody else's session.
- The TUI runs on a private tmux socket (`tmux -S`), never the default tmux server. The one probe that uses `--bg`
  refuses to run when a background-session supervisor is already up, and otherwise stops the one it started.
- A keystroke is sent to a scratch TUI only to answer a first-run dialog or to do something a person at the keyboard
  would (type `/clear`). Nothing the adapter would do is typed (invariant I4).
- Public repository: no host, user, tailnet, pane or home-directory names are in the probes or in this note. `<config
  dir>` below stands for the Claude Code config directory under `HOME`; `<scratch-dir>` for a probe's temp directory.

**Evidence labels used below.** **Verified**: observed by running the probes (each probe was run again in its final form
and passed). **Documented**: stated on `code.claude.com/docs` on 2026-10-11 and not run. **Read**: taken from
`claude --help`, the client's own `--debug-file` log, or its bundled code, and not an interface the vendor documents, so
it is pinned to the version above. **Inferred**: a conclusion drawn from the other kinds and not tested. **Not tested**
says so.

## Summary

**A Claude Code pane is one process, not a server and a TUI.** The interactive `claude` process is the harness, the
TUI and the session holder at once; it listens on a per-process unix socket and lists itself with `claude agents
--json`. Everything below follows from that.

| Question | Answer | Evidence |
|---|---|---|
| 1. Session of record | Yes. `--session-id <uuid>` chooses it; `--resume <uuid>` relaunches exactly it (new process, same id, no new transcript). `claude agents --json` gives `sessionId` per process, which is "the session this pane shows", and follows a person's `/clear` or `/resume` in the pane. Two limits: an id with no transcript yet (never prompted) cannot be `--resume`d, and `--bg` ignores `--session-id`. | Verified |
| 2. Delivery without typing | **The session's inbox socket**: write one JSON line `{"type":"user","message":{"role":"user","content":TEXT}}` to it. The TUI stays usable. The pane must be launched with `crossSessionInbound: accept` in `--settings`, or a pane that bypasses permission prompts holds every message behind a dialog. The model sees it framed as a peer session's message, not the user's. | Verified; frame **Verified** 2026-10-13 (real-binary test), framing **Read** |
| 3. Interrupt / abort | **No in-place, no-typing interrupt exists for a TUI pane.** A signal is not one (SIGINT ends the process). The socket takes no interrupt frame. What works: stop the process and relaunch `--resume` (loses the in-flight turn, keeps the session), or a hub-controlled `PreToolUse` hook that ends the turn at the next tool boundary. A hub-owned `-p` process has a real `interrupt` request, but no TUI. | Verified |
| 4. Observation | `claude agents --json` (about 90 ms a call) gives `idle` / `busy` / `waiting` with `waitingFor` (`permission prompt`, `input needed`), and drops a dead process. Hooks add events. A wedged or four-hours-busy process has no heartbeat: the hub must timestamp its own observations. | Verified |
| 5. Login | The process needs `HOME` (the config directory with the login) and nothing else. Env var **names** that change the login silently: `ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CONFIG_DIR`, the cloud-provider switches. | Verified (names); the rest **Documented** |
| 6. Where it runs | A normal child of the pane's tmux session; `HostPort::run` / `ps` / `stop_owned` fit. SIGTERM stops it cleanly (transcript kept, registry entry removed). Do not use `--bg`: it puts the process under a second supervisor outside tmux. | Verified |

**Recommendation: make `ClaudeCode` a `HarnessPort` implementation, with three contract changes that matter and a few
small ones** (full list in [Contract changes](#contract-changes-for-story-831)):

1. The port is keyed by a server `port: u16` everywhere (`HarnessInfo.port`, `health`, `create_session`, `abort`,
   `send_prompt`, `session_activity`). A Claude Code process has no port. Replace the key with an endpoint type that is
   `Port(u16)` for OpenCode and a process identity for Claude Code.
2. The port assumes a server plus a TUI that attaches to, and switches between, sessions. Claude Code has neither seam:
   a session switch or an abort is a relaunch of the same process. Add a harness *shape* the launch/switch/reset/interrupt
   verbs ask for, so they relaunch where an in-place call does not exist.
3. `Activity` has no permission wait, and `abort` has no honest meaning in place. Add the variant and an abort style.

Decisions that I took, and the one that is the operator's to confirm, are in [Decisions](#decisions).

## 1. The session of record

All **Verified** by `claude-session.sh` unless noted.

| Observation | Result |
|---|---|
| `claude --session-id <uuid>` | The process holds that id. `claude agents --json` reports it as `sessionId`; so does the per-process registry file (internal, below). |
| When the transcript appears | **Not until the first prompt.** `<config dir>/projects/<cwd with every non-alphanumeric character replaced by "-">/<id>.jsonl` (the rule is **Documented**; the path was checked). A launched, never-prompted session has no transcript. |
| A second launch with an id that is in use | `Error: Session ID <id> is already in use.`, exit 1. |
| A launch with an id whose transcript exists, nothing running | The same message, exit 1. So **`--session-id` is for the first launch only.** |
| `--resume <id>` after the process was stopped | A new process (new pid) holding the same id. The pane shows the earlier conversation. No second transcript file. |
| `--resume <unknown id>` | `No conversation found with session ID: <id>`, exit 1. |
| A chosen id that was never prompted | `--resume <id>` → `No conversation found…`, exit 1. A second `--session-id <id>` is **accepted** and holds that id. So a relaunch picks `--resume` when the transcript file exists and `--session-id` when it does not. |
| A person types `/clear` in the pane | The same process now holds a **different** `sessionId` in `agents --json`. A person's `/resume` does the same (**Documented**). |
| `claude --bg … --session-id X` | `warning: --bg manages the session id; ignoring --session-id`. The id is assigned; it is found afterwards in `agents --json`. |

**What Holler reads as "the session this pane shows".** `claude agents --json` is the documented interface for scripts
(it "does not require a TTY"): each running process is an entry `{cwd, kind, name, pid, sessionId, startedAt, status}`
(`waitingFor` when waiting; background sessions add `id` and `state`). Match the entry by the pane's `pid`
(`HostPort::ps`), never by `cwd`: several panes can share a directory. The same value is in the `session_id` field of
every hook payload and in the `system/init` event of a `-p` run (**Documented**).

The per-process registry file `<config dir>/sessions/<pid>.json` has more keys (`messagingSocketPath`, `peerProtocol`,
`peerFeatures`, `statusUpdatedAt`, `updatedAt`, `procStart`, `version`) but the docs call the files under the config
directory not a stable interface. **Do not read it.** After a SIGKILL it is left behind while `agents --json` has
already dropped the entry (it checks the pid), so the file alone would report a dead pane as alive.

## 2. Delivering a message without typing

Candidates, what each does to the TUI, and what each costs. A, B and F are the ones with a real contender.

| | Way | TUI stays usable | Nobody types | Unattended (re)launch | Cost / limit | Verdict |
|---|---|---|---|---|---|---|
| A | **Inbox socket** (a unix socket every session binds) | yes | yes | yes, with `crossSessionInbound: accept` | frame is not a documented interface; peer framing; a socket write | **Use** |
| C | Channel (an MCP server the session loads; `notifications/claude/channel`) | yes | yes | **no**: `--dangerously-load-development-channels` shows a warning dialog that needs a key at every launch | research preview, flag hidden from `--help`, needs `claude.ai` login, an MCP server process per pane, org policy can block it | Reject for now |
| D | `claude -p --resume <id>` against the TUI's session | **no**: the TUI keeps its own view and does not show the new turn | yes | yes | two writers on one session: the SHOWN/DRIVEN drift the epic exists to remove | Reject |
| E | Hub-owned `claude -p --input-format stream-json` process | there is no TUI | yes | yes | right for a *headless* body, not a pane; a TUI opened on the same id is a second writer | Reject for panes |
| F | `--bg` plus `claude --resume <id> "prompt"` | yes (`claude attach`) | the prompt needs a **tty** and no config flags | id cannot be chosen; a second process supervisor outside tmux | Reject |
| G | ACP adapter (`@agentclientprotocol/claude-agent-acp`, a wrapper over the Claude Agent SDK) | no TUI | yes | yes | not a pane; it is the ADR-0012 route for a headless Claude *body* (a `command`) | Out of scope here |

### A. The inbox socket (Verified; frame **Verified** 2026-10-13)

- Every interactive session on macOS and Linux binds a unix socket, restricted to the OS user (mode `600`, in a private
  per-user directory; observed `/tmp/cc-socks/<pid>.sock`, the docs name `/tmp/cc-socks-<uid>` as the fallback). The
  docs say a script or hook may post into it, with an optional `{"type":"auth","token":"…"}` first line on Linux.
- The client prints the working frame in its own `--debug-file` startup log: `{"type":"user","message":{"role":"user",
  "content":"hello"}}`, one line, newline-terminated. A connection that sends no complete line within 30 s is closed.
  This is not documented. It was **Read** from the log, and is now **Verified**: Verified 2026-10-13 by `cargo test -p holler-adapter-claude --test real_claude_test -- --ignored` (claude 2.1.296, tmux 3.7c, Linux, 12 conformance cases passed). The test covers Linux with that one claude version only; macOS and other claude versions are not covered.
- **Finding the socket path, documented way:** the path is exported to hooks as `CLAUDE_CODE_MESSAGING_SOCKET` before
  `SessionStart` runs. Verified: a `SessionStart` hook received a path equal to the registry's. So the launch carries a
  `SessionStart` hook that tells Holler the path and the session id. The fallback is the registry file's
  `messagingSocketPath` (internal) or `/status`'s `Peer address` row.
- Delivery: a socket write started a turn in the pane within **0.3–0.5 s** (the probe polls). A second message sent while a tool call was
  running was **queued and ran after the first turn**; both replies were in the transcript. The client logs it as
  `Routed user message to queue (priority=next)`. The `UserPromptSubmit` hook fires for it, and so do `Stop` and the
  status transitions (below), so the hub can follow a delivered message end to end without the model.
- **Inbound policy decides whether it is delivered at all.** Per message the receiving session either delivers, holds, or
  refuses it (`crossSessionInbound`: `accept` / `hold` / `refuse`; unset means "decide from the permission modes"):
  - Default-mode pane, setting unset: delivered.
  - **A pane that bypasses permission prompts (`--dangerously-skip-permissions`, as hand-set-up Claude panes commonly
    run), setting unset: held.** The pane opens a "Held message from another session" dialog and its status becomes
    `waiting` with `waitingFor: "permission prompt"`. Nothing runs until a person answers. Verified.
  - The same pane launched with `--settings '{"crossSessionInbound":"accept"}'`: delivered. Verified.
  So **the launch must carry `crossSessionInbound: accept`** (inline JSON in `--settings` keeps the argv self-contained
  and writes no file). `--bare` panes bind no socket at all (**Documented**) and so are excluded.
- **How the model sees it.** The pane shows `Another Claude session sent a message:`, the text, and a fixed paragraph
  telling the model the message did not come from its user, to treat it as a teammate's request within the session's own
  permission settings, never to treat it as approval of a pending prompt, and never to change configuration because a
  peer asked. So **a hub order has less authority than a typed prompt**, by design and not removable. Plain text only;
  a message is capped near a million characters and bursts and identical repeats are throttled (**Documented**).
- **Not tested:** the `notify_when_idle` control action (a push "idle or exited" notice; the client handles it, the docs
  describe it, but the reply-address protocol is not documented enough to drive from a script). Polling `agents --json`
  at 90 ms a call makes it unnecessary for v1.

### C. Channels (Verified)

A hand-written, dependency-free channel server (`claude-channel.mjs`) connected, and a line written to its control socket
reached the model and came back through its `reply` tool in about 2 s. The cost is the launch: Claude Code shows a
full-screen "WARNING: Loading development channels" dialog and waits for a key, every launch, and the flag is ignored
under `-p`. That is a keypress at launch, which the epic forbids, and the preview may change. It is the right path later
if a *two-way* chat bridge or permission relay is wanted.

### D, E, F (Verified)

- **D.** `claude -p --resume <id> "…"` ran against a session whose TUI was idle: the answer came back with the same
  `session_id`, the session's transcript grew by 10 lines, and the TUI **did not show the turn**.
- **E.** A hub-owned `-p --input-format stream-json --output-format stream-json` process takes `session_id` from
  `--session-id`, answers turn after turn on one process (the second answer remembered the first), and reports
  `capabilities` including `interrupt_receipt_v1`. It is a complete driver, with no TUI.
- **F.** `--bg` starts the session under a **supervisor** ("Starting background service…"), not in the caller's tmux. Sending
  a prompt to the running background session with `claude --resume <id> "…"` fails without a terminal:
  `Error: That session is running in the background (<id>). Run claude attach <id> …`. With a pty and **no** flag that
  configures the session it attaches and delivers the prompt as the next turn; closing that terminal leaves the session
  running. `claude logs <id>` prints the session's terminal output as a screen dump with escape sequences.

## 3. Interrupt and abort

All **Verified** by `claude-interrupt.sh`.

| What | Result |
|---|---|
| `SIGINT` to the process, mid-turn | **The process exits** (209–615 ms). Its tool child dies with it, the transcript is kept, the in-flight turn writes no answer. A signal is not the Ctrl-C key the TUI handles. |
| `SIGINT` at an idle prompt | The process exits too. **Never use a signal as an abort.** |
| Inbox socket frames `control_request` / `interrupt` / `abort` / `control` + `action: interrupt|abort` | All classified `unhandled` by the client. The socket's control actions are `notify_when_idle` and artifact-reply yield/unyield (**Read**). There is no interrupt over the socket. |
| Stop the process, relaunch `--resume <id>` | The resumed process holds the same session, is idle (the cut-off turn is **not** re-run unless `CLAUDE_CODE_RESUME_INTERRUPTED_TURN` is set, **Documented**), and the next socket message starts a turn normally. |
| `PreToolUse` hook that answers `{"continue":false,"stopReason":…}` while a hub-owned flag file exists | The turn ended at the **next tool boundary**: three sequential `sleep 6` calls, flag raised during the first, the second was refused, the third never ran, and the pane showed `PreToolUse:Bash hook stopped continuation: stopped by the hub`. The process and session stayed. The turn ended about 15 s after the flag (the running tool's remaining time plus a model round trip, plus the probe's own polling). It cannot stop a tool that is already running or a turn that is only generating text. |
| Hub-owned `-p` process, `{"type":"control_request","request":{"subtype":"interrupt"}}` on stdin | A `control_response` (`success`, `still_queued: []`) came back at once (the request was sent 4 s into the turn), the turn ended as `error_during_execution` 0.8 s later, and the **next message on the same process and session was answered**. This is the only true in-place interrupt, and it needs Holler to own the process's stdin (no TUI). |
| A `PermissionRequest` hook answering `deny` with `interrupt: true` | **Documented**, **not tested**: stops Claude at a pending permission prompt only. |
| The Esc / Ctrl-C key typed by `HerdrPort::send_keys` | **Not tested** on purpose: it is typing into a pane (I4). It is the only in-place interrupt of a TUI and would work; see Decision 2. |

So `HarnessPort::abort` for a TUI pane is, honestly, **a relaunch**. The hook gate is an optional "stop after this step",
not an abort.

## 4. Observation

All **Verified** by `claude-observe.sh` (a timeline of one session joined with its hook events).

| State | `claude agents --json` | Hook event | The screen (fallback) |
|---|---|---|---|
| idle | `status: idle` | `Stop` ends a turn | footer `? for shortcuts` |
| working | `status: busy` | `UserPromptSubmit` starts it | footer `esc to interrupt` |
| waiting, permission | `status: waiting`, `waitingFor: "permission prompt"` | `PermissionRequest` (tool name and input), `Notification` | dialog `Do you want to proceed?` … `Esc to cancel · Tab to amend` |
| waiting, question | `status: waiting`, `waitingFor: "input needed"` | `PermissionRequest` for `AskUserQuestion` | `Enter to select · ↑/↓ to navigate · Esc to cancel` |
| held peer message | `status: waiting`, `waitingFor: "permission prompt"` | none | "Held message from another session" dialog |
| dead | the entry is **absent** | `SessionEnd` fires on a clean stop (tmux closing the pane), **not** on SIGKILL | the tmux pane is gone |

The docs list more `waitingFor` values (`sandbox request`, `worker request`, `dialog open`; **Documented**, not seen).
`dialog open` is the one a first-run dialog would show.

- **Latency.** One `agents --json --cwd …` call takes about 90 ms. `status` changed within 100–300 ms of the event in
  every timeline (busy 277 ms after a socket write, a permission wait 1.3 s after the prompt, which is the model's own time).
- **The hub can answer a permission prompt without typing.** A `PermissionRequest` command hook that waits for a hub-owned
  file and prints `{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}` ended the
  wait 684 ms after the file appeared, the command ran, and nobody pressed a key. The local dialog and the hook are both
  live; whichever answers first wins.
- **A question cannot be answered without typing.** `AskUserQuestion` also runs the `PermissionRequest` hook, but an
  `allow` only lets the tool run: the question dialog then waits for a person. **But the question can be removed**: a pane
  launched with `--disallowed-tools AskUserQuestion` never waited (the same request ended `idle`).
- **There is no heartbeat.** The registry's `updatedAt` did not move in 6 s of waiting, and `agents --json` has no
  `statusUpdatedAt`. A wedged process and a process legitimately inside a 4-hour tool call both show `busy`. So the hub
  keeps its **own** observation history (first poll that saw this status) and combines it with the transcript's mtime
  (the path is documented) to say `stalled`. *Inferred, not tested over hours.* This is the case in the epic's comment of
  2026-10-11: a poll would have said `busy` for four hours with no transcript growth, where a screen read said "I'm
  waiting for the seeding to finish…" 25 times.
- A SIGSTOP wedge could not be demonstrated here (the process stayed in state `S`), so it is **Not tested**.
- **How much a screen read can say** (the fallback if `agents --json` is unavailable): the footer separates idle from busy
  and the two dialogs are recognisable, but it cannot tell a stuck `busy` from a working one, and nothing in it says which
  session is shown. It is a fallback for the unmanaged panes of #832, not the design.

## 5. Login

**Verified** (names only; nothing was read) by `claude-session.sh`; credential order **Documented**.

- Under `env -i` with `HOME`, `PATH`, `TERM`, `LANG` only, `claude auth status` reports `authMethod: claude.ai`. The login is
  `<config dir>/.credentials.json` (mode `600`) on Linux; macOS uses the Keychain with that file as a fallback.
- `CLAUDE_CONFIG_DIR` pointed at an empty directory → `none`. The login travels with `HOME` / the config directory, so
  **the pane's process must keep `HOME`** (and `PATH`, to be found). A profile or spec never holds a path to it.
- Variables that **silently change the login**, so the spec's `env` names for a Claude pane must not include them and the
  launch should scrub them: `ANTHROPIC_API_KEY` (a dummy value → `authMethod: api_key`; in an interactive session the
  client asks once, in `-p` it never asks), `CLAUDE_CODE_OAUTH_TOKEN` (→ `oauth_token`), `ANTHROPIC_AUTH_TOKEN`,
  `CLAUDE_CONFIG_DIR`, `CLAUDE_CODE_USE_BEDROCK` / `_VERTEX` / `_FOUNDRY`. This is the cost-control point: an inherited key
  moves a pane from the subscription to per-token billing with no error.
- `claude setup-token` mints a long-lived token for `CLAUDE_CODE_OAUTH_TOKEN`. It is **not** needed and not recommended
  for panes on the machine where the login exists; it would put a secret into an env var, which I7 forbids a profile to hold.
- The health check for the login is `claude auth status` (exit 0 when logged in), `authMethod` field only. An *expired*
  login shows at the first request, not at launch (**Documented**; not provoked here).
- **Not tested:** a directory the client has never trusted. No trust dialog or bypass-permissions dialog appeared in any
  scratch launch, including `--dangerously-skip-permissions`; a never-trusted directory is documented to ask. The signal
  is `waitingFor: "dialog open"` (**Documented**), which the launch's observe step must treat as a failure to bring the
  pane up, not as health.

## 6. Where the process runs, and who stops it

- **Verified.** The interactive `claude` process runs as a normal child of the tmux session the host adapter creates for
  the pane; the pane's pid is the process (`agents --json`'s `pid` matched). `HostPort::run(name, cwd, argv)` starts it,
  `ps(name)` finds its pid, `stop_owned(name)` ends it. `SIGTERM` ended the process and its tmux pane, removed the
  registry entry, and kept the transcript, so a relaunch can resume.
- **Do not** put the process under `--bg` or `claude agents`: a background session lives under a supervisor started on
  demand (`~/.claude/daemon/…`), outside the pane's tmux session, so `HostPort` could neither find nor stop it, and the
  supervisor is a second place that tracks processes next to the pane registry (epic decision 2).
- **The argv is an argv array, never a shell string** (B2): `claude --session-id <uuid> --settings <inline json> --model M
  --effort E --permission-mode P [--allowed-tools …] [--disallowed-tools AskUserQuestion] [--agent KEY]`. All of those are
  documented flags. Nothing in it is a secret.

## Recommended design

`ClaudeCode` implements `HarnessPort`. It composes three small seams, each faked in the test kit (below): `claude
agents --json` (list), a unix-socket write (deliver), and `claude auth status` (login).

**Launch** (the `launch` verb with a Claude harness, a flag that does not exist yet): the launch engine chooses a fresh UUID *first* (this is
`create_session`; it is a Holler-side value, not a server call), records it as `session_of_record`, and runs the argv
above through `HostPort::run` in the pane's tmux session. The `--settings` JSON carries `crossSessionInbound: accept` and a
`SessionStart` hook that tells Holler the socket path. The observe step is `agents --json` for the pane's pid: the entry
exists, `sessionId` equals the record, `status` is `idle`. A failure rolls back by `stop_owned`. **Relaunch** uses
`--resume <id>` if `<config dir>/projects/…/<id>.jsonl` exists and `--session-id <id>` if it does not.

**Switch and reset** are relaunches: stop the owner of record, start `--resume <other id>` (switch) or a fresh UUID
(reset). The session the pane shows can never differ from the session the hub drives, because the socket belongs to the
process; a mismatch can only mean a person typed `/clear` or `/resume`, which `agents --json` shows, and `doctor --fix`
repairs by relaunching `--resume <session_of_record>`.

**Say** (`send_prompt`): the framed JSON line to the socket. **Activity** (`session_activity`): idle / busy / waiting from
`agents --json`. **Answer a permission**: the hook's decision file. **Answer a question**: removed by
`--disallowed-tools AskUserQuestion`; a pane that runs without it reports `HoldingQuestion` and `answer` refuses with a
stable code, because nothing but a keypress answers it. **Abort**: stop and relaunch `--resume` (see the contract change
below); the hook gate is available as a softer stop.

### Contract changes for story #831

Amend-first (ADR-0021 and the epic's contract block), in this order so the OpenCode behaviour never changes in a step:

1. **`HarnessKind::Claude`**, serde name `claude`. `holler_proto::vocab::HARNESS_IDS` already lists `claude`, so no id is
   added; its doc comment ("only `opencode` is a v2 adapter") changes to say that the pane harness `claude` is the
   interactive CLI and is not the ACP body harness of the same name (ADR-0012). The exhaustive match in
   `holler-pane/tests/rework_test.rs` gains the arm, and the existing test that pins every `HarnessKind` serde name to
   `HARNESS_IDS` covers it.
2. **An endpoint instead of a port.** `HarnessInfo.port: u16` and every `HarnessPort` method keyed by `port`
   (`health`, `create_session`, `list_sessions`, `abort`, `send_prompt`, `session_activity`, `verify_serving`) take a
   `HarnessEndpoint`: `Port(u16)` (OpenCode, unchanged behaviour) or `Process` (pane name and pid; the adapter finds the
   rest). A stored record that carries `port` keeps loading. Mechanical but wide: `tx_launch`, `tx_switch`, `tx_apply`,
   `reconcile`, `import`, the OpenCode adapter, `FakeHarness`, the conformance suite.
3. **A harness shape**, `HarnessPort::shape() -> Shape { ServerAndTui, SingleProcess }`. For `SingleProcess` the engine
   skips `health` of a server, `create_session`, `attach_tui` and in-place `select_session`; it asks the port for the
   launch argv (a pure `process_command(name, spec, SessionChoice)` beside `serve_command`) and relaunches for switch,
   reset and doctor's repair.
4. **`Activity::WaitingPermission`** beside `Idle`, `Busy` and `HoldingQuestion`, so `reset` refuses a pane that waits on a
   permission and the roster can say so.
5. **`HarnessPort::abort_style() -> AbortStyle { InPlace, Relaunch, Unsupported }`** (default `InPlace`); `interrupt` (#646)
   relaunches for `Relaunch`. `abort` itself returns the stable refusal `abort-unsupported` for such a harness.
6. **`ServeSpec` grows** (it is `#[non_exhaustive]` and says model, effort, env names and context ceilings join it later):
   `model`, `effort`, `permission_mode`, `disallow_questions` and the allowed-tools list, built into the argv by
   `process_command`. `Pane.opencode_agent` stays OpenCode's; Claude's `--agent KEY` reuses `AgentKey` and
   `verify_serving` answers `agent-unsupported` for a key it cannot check (`agents --json` does not report an agent).
7. **Spec flags**: `--harness opencode|claude` on launch/relaunch and in `ProfileSpec.harness.kind`, and `--port-policy`
   becomes meaningless for `claude` (refused in one line: `port-policy-unsupported`).
8. **Doctor findings** (`FindingKind`): `ClaudeLoginMissing` (`authMethod` is not `claude.ai`, or one of the override
   variables above is in the pane's env), `PaneWaiting` (`waiting` longer than a threshold, with the reason from
   `waitingFor`; this is also what #832's `unmanaged` list needs), `PaneStalled` (`busy` longer than a threshold with a
   transcript that has not grown). `ShownDrivenMismatch`'s repair becomes "relaunch `--resume`" for `SingleProcess`.
   `ServerDown` is reused for "no `agents --json` entry for the recorded pid".
9. **Refusal codes** (open codes owned by the adapter): `abort-unsupported`, `answer-unsupported`, `session-in-use`,
   `no-conversation`, `inbound-held` (a delivered message left the pane `waiting`).
10. **Docs**: ADR-0021 (section on harnesses), `docs/pane-control.md`, and a version note: the delivery frame was
    **Read** (now **Verified** 2026-10-13 on Linux, claude 2.1.296; macOS and other versions are not covered), so the conformance probe in `scripts/spikes/claude-drive.sh` becomes an opt-in `#[ignore]` real-binary test
    that runs on every Claude Code upgrade.

### Test-kit fakes #831 needs

`FakeHarness` models a server by port and a TUI view; it does not fit. Add, in `holler-pane-testkit`:

- **`FakeClaude`**: processes with `{pid, session_id, status, waiting_for, inbox, accept_inbound}`. `launch(argv)` parses
  `--session-id` / `--resume` / `--settings` (reading `crossSessionInbound`) and reproduces the verified refusals
  (`session-in-use` when running or when a transcript exists; `no-conversation` for an unknown or never-prompted id on
  `--resume`). `agents()` returns the `agents --json` entries and drops a killed process. `deliver(pid, frame)` queues,
  or holds when the process "bypasses" and the setting is not `accept`. Faults: `sigint(pid)` exits the process,
  `stall(pid)` leaves it `busy`, `hold_permission(pid)` and `hold_question(pid)` set `waiting`.
- Three tiny seams the real adapter takes, each with a fake: `AgentsList` (runs `claude agents --json`), `InboxWrite`
  (writes the frame to a socket path), `LoginCheck` (`claude auth status`'s `authMethod`).
- **Conformance**: a `claude_cases()` list beside `harness_cases()` and a `run_claude_conformance` that the fake and the
  real adapter (against a scratch Claude Code, opt-in) both run. The cases are the PASS lines of the four probes.

## Decisions

Taken here (reversible; stated so they can be reversed):

1. **Delivery is the inbox socket**, framed as a peer message. It is the only way found that keeps the TUI usable, needs
   no keypress at launch and no second process; its price is that the frame is not a documented interface and the model
   treats the hub's order as a teammate's request. The conformance probe is the guard.
2. **Abort is stop-and-relaunch**, not a keypress. It is deterministic and documented, and loses only the in-flight turn
   (the session and its history survive). The optional `PreToolUse` hook gate gives a softer "stop after this step" later.
3. **Claude panes launch with `--disallowed-tools AskUserQuestion`** (default; a spec field turns it off), because a
   question is the one wait the hub cannot answer.
4. **Use `agents --json`, hooks and the transcript path; never read `<config dir>/sessions/*.json`.**

**Decided by the operator, 2026-10-11 (recorded on #831 and in ADR-0021 section 15):**

- **Is one Esc keypress through `HerdrPort::send_keys` allowed as an in-place interrupt? No.** Abort for a Claude Code pane is
  stop and relaunch with `--resume <session of record>`: it loses only the turn in flight and keeps the session. An
  `AbortStyle::InPlace` for Claude Code is not built, and nothing types into a pane (I4). The optional `PreToolUse` hook gate
  stays a later, separate option. (This note had recommended against the keypress, and asked the operator to confirm.)

## Not tested

A never-trusted working directory; an expired login; a SIGSTOP wedge; a `busy` stretch of hours; the `notify_when_idle`
socket action; two TUIs resuming the same id at once (the docs say messages from both interleave in one transcript); the
ACP adapter (its npm metadata was read, nothing was installed or run: `@agentclientprotocol/claude-agent-acp` 0.89.1 wraps
the Claude Agent SDK, and Zed's documentation says its managed copy ships a vendored CLI; the old `@zed-industries` name is
deprecated); macOS (the Keychain path and
the socket directory differ); Windows (named pipe, required auth line).

## Re-running

```bash
scripts/spikes/claude-session.sh     # questions 1, 5, 6  (2 model turns)
scripts/spikes/claude-drive.sh [A B C D E F]   # question 2   (about 10 turns; F starts and stops its own supervisor)
scripts/spikes/claude-interrupt.sh [A B C D E] # question 3
scripts/spikes/claude-observe.sh     # question 4
```

Each prints `PASS` / `FAIL` / `INFO` lines and exits 1 on a `FAIL`. They touch only their own scratch directory, tmux socket
and (for `--bg`) supervisor, and purge the scratch project on exit.
