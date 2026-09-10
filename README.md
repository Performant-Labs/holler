# holler

Your agents are just a holler away — one binary, hub or body.

This repository is under construction — see the [Phase 0 epic](https://github.com/Performant-Labs/holler-server/issues/320) for what is being built here and why.

## Documentation

See [docs/README.md](docs/README.md) — the index. For testing: [the harness design](docs/testing.md) and [how to run the tests](docs/running-tests.md).

## Attach convenience

`holler body attach` removes the "curl the endpoint and hand-edit a TOML" dance for attaching to an OpenCode session another process already owns (a Herdr pane, or a bare `opencode serve`):

```
holler body attach sessions --endpoint http://127.0.0.1:4096
```

Lists that endpoint's sessions as a `SESSION_ID TITLE UPDATED` table, newest first (`--json` prints a `{"sessions": [...]}` document instead). `--endpoint` defaults to `http://127.0.0.1:4096`; a non-2xx or unreachable endpoint exits 1, naming the URL(s) tried.

```
holler body attach init --endpoint http://127.0.0.1:4096 --session ses_1 --name alpha --out attach.toml --force
```

Writes a ready `mode = "attach"` `[[session]]` row to `--out` (default `./attach.toml`) and prints the next command to run. `--session` defaults to the endpoint's newest session (from the same listing `sessions` uses) and `--name` defaults to `alpha`. Refuses to overwrite an existing `--out` file unless `--force` is given (exit 3). Neither verb ever spawns a process, prompts a model, or touches Herdr — it is pure HTTP plus a file write.
