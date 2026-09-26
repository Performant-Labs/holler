# Guidance for Effective Agent Monitoring

Practical guidance for an orchestrator (a "meta-O") driving one or more Holler
`body` sessions through real, long-running work — what actually broke during
the first live orchestration run against this repo, and what fixed it. This
is operational guidance, not a wire-level ADR: nothing here changes the
protocol. See [`docs/adr/ADR-0001.md`](adr/ADR-0001.md) for the hub/body
split and the multiple-orchestrator ("MO") pattern
for how a meta-O session is set up.

## The problem

`holler roster` exposes exactly two live signals per session: `blocked`
(a real question/permission gate) and connectivity `state`
(`connected`/`reconnecting`/`gone`). Everything else — "is this session still
working on what I gave it, or has it finished and is just sitting there
waiting for a reply?" — is invisible on the wire today. That gap is tracked
as [holler#142](https://github.com/Performant-Labs/holler/issues/142)
(`holler wait SESSION`, a real blocking waiter on roster state + `last_turn`
— now built; see [`docs/orchestrating.md`](orchestrating.md)). This section
records what broke *before* it shipped, and what a hand-rolled workaround
looked like; a new orchestrator should reach for `wait` directly rather than
reproducing this history.

Three failure modes came out of that gap during real use, found in this order:

1. **Polling by hand doesn't scale.** Asking "are you done yet?" via `say`
   in a loop is expensive (each call can legitimately take minutes if the
   session is mid-turn), and it trained the exact wrong habit — retrying a
   transient `say` timeout by hand, over and over, instead of trusting the
   session to reply when it's actually free.
2. **A monitor that only logs is not a monitor.** A deterministic external
   watchdog was built (see below) that correctly detected every blocked
   session and every finished turn — and then sat there logging it,
   unread, for **over two hours** on one occasion, because nothing pulled
   the orchestrator's attention to the log. Detection without an actual
   interrupt is just a diary.
3. **A "notified" flag can go stale and mask a real new event.** After
   wiring the interrupt (§2), a *second* real completion still went
   unreported: the watchdog's own de-duplication state (`probe_notified`,
   set after the first nudge so it doesn't re-nudge for the *same*
   completion) never reset, because every probe in between timed out
   (the session was busy on genuinely new work) instead of ever observing
   a "still working" reply — the one thing that would have cleared the
   flag. The interrupt mechanism was working correctly; the thing it was
   listening for silently stopped firing. See the fix in §1.

## What we decided

### 1. A deterministic, non-LLM watchdog — not another agent

Session state monitoring is a boring, cheap, high-frequency job. It does not
need judgment, so it should not cost a model call. A shell script on a
`systemd --user` timer (cron if timers aren't available) does this instead:

- **Fast check (every 60s):** poll `holler roster --json`, diff against the
  last-known state per session (persisted in a small JSON file), and only
  act on a **transition** — `blocked` flipping false→true, or connectivity
  entering `reconnecting`/`gone`. Recovery back to normal is logged, never
  re-alerted. This is the cheap, reliable 90% of monitoring.
- **Slow probe (every 5min):** a heuristic band-aid for the "finished but not
  blocked" gap. It sends a cheap, explicitly-labeled, non-mutating status
  question (`"Status check only, no new work: are you finished or still
  working?"`) and pattern-matches the reply for completion-sounding phrases.
  This **will** produce false positives and false negatives — it is a guess,
  labeled as one in every nudge it sends, and it is the first thing to retire
  once holler#142 ships.
- **Every run appends one line per event to a plain log file.** Not a
  database, not a dashboard — a `tail`-able log, because the actual fix (§2)
  depends on that.

**Known, undisguised gap:** the watchdog's nudge target is a Holler session
named `mo`, per the MO pattern — but nothing guarantees that session exists.
When it doesn't, every nudge attempt fails with `unknown_session`, and the
script logs that failure explicitly rather than swallowing it
(`nudge FAILED (mo unreachable? ...)`). A monitor that silently no-ops on its
own delivery failure is worse than no monitor — it manufactures false
confidence. Log the failure loudly; don't hide it. (This gap is still open —
fixing it means standing up a real holler body session for the orchestrator
role, which the orchestrator itself cannot do from inside its own
conversation.)

