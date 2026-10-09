# OpenCode pane spike: attach, switch, observe (issue #635)

Verify-first spike for epic [#633](https://github.com/Performant-Labs/holler/issues/633), story
[#635](https://github.com/Performant-Labs/holler/issues/635). The question is whether OpenCode can do what
`HarnessPort` (`serve / health / create_session / list_sessions / abort / attach_tui / select_session /
shown_session`) needs, so that Holler creates the session of record and the TUI attaches to it instead of
Holler attaching to whatever session a TUI happens to show.

- **OpenCode version probed:** `opencode --version` → `1.18.35` (Linux x86-64), probed 2026-10-09.
- **Probes:** `scripts/spikes/opencode-api.sh` (capabilities 1 and 5), `scripts/spikes/opencode-tui.sh`
  (2, 3, 4 and the deleted-session case), `scripts/spikes/opencode-health.sh` (6), with shared helpers in
  `scripts/spikes/opencode-lib.sh`. Each prints one `PASS` / `FAIL` / `INFO` line per observation and exits 1
  on any `FAIL`, so story #642 can lift them into contract tests. Re-run them whenever OpenCode is upgraded.
- **Needs:** `opencode`, `curl`, `jq`, `tmux`, `setsid`, `timeout` (Linux). `OPENCODE_BIN` picks another binary.

## How the probes stay away from a real fleet and a real model

Every result below came from these scripts or from a manual run under the same rules.

- Each run makes a fresh `mktemp -d` directory (`<scratch-dir>`) and runs every OpenCode process under `env -i`
  with `HOME`, `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME` inside it. OpenCode
  1.18.35 honours them: `GET /path` reports `home`, `state` and `config` under `<scratch-dir>`, and the session
  database is created at `<scratch-dir>/data/opencode/opencode.db`. Nothing from the caller's environment
  (provider keys included) reaches OpenCode.
- **No model can be reached.** The scratch config disables the built-in free `opencode` provider (which needs no
  key and would otherwise be live) and enables exactly one provider, `deadend`, with base URL
  `http://127.0.0.1:9/v1`, a closed local port, and no key. Before any probe, `GET /config/providers` must
  report exactly that, and port 9 must refuse connections, or the run stops. A provider is configured at all
  only because `POST /session/:id/shell` needs a model *name* to attribute its message to: with no provider at
  all it fails with `500 ProviderNoProvidersError` (observed). The shell route never calls the model. No prompt is
  ever sent.
- Ports are picked free from 48100-48199 on 127.0.0.1. Servers run in their own process group (`setsid`). TUIs run
  on a private tmux server (`tmux -S <scratch-dir>/tmux.sock`), never the default one, and no key is ever sent to a
  TUI. An `EXIT` trap kills the private tmux server, the server process groups and their descendants, and any
  helper `curl`, then deletes `<scratch-dir>`.

**Evidence labels used below.** **Verified**: observed by running the probes (each script was run twice in a row
and passed both times). **Read**: taken from `opencode --help`, the server's OpenAPI document (`GET /doc`) or the
code bundled in the 1.18.35 binary, and not run. **Inferred**: a conclusion drawn from the other two and not
tested.

## Verdicts

| # | Capability | Verdict | One-line evidence |
|---|---|---|---|
| 1 | Headless `opencode serve`, Holler creates the session of record over the API | **works** | `POST /session {"title":…}` returns the session; no stray session exists; it survives a server restart (verified) |
| 2 | A TUI started attached to that server and that session | **works** | `opencode attach http://127.0.0.1:<port> --dir <dir> --session <id>` shows that session in about 1.6-1.7 s; an unknown id exits 1 (verified) |
| 3 | Switch the TUI to another session without typing | **works with a caveat** | `POST /tui/select-session` switches it in about 110 ms, but goes to *every* TUI on that server and answers `true` with no TUI at all (verified) |
| 4 | Report which session a TUI shows | **works with a caveat** | no API; the TUI sets the terminal title `OC \| <session title>`, which tmux reports as `#{pane_title}`. Holler must give every session a unique title of 40 characters or fewer (verified; the rules were read in the bundled code) |
| 5 | Abort a session | **works** | `POST /session/:id/abort` on a busy session goes idle in 22-28 ms and kills the shell child; aborting a model turn is unverified (it needs a model) |
| 6 | Survive or report a wedged server | **works with a caveat** | OpenCode reports nothing itself, and the TUI shows nothing; Holler detects a wedge with a timed `GET /global/health`. A restart on the same port and data directory keeps the sessions, and the old TUI picks the server up again (verified) |

## 1. Headless server, session of record created by Holler: works

**Calls (verified).**

```
opencode serve --pure --port <port> --hostname 127.0.0.1      # cwd = the project directory
GET  /global/health            -> 200 {"healthy":true,"version":"1.18.35"}
POST /session  {"title":"…"}   -> 200 {"id":"ses_…","directory":"<project dir>","title":"…",…}
POST /session?directory=<dir>  -> 200 (same; the directory query parameter selects the project instance)
GET  /session                  -> 200 [ …sessions, most recently updated first… ]
GET  /session/:id              -> 200 session | 404 {"name":"NotFoundError",…}
PATCH /session/:id {"title":…} -> 200 session (renames it)
DELETE /session/:id            -> 200 true | 404 when already gone
GET  /session/status           -> 200 {} when nothing runs; {"ses_…":{"type":"busy"}} while busy
```

**Observed (verified).** The server is healthy 630-740 ms after spawn, warm or cold, including with a fresh config
directory. A fresh server has **no** session of its own: no "ping" session. A created session keeps the title
Holler gives it and is placed in the server's working directory. After the server is killed and restarted on a new
port with the same data directory, `GET /session/:id` still returns it. Idle sessions are absent from
`/session/status`, so absence means idle. `GET /api/session/active` (the v2 API) returned `{"data":{}}`.

**Observed (verified): two servers that share a data directory share one session store.** A session created on the
second scratch server is readable through the first. The live fleet runs several `opencode serve` processes as one
user with one data directory, so every server lists every pane's sessions. That is how a "most recently active"
guess can pick another pane's session.

**Read (bundled code).** OpenCode replaces a session's title with a generated one only while the title is still its
default (`New session - <ISO time>`): `SessionPrompt.ensureTitle` returns early when `!isDefaultTitle(title)`. A title
Holler sets is therefore kept after the first prompt. This was not run, because running it needs a model call.

**Stable enough to depend on:** yes. These are the documented `/session` routes and appear in `GET /doc` under
`session.create`, `session.list`, `session.get`, `session.update`, `session.delete` and `session.status`
(`opencode-api.sh` asserts each one). **Fallback:** none needed.

## 2. TUI attached to a given server and session: works

**Command (verified).**

```
opencode attach http://127.0.0.1:<port> --dir <project dir> --session <ses_id>     # -s <ses_id> also works
```

**Observed (verified).**
- The TUI shows the given session (terminal title `OC | <title>`, and the title in its sidebar) 1.6-1.7 s after
  launch. Attaching creates no session.
- `-s` is the short form of `--session` and behaves the same.
- An **unknown** session id makes `attach` exit with status 1 and `Error: Session not found: ses_…` on stderr within
  about a second. It fails loudly; it does not fall back to some other session.
- `attach` with no `--session` opens the home screen (title `OpenCode`) and creates no session.

**Read (help text).** `attach` also takes `--continue` (the last session; Holler must never use it, because it is the
"guess" that caused the incidents), `--fork`, and `-p/-u` for a server started with `OPENCODE_SERVER_PASSWORD`. The
scratch servers ran without a password (`serve` warns that the server is unsecured); password auth was not probed.

**Stable enough:** yes. **Fallback:** none needed. If `--session` ever broke, start `attach` without it and then
`POST /tui/select-session` (capability 3).

## 3. Switch the TUI without typing: works with a caveat

**Call (verified).** `POST /tui/select-session {"sessionID":"ses_…"}` (optionally `?directory=<dir>`) → `200 true`.
The attached TUI shows the new session in about 110 ms (its title changes), with no keystroke.

**Observed (verified).**
- An unknown id answers `404 NotFoundError` and the TUI stays where it was. A malformed id (not `ses…`) answers
  `400`.
- **Caveat 1: broadcast.** Two TUIs attached to one server *both* switched on one `select-session`. The call is
  not addressed to a TUI. **Read (bundled code):** the server publishes a `tui.session.select` event, and every TUI
  subscribed to that server whose workspace matches navigates.
- **Caveat 2: no acknowledgement.** On a server with **no** TUI attached, `select-session` still answers
  `200 true`. A `true` therefore proves nothing about the screen, and Holler must confirm the switch by observing
  the TUI (capability 4).

**Stable enough:** yes, under two conditions: one TUI per server, and a switch counts only once the title confirms
it. **Fallback** if it ever stopped working: relaunch the TUI process in the pane with
`attach --session <new id>`, which was verified in capability 2 and costs about 1.7 s.

## 4. Report which session a TUI shows: works with a caveat

There is **no** endpoint that returns the session a TUI shows (read: no such route in `GET /doc`; `/tui/*` routes
only push to the TUI). `GET /tui/control/next` ("Retrieve the next TUI request from the queue for processing", read
in `/doc`) is a long poll: with a 3 s client timeout it returned no answer (verified). Holler must not read from it, because doing so would steal
requests meant for the TUI (inferred).

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
terminal title the way tmux does is #636's question and is unverified here. The probes read it from tmux.

**Stable enough:** moderately. It is UI behaviour rather than API, so `opencode-tui.sh` must run on every OpenCode
upgrade. **Fallback** if the title channel is lost: never use `select-session`; switch only by relaunching
`attach --session <id>`, and take SHOWN from the TUI process's own argv (`--session <id>`, observable with `ps`).
That is weaker evidence, because a person can still navigate the TUI by hand, so `shown_session` should then
answer "unknown (from argv)" and not a confirmed id. A last resort is reading the screen (the sidebar prints the
title), which is unverified for narrow panes where the sidebar may be hidden.

## 5. Abort a session: works

**Call (verified).** `POST /session/:id/abort` → `200 true`.

**Observed (verified).**
- Idle session: `200 true`, no effect.
- Busy session: `POST /session/:id/shell {"agent":"build","command":"sleep <n>"}` makes `/session/status` report
  `busy` without a model call. `abort` then answers `200 true`, the session leaves `/session/status` (idle) within
  22-28 ms, the `sleep` child is gone, and the shell call returns `200` with tool output
  `User aborted the command`.
- **Unknown id:** `200 true`, the same as success. Holler must check `GET /session/:id` (404) first, or it will
  report "aborted" for a session that does not exist.
- The v2 route `POST /api/session/:id/interrupt` answered `204` on an idle session ("Idle interruption is a no-op",
  per `/doc`). The v1 `abort` is the one the probes depend on.
- **Process detail (verified):** OpenCode runs the shell command in a login `bash -l` in a session and process
  group of its own, not the server's. **Inferred:** killing a server's process group alone would not reach a running
  shell command, so `stop_owned` must also stop the server's descendants (the probes' cleanup does).

**Unverified:** aborting a model turn (streaming, or waiting on a provider) needs a model and was not run. **Read
(`/doc`):** abort "stop[s] any ongoing AI processing or command execution". **Stable enough:** yes for what was
verified. **Fallback** for a turn that will not stop: relaunch the server (capability 6).

## 6. Survive or report a wedged server: works with a caveat

The 2026-10-07 wedge (days at 1.7 GB) cannot be reproduced on demand. The probe stands in for it with `SIGSTOP`,
which gives an HTTP client the same symptom.

**Observed (verified).**
- **Frozen (SIGSTOP):** a TCP connect still succeeds (the kernel accepts it), and `GET /global/health` gets **no
  answer**; the client's own 2 s limit is all that ends it (`000` after 2008 ms). Only an HTTP timeout reveals a
  wedge. After `SIGCONT` the server answers again within about 150 ms, and the TUI still obeys `select-session`.
- **Killed (SIGKILL):** the connection is refused within about 6 ms.
- **The TUI reports neither.** While its server was frozen or dead, the TUI stayed alive with its title unchanged
  and printed nothing about the connection. It cannot be asked whether its server is healthy.
- **Restart:** a new `serve` on the same port and data directory was healthy in about 645 ms. The session of record
  was still there, and the **old** TUI received a `show-toast` from the new server, so it re-subscribed by itself.
- **Boot race:** health GETs were sent every 50 ms from spawn, each with a 30 s limit. About 11 were refused, and
  then **1 or 2 GETs sent at about 570-680 ms were accepted and never answered in 30 s**, while GETs sent a few ms
  later were answered at once (seen on every run). An earlier manual run had one such request hang for more than
  3 minutes. **Every request to OpenCode needs a client timeout, and none but a health poll may be sent before the
  first healthy answer.**
- **Load:** with 20 concurrent `POST /session`, then 500 `GET /session` at concurrency 50 while 20 `/event`
  streams were held open, every request answered 200 within about 1.25 s in total. Health latency during the load
  was 6-80 ms. The server's RSS was about 330 MB just after boot and 670-840 MB after the load (the server is a single process).
  This is not a soak: growth over days is **unverified**.

**Stable enough:** the health endpoint, yes. OpenCode's own reporting: there is none. **Design consequence:** the
harness's `health` is Holler's own probe (`GET /global/health` with a hard timeout). Recovery is to restart the
server on the same port with the same data directory, then confirm with `shown_session` that the TUI shows the
session of record. Relaunch the TUI only if that check fails.

## Other items the issue lists

| Item | Result |
|---|---|
| `opencode attach` | verified, see 2 |
| `--session` / `-s` | verified both forms, see 2 |
| `POST /session`, `GET /session` | verified, see 1 |
| `POST /session/:id/abort` | verified on idle, busy (shell) and unknown ids, see 5 |
| `/tui/*` control endpoints | `select-session` and `show-toast` verified (the toast text appears on the TUI's screen); `control/next` is a long poll (verified: no answer in 3 s). `append-prompt`, `submit-prompt`, `clear-prompt`, `execute-command`, `open-help`, `open-sessions`, `open-themes`, `open-models` and `publish` were **read only** in `/doc`, not run: they are typing by another route, which invariant I4 rules out, and `submit-prompt` would call a model |
| Server health under load | verified, see 6 |
| TUI whose session is deleted | verified: it leaves the session within about 110 ms, shows the toast "The current session was deleted", goes to the home screen (title `OpenCode`), **stays running**, and creates no replacement session. `select-session` to a live session brings it back. **Inferred:** anything typed on that home screen would start a new session that no one recorded, so reconcile must treat SHOWN = none as a mismatch to fix at once |

## Recommendation

**The adapter (`holler-adapter-opencode`, story #642) that follows from the results:**

1. **`serve`:** spawn `opencode serve --port <p> --hostname 127.0.0.1` with the project as its working directory, in
   a process group of its own. Poll only `GET /global/health` (2 s per try) until it is healthy (under 1 s here; the
   bound is I5's 10 s), and send nothing else before that.
2. **`create_session`:** `POST /session`, then `PATCH` its title to its own id, so the title is unique, under 40
   characters and maps back to the id. Never use `--continue` or "most recent", because sessions are shared across
   every server on the same data directory.
3. **`attach_tui`:** run `opencode attach http://127.0.0.1:<p> --dir <cwd> --session <id>` in the pane, with
   `OPENCODE_DISABLE_TERMINAL_TITLE` unset, then wait (bounded) until `shown_session` equals the id. If `attach`
   exits, the error is `session-not-found`.
4. **`select_session`:** check `GET /session/:id` (404 means `session-not-found`), call `POST /tui/select-session`,
   then wait (bounded, about 2 s) until `shown_session` equals the id, or fail loudly and record nothing (I3).
   Exactly one TUI per server, so the broadcast reaches only that pane.
5. **`shown_session`:** read the pane's terminal title. `OC | ses_…` gives `Some(id)`, `OpenCode` gives `None`
   (home, a deleted session or a default title), and anything else gives `Unknown`. **`abort`:** `GET` first (an
   unknown id answers `true`), then `POST …/abort`, then observe `/session/status` until the session is idle.
   **`health`:** a timed GET gives `Healthy`, `Unhealthy("timeout")` (frozen) or `Unhealthy("refused")` (dead).
   **`stop_owned`:** kill the server's process group **and its descendants**.

**Fallback, should (3) or (4) stop working in a later version:** switch by relaunching the TUI with
`attach --session <id>` (verified), report SHOWN from the TUI's argv marked "unknown (from argv)", and treat any
hand navigation as drift. (2) works, so there is no fallback for it beyond 3's.

**Contract tests for #642 should assert** (lift them from these scripts; they need only a scratch server, tmux and
no model):
- `/doc` lists `global.health`, `session.create`, `session.list`, `session.get`, `session.update`,
  `session.delete`, `session.status`, `session.abort`, `tui.selectSession` and `tui.showToast`.
- Health answers `{"healthy":true,"version":…}` within 10 s of spawn. A fresh server has 0 sessions. `POST /session`
  keeps the given title and directory. The session survives a restart on the same data directory.
- `attach --session A` gives title `OC | A`. `select-session B` gives `OC | B` within 2 s. An unknown id gives 404
  and the title is unchanged. `attach --session <unknown>` exits non-zero.
- Deleting the shown session gives title `OpenCode`, the TUI is still alive, and no new session appears.
- `abort` on a busy session (the `shell` + `sleep` trick, with the dead-end provider) is idle within 1 s and the
  child is gone. `abort` on an unknown id answers `true` (pinned, so a change in it is noticed).
- A `SIGSTOP`ped server gives health `000` at the client limit. `SIGKILL` gives refused within 1 s. A restart on the
  same port lets the old TUI receive a toast.
- `select-session` with no TUI answers `true` (pinned: it proves that a switch has to be observed).

## Not verified

- Aborting a **model** turn, and whether a set title survives the first prompt (read in the code, not run): both
  need a model call.
- Whether Herdr exposes a pane's terminal title (#636); everything here read it through tmux.
- A wedge that lasts days, and memory growth over time (only the SIGSTOP stand-in and a short load were run).
- Basic-auth servers (`OPENCODE_SERVER_PASSWORD`) and `attach -p/-u`.
- `select-session` to a session from **another project directory** than the TUI's `--dir`. Workspaces
  (`?workspace=`) were not exercised; the bundled code filters `tui.session.select` by workspace.
- What a person sees on a narrow pane where the sidebar may be hidden (only the title was relied on).
- Any OpenCode version other than 1.18.35.
