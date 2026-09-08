# A2A 1.0 fixtures

JSON examples of the A2A object model (`Message`, `Part`) that
`holler_proto::a2a` must round-trip **byte-for-byte** (after canonical
JSON), proving the Rust types are shape-identical to the A2A JSON schema.

## Pinned revision

These fixtures are taken from the **A2A protocol specification, version
1.0.1**:

| Field | Value |
|---|---|
| URL | <https://a2a-protocol.org/latest/specification/> |
| Version | **v1.0.1** |
| Tag | `v1.0.1` |
| Published | **2026-05-28** |

(For reference, A2A **v1.0.0** was published **2026-03-12**.) ADR 0004
pins this revision as the source of the object model Holler reuses
verbatim on the inward wire.

## Convention

A2A's JSON binding serialises enum values as their **protocol-constant
strings** — `role: "ROLE_USER"` / `"ROLE_AGENT"` and
`state: "TASK_STATE_*"` — not the lowercase kebab-case forms.
`holler_proto::a2a` matches that binding exactly (see `a2a.rs`). Note
that Holler's *Holler* wire uses the lowercase forms (`user`/`agent`,
`working`/`input-required`, …) for the ADR 0005 presence `state` — a
Holler choice, distinct from the A2A constant form these fixtures use.

## Files

| File | Covers |
|---|---|
| `message_text.json` | a `Message` with `metadata` + `extensions` and a **text** `Part` — the spec's canonical Message example (A2A spec §6, geolocation) |
| `part_raw.json` | a **raw** `Part` (base64 bytes + `filename` + `mediaType`) in a Message, plus a second text Part |
| `part_url.json` | a **url** `Part` in a Message |
| `part_data.json` | a **data** `Part` (a JSON array) in a Message — the member-name discriminator is `data` |

Each file is a single top-level **`Message`** object. `codec_test.rs::
a2a_fixtures_round_trip` deserialises each through
`holler_proto::a2a::Message`, re-serialises with `serde_json::to_string`
(sorted keys), and asserts the result equals the file's canonical form.