**Fixed: the stale-notified-flag bug (failure mode 3 above).** The
`probe_notified` flag suppresses a repeat nudge for the *same* completion,
but had no expiry — so a completion notified once, followed by hours where
every probe timed out instead of getting a real "still working" reply,
left the flag stuck `true` through a second, genuinely new completion. Fix:
record a timestamp alongside the flag; if it's older than `STALE_SECONDS`
(1200s — well past the 5-minute probe interval, short enough that a stuck
flag doesn't hide a real event for long) when a fresh "looks done" reply
comes in, treat the flag as stale, reset it, and nudge. **Verified live**,
not just reasoned about: the fix was tested by artificially staling a real
session's flag and running a real probe cycle — the script logged the
staleness reset, logged a fresh nudge, and the orchestrator's log-watch (§2)
caught the new line and pushed a real notification, observed arriving in
real time.

### 2. The orchestrator needs a real interrupt, not a habit

Building the watchdog was necessary but not sufficient — it moved the
polling problem from "ask the agent" to "read a log file," and reading a log
file on a self-imposed cadence is still polling. It failed exactly the same
way: a real completion sat logged and unread for hours because nothing
pushed it into the orchestrating session's attention.

The fix is to use the orchestrating harness's own **event-driven** primitive
instead of any polling loop, however disciplined:

    tail -F watchdog.log | grep -E --line-buffered \
      'blocked transition false->true|state transition .*\(bad\)|probe looks DONE'

Run as a persistent background watch, this pushes a notification into the
orchestrator's next turn the moment a matching line appears — no polling
cadence to forget, no log to remember to check. This is the actual
"interrupt that can't be ignored": the harness's task-notification /
event-stream mechanism, not a discipline the orchestrator has to maintain.
**Detection without a wired interrupt is not monitoring — it's a diary
nobody reads.**

**This mechanism is only as good as the events it's fed.** It is a real,
verified fix for "the orchestrator forgot to check the log" — but it cannot
notice an event the upstream script never logs in the first place (see
failure mode 3 above). An interrupt wired to a buggy event source is still
silent. Both halves — the watchdog emitting the right events, and the
orchestrator actually listening for them — have to be independently
correct and independently verified; fixing one does not imply the other
still works.

When the orchestrator itself needs to pull a human's attention (not just its
own) — e.g. it hit a real decision point while the human has stepped away —
that's a separate, deliberately-rare push (a phone/desktop notification),
reserved for things worth walking back to the terminal for. Routine progress
should never trigger one.

### 3. Verify the interrupt actually fires — don't just reason about it

Failure mode 3 happened *after* §1 and §2 were both already built, reasoned
through, and believed to be correct. Neither half was actually broken in
isolation — the watchdog's detection logic was sound, and the log-watch
interrupt genuinely does push notifications — but their combination had a
gap only a live test surfaced: a real event stopped being logged, and a
correctly-wired interrupt has nothing to catch if the thing it watches goes
silent.

The concrete practice this demands: **treat "I fixed it" and "I confirmed
it fires" as two separate claims**, and don't make the first one stand in
for the second.

- **Detection alone proves nothing about delivery.** A script correctly
  writing the right line to a log file is necessary, not sufficient — it
  says nothing about whether anything is actually listening, or whether the
  listener's filter matches that exact line.
- **An interrupt correctly wired to a source proves nothing about the
  source staying correct.** The log-watch in §2 was independently verified
  once — that it fires on a matching line — but that didn't guarantee the
  upstream script would keep producing matching lines under every real
  condition (e.g. a long run of timeouts). Re-verify after any change to
  either half, not just the half that changed.
- **Trigger the condition for real, or synthetically, and watch the
  notification land.** For a stale-state bug like this one, that meant
  hand-editing the state file to simulate the exact staleness window, then
  running a real probe cycle and observing the notification arrive — not
  reading the code and concluding it should work.
- **"Now I know exactly why" is not the same claim as "and I confirmed the
  fix."** Say the second one only once you've done it. A plausible root
  cause, stated with confidence, is still just a hypothesis until something
  external confirms it — the gap between those two claims is exactly where
  this failure mode kept recurring.

---

## Section: Claude (Claude Code)

Claude Code sessions have two native event-driven primitives worth knowing
about — use them instead of any sleep-and-poll loop:

- **A backgrounded command's completion is pushed automatically.** Any shell
  command that runs long enough gets moved to the background and its
  completion (stdout/stderr, exit code) arrives as a notification on its own
  — this covers the "tell me once, when this one thing finishes" case
  without any extra plumbing.
- **A persistent log-watch turns a file into a push source.** Point a
  `tail -F | grep --line-buffered <filter>` (or an equivalent poll loop that
  emits one line per event) at a monitoring tool that streams stdout lines
  as they arrive. Each matching line becomes a notification in the session's
  next turn — this is what turns the watchdog's log file from something that
  has to be remembered into something that interrupts on its own. Keep the
  filter to *actionable* lines only (a firehose of routine "check complete"
  lines gets rate-limited and is not useful anyway).
- Reserve an actual attention-grabbing (desktop/phone) push for genuine
  walk-away-worthy events — a real block needing a decision, a long job
  finishing — never for routine status.
- **Verify the interrupt per §3** — it's the same requirement here as
  anywhere else, not a Claude-specific nuance.
