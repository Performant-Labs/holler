# Golden wire files (issue #155 §4)

Checked-in JSON for every wire shape, as the encoder emits it. `golden_test.rs`
asserts encoder output == file (canonical, sorted-keys form), so **changing the
wire changes a file in this directory** and shows in every PR diff.

| Dir | What | Regenerate |
|---|---|---|
| `envelope/` | one file per `Envelope` shape: request, notification, response, error | `BLESS=1 cargo test -p holler-proto --test golden_test` — then review the diff before committing |
| `docs/` | one file per params/result type in `docs.rs` (both roles for `Hello`/`Status`) | same |
| `foreign/` | frames **written by hand from the JSON-RPC 2.0 and A2A v1.0.1 spec text**, never produced by our encoder; each must decode, or be rejected for an asserted, documented reason | never regenerated — hand-edited only |

`envelope/error.json` is hand-written in the JSON-RPC-conformant form
(`error.code` is a **number**). The encoder does not yet produce that (#145);
its test is `#[ignore]`d with that link until #145 lands. The golden file is
the spec of record, not a snapshot of the bug.
