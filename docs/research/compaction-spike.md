# Compaction spike: OpenCode `summarize` and Claude Code `/compact` (#878)

Status: run 2026-10-12. Probes: `scripts/spikes/opencode-compact.sh` (with `scripts/spikes/opencode-stub-model.py`) and `scripts/spikes/claude-compact.sh`.
Recorded OpenCode reply bodies: [`compaction-spike-fixtures/opencode/`](compaction-spike-fixtures/opencode/).

## Summary and verdicts

- **OpenCode: tested on `opencode 1.18.35` ONLY.** Version 1.19.0 (the version Part A was written from) was not available locally and nothing was downloaded, so every OpenCode
  finding is for 1.18.35. `POST /session/:id/summarize` works and returns `200 true` when the summary is finished; no polling is needed. Part A's assumptions hold except one: a failed
  summary does not return an error status but `200 true` after about 71 s, leaving an empty `summary:true` marker message.
- **Claude Code: tested on `claude 2.1.296`. Verdict: Claude Code stays `compact-unsupported`.** `/compact` sent over the inbox is answered as ordinary text and does not compact;
  `claude agents --json` has only `idle` and `busy` (no `compacting`); and token counts are in the transcript only after a compaction that something else caused.

Evidence labels: **Verified** = observed by running the probe; **Documented** = stated by the server's own OpenAPI document (`GET /doc` of the running server) or by documentation;
**Read** = read from code or config, not run. `<scratch-dir>` stands for a probe's temp directory.

## How it was run

### OpenCode half (1.18.35)

**Version tested: `opencode 1.18.35`** (`opencode --version`). The adapter's Part A was written from the OpenAPI document of 1.19.0. Only 1.18.35 is installed
on the machine that ran this; nothing was installed or downloaded, so **every finding below is for 1.18.35 and none is for 1.19.0.** Differences between the two
are unobserved; the probe is a script, so it can be re-run against 1.19.0 unchanged.

- `scripts/spikes/opencode-compact.sh`, built on `scripts/spikes/opencode-lib.sh` (same safety contract: `env -i`, `HOME` and `XDG_*` in a fresh `mktemp -d`, ports on
  127.0.0.1 in 48100-48199, a private tmux socket, an EXIT trap that kills only what the run started and removes the directory).
- `scripts/spikes/opencode-stub-model.py` replaces the lib's dead-end provider: an OpenAI-compatible, streamed chat-completions stub on 127.0.0.1 with a fixed
  `usage` block (turn: 1000 prompt + 50 completion; summary: 400 + 60), fixed text, a switch that holds a reply open, a switch that answers HTTP 500, and a log of every
  request's `model`, message count and body keys. The scratch config has that one provider only (`enabled_providers: ["stubprov"]`, built-in provider disabled).
  `oc_guard_stub_only` refuses to continue unless `GET /config` and `GET /config/providers` show exactly that one provider on the stub's loopback URL. No model, key,
  token or login exists in the run; nothing real was reached.