- **Known Claude tendency: reaching for `say` when the situation actually
  needs `interrupt`.** `say` queues a new prompt behind whatever the session
  is currently doing — marking the message "urgent" in its text changes
  nothing about when the session sees it. Observed repeatedly in this
  project's own orchestration: Claude sent time-sensitive checks via `say`
  while a session was mid-turn on something unrelated, and the check sat
  queued, unseen, until well after it would have mattered. `holler interrupt
  <session>` is the actual control-frame primitive — it reaches the session
  immediately, even mid-turn, cancelling the in-flight turn while the
  session stays on the roster and promptable again. If a message is
  genuinely time-sensitive, interrupt first, then `say` the real question
  once the session is free — don't just write "urgent" into a queued `say`
  and assume that changes its delivery.

  **Fixed, mechanically, not by memory.** Telling the model "remember to use
  interrupt" doesn't survive compaction or a fresh session — this tendency
  recurred multiple times *within the same session* despite being corrected
  each time. The actual fix is a `PreToolUse` hook on the `Bash` tool
  (`~/.claude/settings.json`, per-user) that inspects every Bash command: if
  it matches a `say` invocation **and** contains urgency language (`urgent`,
  `immediately`, `right away`, `asap`, `as soon as possible`), the hook
  **denies the tool call** with a message pointing at `interrupt` instead.
  It never auto-inserts or auto-prepends `interrupt` — that would cancel
  real in-progress work on *every* routine check, trading one failure mode
  for a worse one. It only blocks-and-explains. Per §3: this was pipe-tested
  against synthetic payloads (which caught a real false positive — "right
  now" is common in ordinary status-check phrasing like "what are you doing
  right now" and had to be dropped from the trigger-word list) and then
  **live-fired** — a real Bash tool call containing the trigger pattern was
  denied before it executed, confirmed by the command's output never
  appearing. A written reminder is a suggestion the model can forget or
  override; a hook is a gate the tool call cannot get past.

## Section: OpenCode

OpenCode sessions (the harness `alpha`/`beta` ran on in this repo's own
build-out) expose their own HTTP surface directly on the running
`opencode serve` instance, independent of whatever Holler's wire currently
carries:

- **`GET /question` and `GET /permission`** on the session's own endpoint
  list any pending structured question or tool-permission gate — this is
  the ground truth behind Holler's `roster` `blocked` field. When `roster`
  shows a session blocked, these two endpoints tell you *what* it's blocked
  on and the exact option labels/indices to answer with.
- **Answer via `holler answer <session> "<choice>"`** (issue #382/#133,
  shipped in holler-server 0.1.1+) rather than reaching into the HTTP API
  directly when a live hub is available — it's the supported path and
  updates the roster's `blocked` field on success. For a **multi-question**
  request, pass comma-separated choices in order (`"Yes,Certain"`), and
  match the exact option **label** text if the numeric index is rejected —
  labels are matched case-insensitively but must otherwise be exact.
- **`roster`'s `blocked` column (0.1.2+)** is pushed live the moment a
  session's own question/permission status changes (`session_blocked`), not
  just at the next reconnect the way `presence` is — this is what makes the
  fast-check half of the watchdog above actually work in near-real-time.
- Spawn-mode (ACP) sessions are answered the same way but by option index
  (`holler answer SESSION 0`), not by the `once`/`always`/`reject` shorthands, which
  only the attach driver understands ([#476](https://github.com/Performant-Labs/holler/issues/476)).
  Multi-question requests where the question count itself is unknown ahead of
  time still require reading `/question` directly to discover the shape before
  answering.

## Section: Codex

Verified against `@agentclientprotocol/codex-acp` 1.13.1 driven through Holler
([#473](https://github.com/Performant-Labs/holler/issues/473), tracked under
[#303](https://github.com/Performant-Labs/holler/issues/303)). The general
principles above (a deterministic external watchdog, transition-only alerting, a
real push interrupt on the orchestrator's side) carry over unchanged.

- **Codex does surface permission requests, and Holler reports them as
  `blocked`.** The session shows `input-required` in `roster`, and
  `roster --json` lists a pending item of `kind: permission` with the prompt
  (for example `Run command`) and its options, such as `Yes, proceed`, a
  "don't ask again for commands that start with ..." option, and `No, and tell
  Codex what to do differently`.
- **Whether Codex asks depends on the adapter's mode, not on `config.toml`.**
  Set `INITIAL_AGENT_MODE` in the body's environment: `read-only` ("Ask for
  approval") asks before editing files outside the workspace or using the
  network, `agent` (the default) only asks for actions it judges unsafe, and
  `agent-full-access` never asks. A write inside the workspace ran without a
  prompt in both the default and `read-only` modes. The adapter ignores
  `approval_policy` in `config.toml`, and a Codex build that no longer supports
  a value exits at startup instead of ignoring it.
- **Answer with the option's index.** `holler answer SESSION 0` approves once;
  `holler answer SESSION 2` rejects. The `once`/`always`/`reject` shorthands do
  not work for an ACP session
  ([#476](https://github.com/Performant-Labs/holler/issues/476)), and an option
  label that contains a comma, such as the reject option, cannot be selected by
  label ([#477](https://github.com/Performant-Labs/holler/issues/477)).
- **Rejecting ends the turn.** The waiting `say` returns `prompt was interrupted
  before it completed` and the session goes back to `idle`.
- **`holler interrupt SESSION` clears a pending request.** The session returns
  to `idle`, the guarded action does not run, and the next `say` works.
