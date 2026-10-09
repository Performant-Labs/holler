# Evidence — #681 the JSON-envelope checker (source facts outside the diff)

## F (Phase 6, implementation)

- **Fact:** `is_valid_code` is the one code validator the checker calls for rule 11 (`^[a-z]+(-[a-z]+)*$`).
  **Source:** `crates/holler-pane/src/error.rs:158-163`
  **Verbatim excerpt:**
  > /// Whether `code` is a well-formed error code: `^[a-z]+(-[a-z]+)*$` (lower-case
  > /// ASCII words joined by single hyphens, no leading or trailing hyphen).
  > ///
  > /// This is the only code validator in the workspace; the CLI's output module calls
  > /// it. It is a `const fn`, so a verb's code constant can be checked at build time.
  > pub const fn is_valid_code(code: &str) -> bool {

- **Fact:** a class turns into the exit code the checker compares in rule 13: usage 2, refusal 3, failure 1.
  **Source:** `crates/holler-pane/src/error.rs:239-245`
  **Verbatim excerpt:**
  > pub const fn exit_code(self) -> i32 {
  >     match self {
  >         ErrorClass::Usage => 2,
  >         ErrorClass::Refusal => 3,
  >         ErrorClass::Failure => 1,
  >     }
  > }

- **Fact:** `class_of` makes any well-formed code that is not closed a refusal and a malformed code a failure. Rule 11 runs
  first, so the checker only ever passes it a well-formed code.
  **Source:** `crates/holler-pane/src/error.rs:266-273`
  **Verbatim excerpt:**
  > pub fn class_of(code: &str) -> ErrorClass {
  >     let Some(closed) = PaneCode::parse(code) else {
  >         return if is_valid_code(code) {
  >             ErrorClass::Refusal
  >         } else {
  >             ErrorClass::Failure
  >         };
  >     };

- **Fact:** the CLI's envelope has exactly the four keys the checker requires (rules 3 and 4), and `data` is `null` on
  failure.
  **Source:** `crates/holler-cli/src/output.rs:152-160`
  **Verbatim excerpt:**
  > /// Keys are written in this order: `schema_version`, `ok`, `data`, `error`. `data` is `null` when
  > /// `ok` is false and `error` is `null` when it is true.
  > #[derive(Debug, Serialize)]
  > pub struct Envelope<T> {
  >     pub schema_version: u32,
  >     pub ok: bool,
  >     pub data: Option<T>,
  >     pub error: Option<ErrorBody>,
  > }

- **Fact:** the CLI's NDJSON writer stops at the first item with a non-zero exit code, so a failure can only be the last
  line. That is the rule `check_ndjson` enforces with `NotLastFailure`.
  **Source:** `crates/holler-cli/src/output.rs:225-237`
  **Verbatim excerpt:**
  > pub fn emit_stream<T: Serialize>(
  >     sink: &mut Sink<'_>,
  >     format: Format,
  >     items: impl Iterator<Item = Result<T, ErrorBody>>,
  >     text: impl Fn(&T) -> String,
  > ) -> i32 {
  >     for item in items {
  >         let code = emit(sink, format, item, &text);
  >         if code != 0 {
  >             return code;
  >         }
  >     }
  >     0

- **Fact:** `serde_json`'s stream deserializer skips leading whitespace before a value, and sets `byte_offset()` to the
  byte just after a value it parsed. So `frame` checks the first byte itself (rule 1a), and reads `byte_offset()` after
  the first `next()` to find text after the value. `{`, `[` and `"` are self-delimiting, so a value starting with them is
  `Ok` whatever follows it (rule 1b relies on this).
  **Source:** `serde_json-1.0.151/src/de.rs:2456-2483` (the version `Cargo.lock` pins, in the cargo registry)
  **Verbatim excerpt:**
  > // skip whitespaces, if any
  > // this helps with trailing whitespaces, since whitespaces between
  > // values are handled for us.
  > match self.de.parse_whitespace() {
  >     Ok(None) => {
  >         self.offset = self.de.read.byte_offset();
  >         None
  >     }
  >     Ok(Some(b)) => {
  >         // If the value does not have a clear way to show the end of the value
  >         // (like numbers, null, true etc.) we have to look for whitespace or
  >         // the beginning of a self-delineated value.
  >         let self_delineated_value = match b {
  >             b'[' | b'"' | b'{' => true,
  >             _ => false,
  >         };
  >         self.offset = self.de.read.byte_offset();
  >         let result = de::Deserialize::deserialize(&mut self.de);
  >
  >         Some(match result {
  >             Ok(value) => {
  >                 self.offset = self.de.read.byte_offset();
  >                 if self_delineated_value {
  >                     Ok(value)
  >                 } else {
  >                     self.peek_end_of_value().map(|()| value)
  >                 }
  >             }

- **Fact:** `serde_json::Map` is a `BTreeMap` (sorted keys) without `preserve_order` and an `IndexMap` (document order)
  with it. A `--workspace` build turns the feature on through a dependency of `holler-body`, a `-p holler-pane-testkit`
  build does not (`cargo tree -e features -i serde_json`). So `exact_members` names the smallest extra key, not the first
  in map order.
  **Source:** `serde_json-1.0.151/src/map.rs:33-36`
  **Verbatim excerpt:**
  > #[cfg(not(feature = "preserve_order"))]
  > type MapImpl<K, V> = BTreeMap<K, V>;
  > #[cfg(feature = "preserve_order")]
  > type MapImpl<K, V> = IndexMap<K, V>;

- **Fact:** `Value` derives `Eq`, so `Envelope` (which holds a `Value`) can derive `Eq` as the brief's API requires.
  **Source:** `serde_json-1.0.151/src/value/mod.rs:115-116`
  **Verbatim excerpt:**
  > #[derive(Clone, Eq, PartialEq, Hash)]
  > pub enum Value {

- **Fact:** the contract the checker enforces: one envelope with these keys, `error` null exactly when `ok`, no `detail`,
  NDJSON for `pane watch`, and exit codes 0/1/2/3.
  **Source:** `docs/adr/ADR-0021.md:353-361`
  **Verbatim excerpt:**
  > {"schema_version": 1, "ok": true, "data": {}, "error": null}
  > {"schema_version": 1, "ok": false, "data": null, "error": {"code": "pane-not-found", "message": "pane not found: hj-c1r1"}}
  > ```
  >
  > - `error` is `null` exactly when `ok` is true. `error.code` is the `PaneError` code verbatim and `error.message` its one-line
  >   text; the envelope has no `detail`. Each verb supplies only its `data`. The types and `emit()`, `emit_stream()` and
  >   `emit_usage_error()` are in `holler-cli/src/output.rs` (#670 creates it, #660 completes it).
  > - `pane watch` in JSON mode writes NDJSON: one envelope per line, flushed per line.
  > - **Exit codes are the same in both formats (operator, 2026-10-09):** 0 ok, 1 runtime failure, 2 usage, 3 refusal. A

## Added by T (Phase 7)

The `preserve_order` removal order, which the rows `extra-keys-among-known-keys-smallest-first` and
`error-extra-keys-among-known-keys-smallest-first` depend on. Copied from `serde_json-1.0.151/src/map.rs` below.

```
serde_json-1.0.151/src/map.rs:150-153,158-163
    /// If serde_json's "preserve_order" is enabled, `.remove(key)` is
    /// equivalent to [`.swap_remove(key)`][Self::swap_remove], replacing this
    /// entry's position with the last element. ...
    pub fn remove<Q>(&mut self, key: &Q) -> Option<Value>
    ...
        #[cfg(feature = "preserve_order")]
        return self.swap_remove(key);
        #[cfg(not(feature = "preserve_order"))]
        return self.map.remove(key);
```
