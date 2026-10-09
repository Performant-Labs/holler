//! The JSON-envelope checker every verb's `--format=json` tests reuse: stdout is one
//! envelope (or NDJSON lines of them) with `schema_version` 1, `ok` agreeing with the
//! exit code, `error` null exactly when `ok`, and nothing else. Empty stub declared by
//! #638 so that no two slices edit `lib.rs`; slice b (#681) fills it.
