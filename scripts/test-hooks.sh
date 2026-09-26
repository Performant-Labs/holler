#!/usr/bin/env bash
# Tests for the tracked git hooks in .githooks/ (run in CI). Each case runs real
# `git commit`s in a throwaway repo that uses copies of the hooks.
set -uo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
git init -q .
git config user.name "hook test"
git config user.email "hook-test@example.com"
cp -R "$root/.githooks" .githooks
git config core.hooksPath .githooks

fail=0
check() { # check <description> <expected: ok|no> <command...>
  local desc=$1 want=$2; shift 2
  if "$@" >/dev/null 2>&1; then got=ok; else got=no; fi
  if [ "$got" = "$want" ]; then echo "ok   - $desc"; else echo "FAIL - $desc (wanted $want, got $got)"; fail=1; fi
}
n=0
commit() { # commit <message> [VAR=value ...]; a fresh staged change each time, CLAUDECODE unset unless given
  local msg=$1; shift
  n=$((n + 1)); echo "change $n" > "f$n.txt"; git add "f$n.txt"
  env -u CLAUDECODE "$@" git commit -q -m "$msg"
}
# The secret scan itself is exercised below; skip it for the message-hook cases.
skip=HOLLER_SKIP_GITLEAKS=1

check "a non-conventional subject is rejected" no commit "did some stuff" "$skip"
check "a conventional subject is accepted" ok commit "fix(hub): retry the lock" "$skip"
check "a scoped chore with an issue number is accepted" ok commit "chore(#12): phase" "$skip"
check "a Revert subject is accepted" ok commit 'Revert "fix(hub): retry the lock"' "$skip"

commit "docs: no trailer outside a session" "$skip" >/dev/null 2>&1
[ "$(git log -1 --format=%B | grep -ci '^Co-Authored-By:')" = 0 ] \
  && echo "ok   - no trailer is added outside an agent session" || { echo "FAIL - trailer added outside a session"; fail=1; }

commit "docs: trailer inside a session" "$skip" CLAUDECODE=1 >/dev/null 2>&1
[ "$(git log -1 --format=%B | grep -ci '^Co-Authored-By:')" = 1 ] \
  && echo "ok   - the trailer is added inside an agent session" || { echo "FAIL - trailer missing inside a session"; fail=1; }

commit "$(printf 'docs: explicit trailer\n\nCo-Authored-By: Someone <s@example.com>')" "$skip" CLAUDECODE=1 >/dev/null 2>&1
[ "$(git log -1 --format=%B | grep -ci '^Co-Authored-By:')" = 1 ] \
  && echo "ok   - an existing trailer is not duplicated" || { echo "FAIL - trailer duplicated"; fail=1; }

# pre-commit: fails closed without the scanner, unless explicitly skipped.
check "a missing scanner blocks the commit" no commit "fix: x" HOLLER_GITLEAKS_BIN=gitleaks-not-installed
check "an explicit skip lets it through" ok commit "fix: x" HOLLER_GITLEAKS_BIN=gitleaks-not-installed "$skip"

if command -v gitleaks >/dev/null 2>&1; then
  check "a clean change passes the scan" ok commit "fix: clean"
  tok="ghp_$(LC_ALL=C tr -dc 'A-Za-z0-9' < /dev/urandom | head -c 36)"
  n=$((n + 1)); echo "github_token = \"$tok\"" > "leak$n.txt"; git add "leak$n.txt"
  check "a realistic token is blocked by the scan" no env -u CLAUDECODE git commit -q -m "fix: leak"
  git reset -q HEAD "leak$n.txt"
else
  echo "skip - gitleaks not installed here; the scan cases were not run"
fi
exit $fail
