# Orchestrating: dispatch, wait, act

How an orchestrator (a "meta-O" driving one or more Holler `body` sessions)
should watch a session without polling it — the pattern [holler#142][142]
built, and [`docs/guidance-effective-agent-monitoring.md`](guidance-effective-agent-monitoring.md)
is the retrospective on what broke before it existed.

[142]: https://github.com/Performant-Labs/holler/issues/142

## The pattern, stated once

1. **Dispatch with `say`.** `say SESSION "…"` (`holler say SESSION TEXT`) sends one prompt and
   returns when that turn settles. Fire it, note the `turn_id` it reports
   (or the roster's `turn_id` column right after), and move on — don't sit
   in the same call waiting on a long-running turn if the orchestrator has
   other sessions to manage.
2. **Do not make the orchestrator poll `roster`.** Reading `roster` in a
   loop ("are you done yet?") is exactly the anti-pattern this story
   replaces: it burns the orchestrator's own turns on a busy-wait, and it
   trains the wrong habit — retrying by hand instead of trusting the
   session to report back.
3. **Run a deterministic waiter instead: `wait … --after TURN_ID`.**
   This is a plain, poll-free blocking call — it parks on the hub's roster
   change channel (`Roster::subscribe`, a `tokio::sync::watch`) and only
   returns when a real state change lands, or the timeout elapses. Put it in
   a shell loop (or a systemd/launchd unit — anything that can run a
   long-lived process and send a message on exit) so it messages the
   orchestrator **only** when judgment is actually needed.
4. **Redirect with `interrupt SESSION TEXT`.** If the waiter reports
   `input-required` or the orchestrator decides the current turn is wrong,
   `interrupt` cancels it and (with `TEXT`) immediately starts a new one —
   one atomic step, not a cancel-then-separately-say race.
5. **Answer with `answer SESSION CHOICE`.** A held permission/elicitation
   (issue #151) is resolved here, never by re-`say`ing — a plain `say` to an
   `input-required` session is refused (`session_busy`, hinting at `answer`).
6. **Treat `stalled` as "look", never "interrupt".** `stalled` is a
   *display* hint (`roster`'s own derived state: `working` with no update
   for longer than `HOLLER_STALL_MS`) — it means "check on this", not "this
   is stuck, kill it". A session that is merely slow (a long compile, a big
   diff) looks identical to a wedged one from the outside; `interrupt`ing on
   `stalled` alone throws away real work on a guess. Look first (`say` a
   status probe, read logs, check `roster`'s `LAST TURN` column), and only
   `interrupt` once something concrete says the turn is actually dead.

## `wait`'s own contract (issue #142)

```
holler wait SESSION[,SESSION…] [--until STATE[,STATE…]] [--after TURN_ID] [--timeout 600s] [--json]
```

- **Edge-triggered, caller-held watermark — no hub `ack`.** If any named
  session already matches `--until` at call time, `wait` returns
  immediately. `--after TURN_ID` (the last turn id the caller already
  processed) stops a terminal-state match from re-firing on a turn already
  reported: a match against `completed`/`canceled`/`failed`/`rejected` then
  additionally requires `last_turn.turn_id != TURN_ID`. The hub itself keeps
  **no** watermark, no `done` state, and no `ack` verb — "seen" is a
  per-consumer fact (the orchestrator vs. a human watching the same session
  may be at different points), so it lives with the consumer, in the
  `--after` value the *next* `wait` call is launched with.
- **Default `--until`** (nothing given): `completed,failed,rejected,
  input-required,gone` — every terminal turn outcome, a held
  permission/elicitation, and a vanished body. Bare `idle` is deliberately
  **not** in the default set: a session goes idle after a join, an
  interrupt, a refusal, *and* a plain success, so a default that fired on
  it would wake the orchestrator far too eagerly. Ask for it explicitly
  (`--until idle,completed`) when idle itself is the signal you want.
- **`--prefix io/` instead of a session list** watches every roster row
  under that prefix at once (the same `ADR 0005 §4` grammar `roster
  --prefix` uses) — handy for "wake me when *any* session under this
  workspace needs me", not just one.
- **Exit codes:** `0` a match landed (one line per matched row, or
  `--json`'s array); `1` no live hub reachable, or a control-socket error;
  `2` the timeout elapsed with no match — the normal "nothing happened in
  this window" outcome, not a failure.
- **No polling anywhere.** `wait` opens one control-socket call
  (`control/wait`) and blocks there; the hub side selects on the roster's
  own change channel and re-checks only when something actually changed. A
  `wait` that runs for an hour against a quiet session costs the hub
  exactly the handful of checks its actual state changes triggered, not
  one every however-many milliseconds.

## A reference loop (~12 lines)

```bash
#!/usr/bin/env bash
# Dispatch once, then let a deterministic waiter do the watching.
set -euo pipefail
SESSION="io/alpha"
holler say "$SESSION" "run the migration and report back"
AFTER=""
while :; do
  OUT=$(holler wait "$SESSION" ${AFTER:+--after "$AFTER"} --timeout 600s --json) || {
    code=$?; [ "$code" -eq 2 ] && continue; exit "$code"   # 2 = timeout, keep waiting
  }
  TURN=$(echo "$OUT" | jq -r '.rows[0].turn_id')
  STATE=$(echo "$OUT" | jq -r '.rows[0].state')
  echo "wait: $SESSION is now $STATE (turn $TURN)"          # <- message the orchestrator here
  AFTER="$TURN"
done
```

Swap the `echo` for whatever wakes the orchestrator's own attention (a
message send, a task-queue push) — the loop's only job is to hold the
`--after` watermark across calls and hand back exactly one line when
something real happens.
