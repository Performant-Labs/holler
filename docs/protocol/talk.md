# Talk — attach mode

The "talk" surface (`say`/`interrupt`/`answer` → `session/prompt`/`session/
cancel`/`session/answer`, §6/§10 of [v2](v2.md)) is dispatched identically
regardless of a session's `mode` — `SessionManager` picks the driver
(`AcpDriver` for `spawn`, `HttpAttachDriver` for `attach`, issue #195) once,
at the session-task boundary, and every wire leaf above it is unaware which
one is underneath.

## Attach-mode turn

```
operator          hub               body (session task)      HttpAttachDriver        OpenCode process
   |  say alpha "…" |                       |                        |                        |
   |--------------->|  session/prompt       |                        |                        |
   |                |---------------------->|  driver.prompt(text)   |                        |
   |                |                       |----------------------->|  POST /session/{id}/    |
   |                |                       |                        |  prompt_async          |
   |                |                       |                        |----------------------->|
   |                |                       |                        |   204 (fire-and-forget)|
   |                |                       |                        |<-----------------------|
   |                |                       |                        |  GET /event (SSE)      |
   |                |                       |                        |<======================>|
   |                |                       |  DriverEvent::Chunk    |  message.part.updated  |
   |                |                       |<-----------------------|<-----------------------|
   |                |  session/update       |                        |                        |
   |<---------------|<----------------------|                        |                        |
   |                |                       |  DriverEvent::Done     |  session.idle          |
   |                |                       |<-----------------------|<-----------------------|
   |  reply (final) |  session/prompt result|                        |                        |
   |<---------------|<----------------------|                        |                        |
```

Interrupt (`interrupt alpha`, no replacement text) is the same shape with
`session/cancel` → `driver.cancel()` → `POST /api/session/{id}/interrupt`
(note: **`/api`-prefixed**, unlike `prompt_async` — the two routes were
independently confirmed live to need different prefixes; see
`crates/holler-body/src/http_attach_driver.rs`'s own module doc) → the same
`/event` SSE stream reports `session.idle`, which resolves the cancel with
the driver's real `StopReason` (`Cancelled`, unless the OpenCode process
itself had already settled the turn some other way).

**Shutdown/detach/SIGINT stop at the body↔driver boundary**: `HttpAttachDriver::
shutdown` only ends this body's own background SSE/poll loop. It never sends
anything to the OpenCode process on the right — there is no HTTP `DELETE`,
no signal, nothing to kill (this driver never spawned a child in the first
place). The OpenCode process keeps running, and the *next* `body run` to
attach the same `session_id` picks up exactly where this one left off.
