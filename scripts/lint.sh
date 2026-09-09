#!/usr/bin/env bash
# Build guards (issue #149): the #145–#148 defect classes must fail CI, not
# rely on a reviewer noticing. Runs as the FIRST CI step, before the canary.
set -euo pipefail
fail=0

# 1. No blanket dead-code allows, and no #[allow(...)] without an issue link
#    on the same line. Exception: a forward-contract stub (a helper shipped
#    ahead of its first caller, e.g. the test harness's Hub::start/join) may
#    allow(dead_code) — but only if that allow carries a trailing `// #NNN`
#    issue link naming the story that lands the caller. Otherwise dead code
#    stays dead: land helpers with their first caller.
#    An `allow(...)` can also arrive wrapped inside `cfg_attr(cond, allow(...))`
#    (#248) — that hides the same bypass from a regex anchored on `#!?\[allow\(`,
#    so both checks below also match `allow(` reached via `cfg_attr\([^)]*`.
if grep -rn --include='*.rs' -E '(#!?\[allow\([^)]*dead_code[^)]*\)|cfg_attr\([^)]*allow\([^)]*dead_code[^)]*\))' crates/ \
    | grep -vE '//\s*#[0-9]+'; then
  echo "lint: allow(dead_code) without a '// #NNN' link is forbidden — land helpers with their first caller (or link the forward-contract story)"
  fail=1
fi
if grep -rn --include='*.rs' -E '(#!?\[allow\(|cfg_attr\([^)]*allow\()' crates/ \
    | grep -vE '//\s*#[0-9]+' \
    | grep -v 'allow(clippy::assertions_on_constants)'; then
  echo "lint: every #[allow] needs a trailing '// #NNN' issue link"
  fail=1
fi

# 2. process::exit only in a bin's main.rs (any bin: holler's src/main.rs or
#    the test stub's tests/stub-acp/main.rs). A helper must panic, never exit
#    — an exit in a helper masks a failure from the caller (defect #147).
if grep -rn --include='*.rs' 'process::exit' crates/ | grep -v '/main.rs:'; then
  echo "lint: process::exit outside a bin's main.rs (a helper must panic, never exit)"
  fail=1
fi

# 3. CARGO_BIN_EXE_* is set by cargo per test target — read it with env! at
#    compile time, not env::var at runtime (defect #147).
if grep -rn --include='*.rs' -E 'env::var\("CARGO_BIN_EXE' crates/; then
  echo 'lint: use env!("CARGO_BIN_EXE_…"), not env::var'
  fail=1
fi

# 4. File-size gate: warn at 600 lines, fail at 900 (skill rule: never let a
#    file cross 1k unnoticed).
while read -r n f; do
  if [ "$n" -ge 900 ]; then
    echo "lint: $f is $n lines — decompose before merging"
    fail=1
  elif [ "$n" -ge 600 ]; then
    echo "warn: $f is $n lines"
  fi
done < <(find crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | grep -v ' total$')

# 5. Dependency features must name their consumer, or the set is unreviewable.
if grep -nE 'features\s*=' Cargo.toml crates/*/Cargo.toml 2>/dev/null \
    | grep -vE '#.*(used by|consumer|for )' \
    | grep -v 'features.workspace'; then
  echo "lint: every dep feature needs a comment naming its consumer"
  fail=1
fi

# 6. Golden-file blessing side effect (#246): BLESS=1 reorders every golden
#    file's keys alphabetically, which can bury a real value-drift regression
#    in a wall of cosmetic reorder diff. Summarize which changed golden files
#    carry a real value change vs. a pure reorder, so reviewer attention goes
#    to the former. Informational only — never fails the build on its own.
bash scripts/golden-diff-summary.sh

exit $fail