- Each case runs on a session the probe created, with three short turns first. The run took about 2.5 minutes (70 s of it is case 8's retries). It was run several
  times with the same results; the final run had 0 FAIL.
- The recorded reply bodies are in `docs/research/compaction-spike-fixtures/opencode/`, one file per route and case, ids replaced (`ses_<id1>`...), scratch directory
  as `<scratch-dir>`, ports as `<port>`. They are the input for the follow-up fixture swap.

### Claude half (2.1.296)

**Run.** Claude Code 2.1.296, model `haiku`, subscription login, one scratch session started by the probe
with the adapter's argv shape (`--session-id`, `--settings` with `crossSessionInbound: accept` and the
`SessionStart` hook that writes the inbox socket path, `--disallowed-tools AskUserQuestion`). Everything
ran under `env -i` with only `HOME`, `PATH`, `TERM`, `LANG` and the hook's directory variable (names only;
no `CLAUDE_CODE_*`, no `ANTHROPIC*`), on a private tmux socket, in a fresh working directory inside an
already trusted directory (no dialog appeared). **Model turns used: 3 of 3** (seed, the model's reply to the
inbox `/compact`, the typed control's summary call). The planned follow-up turn was not run: it would have
been a fourth turn (recorded as `INFO`).

## What exists in OpenCode 1.18.35

**Documented.** Both routes are in the running server's OpenAPI document.

| Route | operationId | Request body | Responses |
|---|---|---|---|
| `POST /session/{sessionID}/summarize` | `session.summarize` | `{"type":"object","properties":{"providerID":{"type":"string"},"modelID":{"type":"string"},"auto":{"type":"boolean"}},"required":["providerID","modelID"],"additionalProperties":false}` | `200` "Summarized session" (`application/json`, schema `{"type":"boolean"}`), `400` BadRequest or InvalidRequestError, `404` NotFoundError |
| `POST /api/session/{sessionID}/compact` | `v2.session.compact` | none | `204`, `400`, `401`, `404` SessionNotFoundError, `503` ServiceUnavailableError |

**Verified**: `summarize` works. **`/api/session/:id/compact` does not: it answers `503 {"_tag":"ServiceUnavailableError","message":"Session compact is not available yet","service":"session.compact"}`
on every session, idle or busy** (fixture `compact-reply.json`). It takes no model and no body in the document either way. The v2 read routes next to it do answer
(`GET /api/session/:id` 200, `GET /api/session/:id/context` 200 `{"data":[]}`), so the 503 is the route, not the v2 API. `compact` is therefore not usable on
1.18.35; `summarize` is the only route to build on. On an unknown session `compact` answers `404 {"_tag":"SessionNotFoundError",...}` (`compact-unknown-session-reply.json`),
`summarize` answers `404 {"name":"NotFoundError","data":{"message":"Session not found: ses_<id1>"}}`.

## Answers Q1-Q7

### OpenCode (Q1-Q6)

#### Q1. Returns when finished, or at once? How long?

**Verified: it returns when the summary is finished, not at once; no polling is needed.** With the stub holding the summary request open, the `summarize` call stayed pending
for the whole hold (still pending at 4 s; it returned `200 true` 4.1 s after the call started, immediately after the hold was released). With an instant stub it returns in
roughly 70-160 ms (model latency plus the server's own work). While the call was pending the message list already held the (empty) compaction marker and summary message, so
the in-flight state is visible, but the call itself is the completion signal.

The one long case is a failing model call (Q6): the call took **70.9 s** to return, because the server retries the model request five times with growing delays (6 requests in
all) before giving up. That is inside the 120 s bound but not by a wide margin; a model provider that is slow to fail (a connect timeout per attempt) can exceed it.
**Part A's 120 s bound and "no polling" are confirmed for the success path**, with the caveat that the failure path consumed 71 s of it with the stub failing instantly.

#### Q2. Exact request, reply, route differences

**Verified** (replies are in the fixtures directory, verbatim apart from ids):

| Call | Status | Content type | Body |
|---|---|---|---|
| `POST /session/:id/summarize` `{"providerID":"stubprov","modelID":"stub-other"}` | `200` | `application/json` | `true` (`summarize-a-explicit-model-reply.json`) |
| same, body `{}` | `400` | `application/json` | `{"name":"BadRequest","data":{"message":"Missing key\n  at [\"providerID\"]","kind":"Payload"}}` |
| same, no body at all | `400` | `application/json` | `{"name":"BadRequest","data":{"message":"Expected object, got undefined","kind":"Payload"}}` |
| same, `{"providerID":"stubprov"}` | `400` | `application/json` | `... Missing key\n  at [\"modelID\"] ...` |
| same, unknown provider or unknown model | `500` | `application/json` | `{"name":"UnknownError","data":{"message":"Unexpected server error. Check server logs for details.","ref":"err_..."}}` |
| same, unknown session `ses_...` | `404` | `application/json` | `{"name":"NotFoundError","data":{"message":"Session not found: ses_<id1>"}}` |
| same, id not starting with `ses` | `500` | `application/json` | the same `UnknownError` (the document says the path pattern is `^ses`; the server does not answer 400 for it) |
| `POST /api/session/:id/compact` | `503` | `application/json` | `{"_tag":"ServiceUnavailableError","message":"Session compact is not available yet","service":"session.compact"}` |

- **The accepted status is `200` with the JSON boolean `true`** (Part A: "accept only the one status the document gives": confirmed; the success body is `true`, not an object).
- `summarize` needs `providerID` and `modelID` in the body; with the pin present they are still required. Two error shapes exist (`{"name","data"}` on the v1 route,
  `{"_tag",...}` on the v2 route); neither carries a secret.
- Extra body keys are **accepted and ignored** although the document says `additionalProperties:false`: `variant:"high"` returned `200 true` and the stub request was
  identical (same model, same message count, same body keys; the stub's models define no variants, so "ignored" is for that case; the pin's `variant` was not exercised).
- `auto:true` is accepted and **Verified** to do something extra: after the summary the server added a user message and an assistant reply from the model named in the
  body (the stub saw a second request, kind turn, `stub-other`). `auto` therefore means the automatic, overflow-triggered path: summarize and then carry on with the
  conversation. Part A does not send it, which is right: `auto:false`/absent summarizes and stops.
- The session id is unchanged by `summarize` (**Verified**; `GET /session` lists the same single session; `GET /session/:id` has the same `id`, same `title`).

#### Q3. Which model runs the summary

**Verified by the stub's request log**, which is the model the server asked for:

| Case | Server | Body | Model that ran |
|---|---|---|---|
| (a) explicit | no `agent.compaction` | `stubprov/stub-other` | `stub-other` |
| (b) pin and a different model in the body | `agent.compaction.model = stubprov/stub-compact` | `stubprov/stub-other` | **`stub-compact`** (the pin wins) |
| (b2) pin, body `{}` | pinned | `{}` | nothing: `400` (the body is still required) |
| (b3) pin, body names an unknown provider | pinned | `nosuchprovider/x` | nothing: `500` (the body's model is resolved before the pin applies) |
| (c) no pin, no usable body | none | `{}`, none, or one key | nothing: `400` |
| (d) unknown provider, or known provider and unknown model | none | e.g. `nosuchprovider/x` | nothing: `500 UnknownError`, zero requests reach the stub |

- **A pin is honoured over the body, and a body is needed anyway.** The server never falls back to its configured default model (`model`) for a summary: with no usable
  body it refuses. So an adapter that sends the pinned `provider/model` in the body (Part A) runs exactly the model in `agent.compaction`; one that sends anything else on a
  pinned server still gets the pin. `compact-model-unpinned` is therefore a **Holler rule, not a server one**: the server would happily summarize on whatever model the body names
  when nothing is pinned (case (a)). Confirmed as a deliberate choice, not a protocol need.
- **`GET /config` shows the pin** (**Verified**): the scratch config's `agent` block comes back as `"agent":{"compaction":{"model":"stubprov/stub-compact","options":{},"permission":{}}}`
  (fixture `config-pinned.json`). The server adds `options` and `permission`; the pin keeps its `provider/model` form (split at the first `/`: `stubprov` / `stub-compact`).
  An unpinned server returns `"agent"` absent or `{}` (`{}` was seen). A `variant` under `agent.compaction` (what `pane-restart` writes when it pins one) was not exercised.
- `variant` in the body: accepted and ignored (see Q2). `auto`: see Q2.

#### Q4. A busy session: refused, queued, or run on top?

**Verified: queued behind the turn.** With a turn held open at the stub, `GET /session/status` reported `{"ses_<id1>":{"type":"busy"}}`; `summarize` on that session
did not answer for the 3 s watched and the stub saw **no** new request in that time (count unchanged). After the turn was released the summary request followed (stub log: the held
turn, then one `summary` request), and `summarize` returned `200 true`; the summary message ended up after the turn's reply. So the server does neither refuse nor run on top: it waits for the turn.

Consequence for the adapter: the server serializes, so a `summarize` sent into a busy session blocks until the turn ends (and then compacts whatever the turn produced). Part A's own
re-check (refuse `session-busy` before sending, ADR-0021) is **still needed**: without it the call would sit in the 120 s bound for the length of a turn, and would compact a conversation
the caller did not mean. `/api/session/:id/compact` on a busy session also answers `503` at once (the same body).

#### Q5. Token source, before and after; session id unchanged

**Verified** (field names verbatim):

| Candidate | Before (3 turns) | After `summarize` | Tracks the context? |
|---|---|---|---|
| last assistant message `info.tokens` in `GET /session/:id/message` | `{"total":1050,"input":1000,"output":50,"reasoning":0,"cache":{"read":0,"write":0}}` | `{"total":460,"input":400,"output":60,"reasoning":0,"cache":{"read":0,"write":0}}` | yes, with a caveat below |
| session object `GET /session/:id` `.tokens` | `{"input":3000,"output":150,"reasoning":0,"cache":{"read":0,"write":0}}` | `{"input":3400,"output":210,...}` | **no**: cumulative usage of every call, only ever grows |

- **The last assistant message's `info.tokens.total` is the right source, as Part A assumed**: before it is the last turn's prompt plus output (the size of the context at that turn); after, the last
  assistant message is the summary message (`info.mode:"compaction"`, `info.summary:true`), whose `tokens` are the summary call's own usage. It is smaller (460 < 1050), so the shrink rule passes.
- **Caveat (a contradiction of an implicit Part A assumption):** that "after" number is the size of the **summary request** (its prompt, 400 here, plus the summary's output), not a measured size of
  the new context. The real post-compaction context size appears only on the next turn. It is a faithful "the context is now about this big" only to the extent the summary request carried the whole old
  history; with a real model the two can differ. The shrink rule `tokens_after < tokens_before` is still sound as a necessary check; `tokens_after` should be described as "tokens of the summary message".
- **Verified the context did shrink for the model**: the first turn after the summary was sent 4 messages where the third turn before it had been sent 6
  (the stub's `nmsg`; 9701 characters in that earlier request).
- The session id is unchanged, no message is deleted: `summarize` only **appends** two messages (a user message with a `compaction` part, then an assistant message `mode:"compaction"`, `summary:true`,
  `finish:"stop"`, whose text part is the summary). History stays in the list; the server stops sending the part before the marker to the model. Fixtures: `messages-before-summarize.json`,
  `messages-after-summarize.json`, `session-after-summarize.json`.
- The TUI (private tmux, `opencode attach` on the scratch server, session selected through `POST /tui/select-session`): the pane title was `OC | timed` before, during and after the call and did not change; the
  screen showed the summary text afterwards. The title is not a completion signal.

#### Q6. Failure: is the history intact, and what does the call return?

**Verified, and this contradicts what Part A assumed about the reply.** The stub answered HTTP 500 to every summary request:

- **The call returned `200` with body `true`, after 70.9 s**, having sent the model request six times (one try plus five retries with growing delays). A failed model call does not surface as an
  error status. (`summarize-stub-500-reply.json` is `true`.)
- The message list afterwards: all earlier messages intact and in order (**Verified**, PASS), plus two new ones: a user message with a `compaction` part, and an assistant message `mode:"compaction"`,
  `summary:true` with **`parts: []`, `finish` and `info.tokens` absent** (`messages-after-failed-summarize.json`). Session cumulative `.tokens` unchanged; `GET /session/status` returns `{}` (idle).
- **The stored history is intact, but the model no longer sees it.** The next ordinary turn on that session (answered `200`) reached the stub with **2 messages** (system and the new input) where the
  third turn before had 6, and its reply was written as a `mode:"compaction"` assistant message. A failed summarize therefore leaves the session in a worse state than before: an empty summary stands
  where the history was.
- Consequence for the adapter: **do not trust the status.** Success needs the post-read: the last assistant message must be `summary:true` **and** have `info.tokens` (and `finish`); `tokens` absent means
  the summary did not happen and the result is `unavailable` (never a zero count), and the error text should say the pane's session now carries an empty summary marker. The shrink rule catches this only
  if a missing count is treated as a failure rather than as 0 (it must be). Whether the empty marker can be undone through the API (revert, delete messages) was not probed.

#### Q7. Claude Code `/compact`

Findings (Claude Code 2.1.296):

| # | Finding | Label |
|---|---|---|
| 1 | The scratch session registered `idle` in `claude agents --json` with no dialog; the `SessionStart` hook wrote the inbox socket path under the claude pid (the `$PPID` key holds). | Verified |
| 2 | A seed turn ("Reply with the single word: ok") through the inbox frame `{"type":"user","message":{"role":"user","content":TEXT}}` ran as a turn: `busy` then `idle`, a reply in the transcript. The pane shows it as "Another Claude session sent a message". | Verified |
| 3 | **`/compact` sent through the inbox does not compact.** The text reached the model as an ordinary peer message (the pane shows "Another Claude session sent a message: /compact"); status went `busy` then `idle`; the transcript gained a user entry and an assistant reply and **no** `compact_boundary` and no `isCompactSummary` entry. The model answered, in words, that it cannot run `/compact` from inside the session and that the user should type it. That turn cost a model call and produced nothing. | Verified |
| 4 | **Control (typed by hand into the scratch TUI, scratch only, never an adapter action):** `/compact` typed with Enter compacts. The pane prints "Conversation compacted"; the transcript gains a `system` entry with `subtype: "compact_boundary"`, a user entry flagged `isCompactSummary: true`, and the session id is unchanged. It took about 4 s. | Verified |
| 5 | Status values seen with 150 ms sampling of `agents --json`, for the inbox attempt and for the typed control alike: **`idle`, `busy` only.** There is **no `compacting` value**, so even a working compaction would look like an ordinary busy turn. | Verified |
| 6 | `agents --json` entry fields: `cwd, kind, name, pid, sessionId, startedAt, status`. No token, context or usage field. | Verified |
| 7 | **A token source exists, but only after a compaction that was typed.** The `compact_boundary` entry carries `compactMetadata` with `trigger` (`manual`), `preTokens` (33983 in this run), `postTokens` (5103), `cumulativeDroppedTokens` (28880) and `durationMs` (4009). Every assistant entry also carries `message.usage` (`input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`), which tracks the context before a compaction. Both are read from the transcript file at `<config dir>/projects/<cwd as "-">/<session id>.jsonl` (the documented path rule). | Verified (the fields); Documented (the path rule) |
| 8 | The pre-compaction context size is in the last assistant `usage` (`cache_read` + `cache_creation` + `input`), without typing anything. The post-compaction size is `postTokens` of the boundary entry, which exists only once a compaction ran. | Verified |
| 9 | The transcript is an internal file format, not a documented interface; its entry types and fields can drift per version. | Read |

Q7 answer:

| Question | Answer |
|---|---|
| Does `/compact` over the inbox compact? | **No.** It is delivered as a peer message, the model replies as text and says the user must type it (Verified, finding 3). |
| Status value during compaction | None of its own: `busy` then `idle` (Verified, finding 5). `agents::activity` has no `compacting` to learn. |
| Token source | `compactMetadata.preTokens` / `postTokens` in the transcript's `compact_boundary` entry, and per-turn `message.usage` before it. Readable without typing, but only after a compaction that something else has caused (Verified, findings 7 and 8). |


## Part A's assumptions: confirmed and contradicted

| Assumption | Verdict | Evidence |
|---|---|---|
| Request body `{providerID, modelID}`, no `auto`, no `variant` | **Confirmed** (Documented + Verified). Both keys required; `auto` and `variant` accepted but not needed | Q2 |
| Accepted status is the one the document gives (`200`) | **Confirmed**: `200` with JSON `true` | Q2 |
| ... and an error status reports a failed summary | **Contradicted**: a failing model call returns `200 true` after ~71 s and leaves an empty `summary:true` message | Q6 |
| Token source is the last assistant message's `info.tokens` | **Confirmed** as a source; the after-count is the summary message's own usage; the session object's `tokens` is cumulative and unusable | Q5 |
| `GET /config` exposes `agent.compaction` as the pin | **Confirmed**: `{"model":"<provider>/<model>","options":{},"permission":{}}`; the model that ran was the pinned one even when the body named another | Q3 |
| 120 s bound, no polling | **Confirmed** for success (returns when finished); failure alone took 71 s | Q1 |
| `POST /api/session/:id/compact` as an alternative | **Contradicted for 1.18.35**: always `503` "not available yet" | route table |
| A busy session needs the adapter's own re-check | **Confirmed**: the server queues instead of refusing | Q4 |
| Claude Code stays `compact-unsupported` (Part A's default) | **Confirmed**: `/compact` over the inbox is delivered as a peer message and the model answers in words that the user must type it | Q7, finding 3 |
| `agents::activity` maps only `idle`, `busy`, `waiting`; any other status is `unavailable` (a `compacting` value was possible) | **Confirmed, nothing to learn**: only `idle` and `busy` were seen, during the inbox attempt and the typed control alike | Q7, finding 5 |


## Hand-off

What the code must do next.

1. **B.1 confirms Part A**, except one point: a failed summary returns `200 true` after about 71 s with an empty `summary:true` marker. The merged adapter already turns this into
   `unavailable`, because the last assistant message has no tokens. Do not trust the status; success needs the post-read (last assistant message `summary:true` with `info.tokens`).
2. Remaining small items:
   - The error text should say the pane's session now carries an empty summary marker, and the elapsed time (surface it in the `timeout` message; never retry the verb).
   - `tokens_after` is the size of the summary message (the summary call's own usage), not a measured size of the new context; describe it so.
   - The fixtures are to be swapped for the recorded ones in `docs/research/compaction-spike-fixtures/opencode/`.
3. Keep to `POST /session/:id/summarize`; `POST /api/session/:id/compact` answers `503` ("not available yet") in 1.18.35. Do not build on it until a version answers `204`.
4. Send the pinned `provider/model` in the body; `compact-model-unpinned` is Holler's own safeguard, not a server rule (Q3).
5. Re-run `scripts/spikes/opencode-compact.sh` against 1.19.0 when it is available; the fixtures here are for 1.18.35.
6. **Claude Code stays `compact-unsupported`.** The only way found to compact is a person typing `/compact`
   into the TUI, which invariant I4 forbids the adapter from doing, and the inbox frame cannot carry a slash
   command: it arrives as a message and costs a model turn. The token source (finding 7) would let a future
   adapter report counts if Claude Code ever accepts a compact request over a non-typing channel, but nothing
   that was observed provides that channel. The docs should say: "Claude Code compacts only when a person
   types `/compact` in the pane or when it compacts itself; Holler cannot request it."

   Open questions not tested (budget): whether a context-limit autocompaction writes the same `compact_boundary` entry (`trigger` other than `manual`), and whether a `PreCompact`
   hook or a documented SDK/`-p` route can start a compaction. Either would be a new spike.

## Cleanup

### OpenCode probe
**Verified** after each run: no process of the run left (server, stub, TUI client), the private tmux server gone, `<scratch-dir>` removed, nothing under any real OpenCode configuration read or written (the
probe's `HOME` and `XDG_*` were scratch). The EXIT trap prints `cleanup: killed process groups [...], private tmux server, temp dir removed`.

### Claude probe
The private tmux server was killed, the claude process was the one started by this run (its parent was the
private tmux server; checked by pid and start time, no pattern kill), the scratch working directory, the
scratch directory under a short `/tmp` path and the single transcript directory of the scratch working
directory were removed, and each was confirmed gone afterwards. No other claude process or config-directory
path was touched.
