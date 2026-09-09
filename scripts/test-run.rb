#!/usr/bin/env ruby
# frozen_string_literal: true

# test-run.rb -- the loop described in the master testing issue #168 "Test
# runs": select test cases from the catalog, run the automated ones for real,
# write results back into the SAME test-run issue.
#
# Ported to the single holler binary from the retired holler-server
# scripts/test-run.rb (that repo's twin, read-only reference). The single
# binary changes three things from the old two-repo script:
#   * `--server-dir`/`--client-dir` collapse to one `--dir DIR` (default '.')
#     -- there is one checkout, not two.
#   * `parse_segment` is crate-aware per #168's `Automation` grammar (the
#     `<crate>` is derived from the path form; `tests/<file>.rs` with no
#     prefix means the `holler-cli` crate). The grammar lives in
#     automation.rb so run/ and exec share one implementation.
#   * a `test-tag-interop` case runs with `-- --ignored` but NO sibling
#     binary build: the single binary finds `holler` via cargo_bin, so the
#     old HOLLER_SERVER_BIN + `cargo build --release` sibling step is gone.
#
# Ruby + gems: requires the `octokit` gem (`gem install octokit --user-install`)
# and a Ruby new enough for it (macOS ships 2.6; if octokit won't install on
# your system Ruby, use a Homebrew Ruby: /opt/homebrew/opt/ruby/bin/ruby ...).
# Auth: uses `gh`'s stored token via `gh auth token` when GITHUB_TOKEN is not
# already set, so no separate credential setup is needed on a machine that
# already has `gh` logged in.
#
# Catalog source: holler issues labeled `test-case` whose body contains a
# "| Test ID |" header row (the "Test case slot" reserved placeholders are
# skipped). Each such issue is ONE case, ONE Test ID (single binary). Its
# header table supplies: Test ID (hlr-NNNN), Applies to (hub/body/both),
# Group (one of the 11 #168 groups), Automation (the #168 grammar), plus the
# test-cat-*/test-tag-* labels. `exec`'s --group matches on the test-grp-*
# label (the Group field is a fallback).
#
# Usage:
#   ruby scripts/test-run.rb discover
#   ruby scripts/test-run.rb start [--applies hub|body|both|all] [--group G] \
#                                   [--cat C] [--tag S...] [--tag-invert S...] \
#                                   [--type auto|manual|all]
#   ruby scripts/test-run.rb run ISSUE [--dir DIR]
#   ruby scripts/test-run.rb record ISSUE TEST_ID pass|fail [note]
#   ruby scripts/test-run.rb exec [TEST_ID] [--applies X] [--group G] [--cat C] \
#                                  [--tag S...] [--tag-invert S...] [--list F] \
#                                  [--list-invert F] [--list] [--last-failed N] [--dir DIR]
#     Runs the selected test case(s)' Automation fields locally, right now,
#     streaming real `cargo test` output live -- no GitHub write of any kind
#     (no test-run issue, no comment). A positional TEST_ID still works
#     (just the ID, e.g. `exec hlr-1000`) and composes (ANDs) with the
#     selection flags, each of which is an independent conjunct over the
#     catalog (Playwright-style; --tag is OR-within). --list (bare, no file)
#     PREVIEWs the resolved Test IDs + their Automation commands one per line
#     and exits 0 without running anything or writing to GitHub.
#     --last-failed N reads the ❌ rows of test-run issue N and runs exactly
#     those. Exits with the same status the underlying `cargo test` exits
#     with (exit 0 only if every selected case passed). --dir defaults to '.'.

require 'time'
require 'open3'
require 'optparse'
require 'set'
require_relative 'test_selection'
require_relative 'automation'

REPO = 'Performant-Labs/holler'
MARKER_START = '<!-- test-run-fields:start -->'
MARKER_END = '<!-- test-run-fields:end -->'

# Lazily constructs the GitHub client on first use (and only for the
# subcommands that talk to GitHub: discover, start, run, record, and exec --
# every exec path, including a bare `exec --list` whole-catalog preview,
# resolves against the catalog and so builds the client).
#
# `require 'octokit'` lives HERE, not at the top of the file, on purpose: a
# top-of-file require fires at load time, before `main` has parsed the
# subcommand, so a token-free `discover`-less invocation would die with a
# LoadError before it even knew which subcommand was requested. Deferring it
# into the lazy client means only the GitHub-touching paths ever pay for the
# gem. (The CI smoke DOES use the client, so CI installs octokit into
# GEM_HOME -- see .github/workflows/ci.yml.)
def client
  require 'octokit'
  token = ENV['GITHUB_TOKEN']
  if token.nil? || token.empty?
    token, status = Open3.capture2e('gh', 'auth', 'token')
    token = token.strip
    abort('error: no GITHUB_TOKEN and `gh auth token` failed -- run `gh auth login` first') if token.empty?
  end
  Octokit::Client.new(access_token: token, auto_paginate: true)
end

# ---------------------------------------------------------------------------
# discover: pull the filled test-case catalog as an Array of Hashes.
# One issue == one case == one Test ID (single binary; #168 §2/§3).
# ---------------------------------------------------------------------------
def discover(gh)
  # A nil client means the caller is a pure preview (a catalog filter with no
  # --last-failed), which never needs GitHub: it only reads the catalog to
  # display it. Such a run has no GITHUB_TOKEN requirement and does not load
  # octokit at all (the client is built lazily, only for the GitHub-touching
  # paths), so a preview cannot fail on a token- or gem-poor runner. An empty
  # catalog is the correct, tolerated answer in that case (#170).
  return [] if gh.nil?

  issues =
    begin
      gh.list_issues(REPO, labels: 'test-case', state: 'open', per_page: 100)
    rescue StandardError => e
      # A bad token or any other GitHub API hiccup must not crash a run that
      # does have a client: degrade to an empty catalog (a warning) instead of
      # aborting, so `--last-failed` (which still builds the client) keeps the
      # same "tolerate an unreadable catalog" contract as a preview.
      warn "warning: discover: could not read the catalog from #{REPO} (#{e.class}: #{e.message}) -- treating as empty"
      []
    end
  issues.flat_map do |issue|
    body = issue.body || ''
    next [] unless body.include?('| Test ID |')
    next [] if issue.title.start_with?('Test case slot')

    field_all(body, 'Test ID').map do |id|
      {
        issue: issue.number,
        title: issue.title,
        labels: issue.labels.map(&:name),
        id: id,
        applies: field(body, 'Applies to'),
        group: field(body, 'Group'),
        automation: field(body, 'Automation')
      }
    end
  end
end

def field(body, name)
  field_all(body, name).first
end

def field_all(body, name)
  body.lines.select { |l| l.strip.start_with?("| #{name} |") }.map do |line|
    # "| Field | Value |" -> "Value" (keep everything between the second and
    # the last pipe so a Value containing "|" inside a code span isn't cut).
    cells = line.strip.split('|').map(&:strip).reject(&:empty?)
    cells[1..].join(' | ')
  end
end

# ---------------------------------------------------------------------------
# start: create a new test-run issue from a selected slice of the catalog.
# ---------------------------------------------------------------------------
def start(gh, applies_filter: 'all', group_filter: nil, cat_filter: nil, tags: nil, tag_inverts: nil, type_filter: 'all')
  catalog = discover(gh)

  # --group/--cat/--tag/--tag-invert route through the shared selection core
  # (TestSelection); --type (auto/manual) is a start-only axis the selection
  # core intentionally doesn't model (an exec never needs a type split).
  sel = TestSelection.new(group: group_filter, cat: cat_filter, applies: applies_filter, tags: tags, tag_inverts: tag_inverts)
  selected = sel.any? ? sel.call(catalog) : catalog
  selected = selected.select do |c|
    case type_filter
    when 'all' then true
    when 'auto' then c[:labels].include?('test-auto')
    when 'manual' then c[:labels].include?('test-manual')
    else false
    end
  end

  abort("error: start: no catalog entries matched the selection") if selected.empty?

  now = Time.now.utc.strftime('%Y-%m-%d %H:%M UTC')
  rows = selected.map { |c| Row.new(id: c[:id], type: row_type(c[:labels]), status: '⏳ pending', evidence: '') }
  # #168 §6: the fields block carries a SINGLE `Commit` (the one commit the
  # run is against). The old script's separate Server/Client commit fields are
  # gone -- one binary, one commit.
  commit = `git rev-parse --short HEAD 2>/dev/null`.strip
  body = render_body(
    fields: { 'Commit' => (commit.empty? ? 'unknown' : commit) },
    rows: rows,
    catalog: catalog
  )

  issue = gh.create_issue(REPO, "Test run: #{now}", body, labels: 'test-run')
  puts issue.html_url
end

def row_type(labels)
  auto = labels.include?('test-auto')
  manual = labels.include?('test-manual')
  return 'auto+manual' if auto && manual
  return 'auto' if auto

  'manual'
end

Row = Struct.new(:id, :type, :status, :evidence, keyword_init: true)

def render_body(fields:, rows:, catalog:)
  passed = rows.count { |r| r.status.include?('✅') }
  pending = rows.count { |r| r.status.include?('pending') }
  total = rows.size
  overall = "#{passed}/#{total} passed (#{pending} pending)"

  lines = []
  lines << MARKER_START
  lines << '| Field | Value |'
  lines << '|---|---|'
  fields.each { |k, v| lines << "| #{k} | #{v} |" }
  lines << "| Overall | #{overall} |"
  lines << ''
  lines << '| Test Case | Type | Status | Evidence |'
  lines << '|---|---|---|---|'
  rows.each do |r|
    cat = catalog.find { |c| c[:id] == r.id }
    link = cat ? "https://github.com/#{REPO}/issues/#{cat[:issue]}" : ''
    lines << "| [#{r.id}](#{link}) | #{r.type} | #{r.status} | #{r.evidence} |"
  end
  lines << MARKER_END
  lines.join("\n")
end

# ---------------------------------------------------------------------------
# Shared: parse the current results table out of a test-run issue body.
# ---------------------------------------------------------------------------
def extract_rows(body)
  block = body[/#{Regexp.escape(MARKER_START)}(.*)#{Regexp.escape(MARKER_END)}/m, 1] || ''
  rows = block.lines.map do |line|
    next unless line.strip.start_with?('| [')

    m = line.match(/^\|\s*\[([^\]]+)\][^|]*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*(.*?)\s*\|\s*$/)
    next unless m

    Row.new(id: m[1], type: m[2], status: m[3], evidence: m[4])
  end
  rows.compact
end

def fields_before_table(body)
  block = body[/#{Regexp.escape(MARKER_START)}(.*?)\n\n/m, 1] || ''
  fields = {}
  block.lines.each do |line|
    next unless line.strip.start_with?('|') && !line.include?('---') && !line.include?('| Field |')

    cells = line.strip.split('|').map(&:strip).reject(&:empty?)
    fields[cells[0]] = cells[1] if cells.size >= 2
  end
  fields.reject { |k, _| k == 'Overall' }
end

def splice_body(body, rows, catalog)
  fields = fields_before_table(body)
  new_block = render_body(fields: fields, rows: rows, catalog: catalog)
  body.sub(/#{Regexp.escape(MARKER_START)}.*#{Regexp.escape(MARKER_END)}/m, new_block)
end

# ---------------------------------------------------------------------------
# run: execute pending automated cases in a test-run issue for real.
# ---------------------------------------------------------------------------

# Cases carrying this label AND a single, unqualified `src/` Automation
# segment (issue #248) get batched: every such pending case for one crate
# runs together as one `cargo test -p <crate> --lib` invocation instead of
# one subprocess per case. --lib already runs a crate's entire unit-test
# binary in one process regardless of case count, so N separate subprocesses
# were pure per-process overhead. Batching changes only *how* a result is
# produced -- each case still gets its own status/evidence.
UNIT_BATCH_LABEL = 'test-cat-unit'

# Cases carrying this label are real, deliberately #[ignore]d cross-process
# interop tests (hub+body). In the single binary they find `holler` via
# cargo_bin, so the ONLY change vs a plain tests/ segment is the trailing
# `-- --ignored`: no sibling binary build, no env var (the old two-repo
# HOLLER_SERVER_BIN step is gone).
INTEROP_LABEL = 'test-tag-interop'

# Runs `cargo test -p <crate> --lib` once in `dir` (no <fn> filter -- the
# whole unit-test binary) and parses cargo's own `test <path> ... ok|FAILED`
# lines back into a { bare_fn_name => passed? } map, keyed on the last
# `::`-segment of each test's full path -- the same bare-name matching a
# single-case `--lib <fn>` run relies on.
def run_unit_batch(dir, crate)
  full_cmd = "source \"$HOME/.cargo/env\" 2>/dev/null; cargo test -p #{crate} --lib"
  out, = Open3.capture2e('bash', '-lc', full_cmd, chdir: dir)
  results = {}
  out.each_line do |line|
    m = line.match(/^test (\S+) \.\.\. (ok|FAILED)/)
    next unless m

    results[m[1].split('::').last] = (m[2] == 'ok')
  end
  { out: out, results: results }
end

# Partitions `rows` into { row.id => [crate, fn] } for every pending, auto,
# single-segment, `test-cat-unit`-labeled case whose Automation resolves to a
# plain lib-test Segment -- everything else (integration, manual, multi-
# segment, anything parse_segment can't cleanly resolve) runs one at a time.
def partition_unit_batchable(rows, catalog, dir:, interop:)
  batchable = {}
  rows.each do |row|
    next unless row.status.include?('pending') && row.type.include?('auto')

    cat = catalog.find { |c| c[:id] == row.id }
    next unless cat && cat[:labels].include?(UNIT_BATCH_LABEL)

    automation = cat[:automation]
    next if automation.nil? || automation.empty? || Automation.manual?(automation) || automation.include?(';')

    seg = Automation.parse_segment(automation, dir: dir, interop: interop)
    next if seg.error || !seg.lib_test

    batchable[row.id] = [seg.crate, seg.fn]
  end
  batchable
end

def run_cases(gh, issue_number, dir:)
  issue = gh.issue(REPO, issue_number)
  catalog = discover(gh)
  rows = extract_rows(issue.body)

  # Resolve every pending automated case against the real cargo commands and
  # collect the result comment to post for each case that failed (or used a
  # fallback). This is the only place `run` talks to GitHub: the resolution
  # logic itself is factored out into resolve_run_rows so it can be exercised
  # against a synthetic fixture without a GitHub client.
  comments = resolve_run_rows(rows, catalog, dir: dir)
  comments.each { |comment_body| gh.add_comment(REPO, issue_number, comment_body) }

  new_body = splice_body(issue.body, rows, catalog)
  gh.update_issue(REPO, issue_number, body: new_body)
  passed = rows.count { |r| r.status.include?('✅') }
  puts "Updated https://github.com/#{REPO}/issues/#{issue_number} -- #{passed}/#{rows.size} passed"
end

# Pure (no GitHub, no clock): resolves each pending automated row in `rows`
# against `catalog`, running the real cargo commands via `dir`. Batches the
# test-cat-unit cases per crate (one `cargo test -p <crate> --lib` each) and
# runs every other automated case's segments directly. Returns, in row order,
# the markdown body of the result comment that should be posted for each row
# that FAILED or used a fallback (empty when nothing needs a comment). The
# caller posts those (run_cases on GitHub; the mechanism test to a stub).
def resolve_run_rows(rows, catalog, dir:)
  interop = ->(row) { (c = catalog.find { |x| x[:id] == row.id }) && c[:labels].include?(INTEROP_LABEL) }
  batchable = partition_unit_batchable(rows, catalog, dir: dir, interop: interop)
  batch_runs = {}
  batchable.values.map(&:first).uniq.each do |crate|
    puts "==> batched unit run: #{crate} (cargo test -p #{crate} --lib)"
    batch_runs[crate] = run_unit_batch(dir, crate)
  end

  comments = []
  rows.each do |row|
    next unless row.status.include?('pending') # already resolved by a prior run/record

    unless row.type.include?('auto')
      next # manual-only, stays pending for `record`
    end

    cat = catalog.find { |c| c[:id] == row.id }
    automation = cat && cat[:automation]

    if automation.nil? || automation.empty? || Automation.manual?(automation)
      row.status = '⏳ pending — manual, use `record`'
      next
    end

    if (crate_fn = batchable[row.id])
      crate, fn = crate_fn
      batch = batch_runs[crate]
      found = batch[:results].key?(fn)
      passed = found && batch[:results][fn]
      ts = Time.now.utc.strftime('%Y-%m-%dT%H:%MZ')

      if passed
        row.status = '✅ pass'
        row.evidence = "batched unit run #{ts}"
      else
        row.status = '❌ fail'
        note = found ? '' : " -- test '#{fn}' not found in --lib output (renamed or removed?)"
        row.evidence = "batched unit run #{ts} — see comment"
        comments << "### Result for `#{row.id}`: #{row.status}\n\n" \
                    "Part of a batched `cargo test -p #{crate} --lib` run#{note}.\n\n" \
                    "```\n#{batch[:out].lines.last(25).join}\n```"
      end
      puts "==> #{row.id}: #{row.status} (batched, #{crate})"
      next
    end

    puts "==> #{row.id}: #{automation}"
    all_ok = true
    fallback_used = false
    log = +''

    run_interop = cat[:labels].include?(INTEROP_LABEL)

    Automation.split_segments(automation).each do |raw_seg|
      seg = Automation.parse_segment(raw_seg, dir: dir, interop: run_interop)
      if seg.error
        all_ok = false
        log << seg.error
        next
      end
      fallback_used ||= seg.fallback_used

      seg_ok, out, exitstatus = exec_segment_captured(seg)
      if !seg_ok && seg.fn
        m = out.match(/^test result: \w+\. (\d+) passed; (\d+) failed;.*?(\d+) filtered out/)
        if m && m[1].to_i.zero? && m[2].to_i.zero?
          log << "[named test '#{seg.fn}' did not run -- filtered out or does not exist#{seg.file ? " in #{seg.file}.rs" : ''}]\n"
        end
      end

      all_ok &&= seg_ok
      target = if seg.lib_test then '--lib'
               elsif seg.file then seg.file
               else 'whole workspace'
               end
      log << "\n--- #{seg.crate || 'workspace'} (#{target}#{seg.fn ? " / #{seg.fn}" : ''}), exit #{exitstatus} ---\n"
      log << out.lines.last(25).join
    end

    ts = Time.now.utc.strftime('%Y-%m-%dT%H:%MZ')
    if all_ok
      row.status = '✅ pass'
      row.evidence = "local run #{ts}"
    else
      row.status = '❌ fail'
      row.evidence = "local run #{ts} — see comment"
    end
    row.evidence += ' (fallback: whole-workspace run, automation field not precisely parseable)' if fallback_used

    comments << "### Result for `#{row.id}`: #{row.status}\n\n```\n#{log}\n```" if row.status == '❌ fail' || fallback_used
  end

  comments
end

# Runs a fully-resolved Segment's command, capturing output (for `run`).
# Returns [ok, output, exitstatus]. A filter matching zero tests still exits
# 0 -- cargo can't say "your filter named nothing real" -- so a named-fn
# segment with 0 passed/0 failed is treated as a failure, not a silent pass.
def exec_segment_captured(seg)
  full_cmd = "source \"$HOME/.cargo/env\" 2>/dev/null; #{seg.cmd}"
  out, status = Open3.capture2e(seg.env || {}, 'bash', '-lc', full_cmd, chdir: seg.dir)
  ok = status.success?
  if ok && seg.fn && (m = out.match(/^test result: \w+\. (\d+) passed; (\d+) failed;.*?(\d+) filtered out/))
    ok = false if m[1].to_i.zero? && m[2].to_i.zero?
  end
  [ok, out, status.exitstatus]
end

# ---------------------------------------------------------------------------
# record: manually record one result (typically a manual-labeled case).
# ---------------------------------------------------------------------------
def record(gh, issue_number, target_id, result, note)
  abort("error: record: result must be 'pass' or 'fail'") unless %w[pass fail].include?(result)

  issue = gh.issue(REPO, issue_number)
  catalog = discover(gh)
  rows = extract_rows(issue.body)

  target = rows.find { |r| r.id == target_id }
  abort("error: record: test id '#{target_id}' not found in the results table of issue ##{issue_number}") unless target

  ts = Time.now.utc.strftime('%Y-%m-%dT%H:%MZ')
  target.status = result == 'pass' ? '✅ pass' : '❌ fail'
  target.evidence = "manual, recorded #{ts}#{note && !note.empty? ? " — #{note}" : ''}"

  new_body = splice_body(issue.body, rows, catalog)
  gh.update_issue(REPO, issue_number, body: new_body)
  puts "Updated https://github.com/#{REPO}/issues/#{issue_number} -- #{target_id} -> #{target.status}"
end

# ---------------------------------------------------------------------------
# exec: run selected test case(s)' Automation fields locally, live, for a
# human at a terminal -- no GitHub write of any kind. Streams real cargo
# output as it happens and exits with the status the commands exit with.
# ---------------------------------------------------------------------------
def exec_test(gh, entries, dir:)
  overall_ok = true

  entries.each do |cat|
    test_id = cat[:id]
    automation = cat[:automation]
    if automation.nil? || automation.empty? || Automation.manual?(automation)
      warn "error: exec: #{test_id} is a manual case with no automated command to run (Automation: #{automation.inspect}) -- skipping"
      overall_ok = false
      next
    end

    run_interop = cat[:labels].include?(INTEROP_LABEL)

    puts "==> #{test_id}: #{automation}"
    Automation.split_segments(automation).each do |raw_seg|
      seg = Automation.parse_segment(raw_seg, dir: dir, interop: run_interop)
      if seg.error
        warn seg.error
        overall_ok = false
        next
      end

      puts "--- #{seg.crate || 'workspace'} $ #{seg.cmd} (in #{seg.dir}) ---"
      full_cmd = "source \"$HOME/.cargo/env\" 2>/dev/null; #{seg.cmd}"
      ok = system(seg.env || {}, 'bash', '-lc', full_cmd, chdir: seg.dir)
      overall_ok &&= ok
    end
  end

  exit(overall_ok ? 0 : 1)
end

# ---------------------------------------------------------------------------
def main
  cmd = ARGV.shift
  gh = nil

  case cmd
  when 'discover'
    require 'json'
    gh = client
    puts JSON.pretty_generate(discover(gh))
  when 'start'
    opts = { applies: 'all', type: 'all' }
    OptionParser.new do |o|
      o.on('--applies X') { |v| opts[:applies] = v }
      o.on('--group G') { |v| opts[:group] = v }
      o.on('--cat C') { |v| opts[:cat] = v }
      o.on('--tag S', 'test-tag-<S> (OR-within, multiple allowed)') { |v| (opts[:tags] ||= []) << v }
      o.on('--tag-invert S', 'exclude cases carrying test-tag-<S> (multiple allowed)') { |v| (opts[:tag_inverts] ||= []) << v }
      o.on('--type X') { |v| opts[:type] = v }
    end.parse!(ARGV)
    gh = client
    start(gh, applies_filter: opts[:applies], group_filter: opts[:group], cat_filter: opts[:cat], tags: opts[:tags], tag_inverts: opts[:tag_inverts], type_filter: opts[:type])
  when 'run'
    issue = ARGV.shift or abort('usage: run ISSUE [--dir DIR]')
    opts = { dir: '.' }
    OptionParser.new do |o|
      o.on('--dir DIR') { |v| opts[:dir] = v }
    end.parse!(ARGV)
    gh = client
    run_cases(gh, issue.to_i, dir: opts[:dir])
  when 'record'
    issue = ARGV.shift or abort('usage: record ISSUE TEST_ID pass|fail [note]')
    test_id = ARGV.shift or abort('usage: record ISSUE TEST_ID pass|fail [note]')
    result = ARGV.shift or abort('usage: record ISSUE TEST_ID pass|fail [note]')
    note = ARGV.shift || ''
    gh = client
    record(gh, issue.to_i, test_id, result, note)
  when 'exec'
    # Positional TEST_ID is OPTIONAL (selection flags may stand in for it).
    # Only shift it when the first remaining arg is NOT an option -- a bare
    # ARGV.shift would otherwise grab `--group` (the first flag) as the ID on
    # a flag-only invocation. A bare `exec` still reaches the has_selection
    # guard below and prints usage, exiting non-zero.
    test_id = (ARGV.first && !ARGV.first.start_with?('-')) ? ARGV.shift : nil
    opts = { dir: '.' }
    exec_banner = "usage: exec [TEST_ID] [--applies X] [--group G] [--cat C] [--tag S...] " \
                  "[--tag-invert S...] [--list F] [--list-invert F] [--list] " \
                  "[--last-failed N] [--dir DIR]"
    OptionParser.new do |o|
      o.banner = exec_banner
      o.on('--dir DIR') { |v| opts[:dir] = v }
      o.on('--group G') { |v| opts[:group] = v }
      o.on('--cat C') { |v| opts[:cat] = v }
      o.on('--applies X') { |v| opts[:applies] = v }
      o.on('--tag S', 'test-tag-<S> to select (OR-within, multiple allowed)') { |v| (opts[:tags] ||= []) << v }
      o.on('--tag-invert S', 'exclude cases carrying test-tag-<S> (multiple allowed)') { |v| (opts[:tag_inverts] ||= []) << v }
      o.on('--list [FILE]', 'bare: preview resolved IDs without running; FILE: keep only those IDs') { |v| opts[:list] = v; opts[:preview] = (v.nil?) }
      o.on('--list-invert FILE', 'exclude the Test IDs listed in FILE') { |v| opts[:list_invert] = v }
      o.on('--last-failed N', 'run only the cases that failed in test-run issue N (reads its ❌ rows)') { |v| opts[:last_failed] = v }
      o.on('-h', '--help') { puts o.banner; exit 0 }
    end.parse!(ARGV)

    # A BARE `--list` (no file) is a real selection: it previews the WHOLE
    # catalog (one line per case). #170's CI smoke relies on exactly this --
    # "`exec --list` ... prints >=1 line (0 real cases yet ... until cases
    # exist discover must exit 0 with []; tolerate that)". So a lone --list is
    # a legitimate (empty-selection) request that must resolve through the
    # catalog and exit 0, NOT a usage error. It counts as a selection below.
    # NOTE: a bare `--list` (no FILE) is registered with opts[:list] = nil
    # (the preview flag), so "given" must be tested as KEY PRESENCE
    # (opts.key?), not `!opts[:list].nil?` (which is false when list is nil --
    # the bare-preview case). opts[:list_invert] and opts[:last_failed] are
    # always strings, so they are checked with the normal nil test.
    has_selection = !test_id.nil? || !opts[:group].nil? || !opts[:cat].nil? || !opts[:applies].nil? ||
                    !opts[:tags].nil? || !opts[:tag_inverts].nil? ||
                    opts.key?(:list) || !opts[:list_invert].nil? || !opts[:last_failed].nil?
    # Only a TRULY bare `exec` (no positional id, no flags at all) prints
    # usage. Everything else -- including a lone `exec --list` -- proceeds to
    # resolve against the catalog.
    abort(exec_banner) unless has_selection

    # Which flags were given (names the "no match" error). --last-failed is
    # not in this list: it narrows via list_ids, and an empty set is a warning
    # above, not a "no match" abort.
    active =
      (opts[:group] ? ['group: ' + opts[:group]] : []) +
      (opts[:cat] ? ['cat: ' + opts[:cat]] : []) +
      (opts[:applies] ? ['applies: ' + opts[:applies]] : []) +
      (opts[:tags] ? ['tag: ' + opts[:tags].join(',')] : []) +
      (opts[:tag_inverts] ? ['tag-invert: ' + opts[:tag_inverts].join(',')] : [])
    active = ['list: ' + opts[:list]] unless opts[:list].nil? || opts[:list] == ''
    active += ['last-failed: #' + opts[:last_failed]] unless opts[:last_failed].nil?

    # Which paths need the GitHub client. A pure PREVIEW (any catalog filter
    # WITHOUT --last-failed) only READS the catalog to display it -- it never
    # launches a process or fetches a single issue -- so it does not need the
    # client at all. `--last-failed` is the one selection axis that makes a
    # real per-issue GitHub read (it fetches issue N's body), so it needs the
    # client; so does the run path (it launches processes).
    #
    # Making preview token-free is deliberate: a CI smoke of `exec --list`
    # then needs no GITHUB_TOKEN and no octokit (the gem is only loaded by the
    # lazy client, which preview never builds), so it cannot break on a runner
    # that has no token or whose gem environment differs per event type.
    preview = !test_id.nil? || !opts[:group].nil? || !opts[:cat].nil? ||
              !opts[:applies].nil? || !opts[:tags].nil? || !opts[:tag_inverts].nil? ||
              opts.key?(:list) || !opts[:list_invert].nil?
    # (preview || last_failed.nil?) must be parenthesized: the ternary binds
    # looser than ||, so without the parens this would be (preview || nil?)
    # ? nil : client -- i.e. nil for EVERY non-preview command (a plain run
    # command is "no preview, no --last-failed") and the run path would lose
    # its client. We want the client only for the run path and --last-failed.
    gh = (preview || opts[:last_failed].nil?) ? nil : client

    # --last-failed N resolves to a list of Test IDs (the ❌ rows of issue N)
    # that feeds the selection's list_ids axis. It combines with --list FILE.
    # It is a REAL GitHub read (it fetches issue N's body), so it runs only
    # now that `gh` (the client) exists.
    last_failed_ids = nil
    unless opts[:last_failed].nil?
      run_issue = gh.issue(REPO, opts[:last_failed].to_i)
      last_failed_ids = TestSelection.last_failed(run_issue.body)
      warn "warning: exec: no failing rows in test-run issue #{opts[:last_failed]} -- nothing to re-run" if last_failed_ids.empty?
    end

    catalog = discover(gh)
    # A positional TEST_ID narrows the catalog to that one entry (the legacy
    # single-case path). It then ANDs with any selection flags below.
    selected = test_id ? catalog.select { |c| c[:id] == test_id } : catalog

    if active.empty?
      abort("error: exec: test id '#{test_id}' not found in the catalog") if test_id && selected.empty?
    else
      list_ids = []
      list_ids += TestSelection.read_list_file(opts[:list]) if opts[:list] && !opts[:list].empty? && !opts[:preview]
      list_ids += last_failed_ids if last_failed_ids
      sel = TestSelection.new(
        group: opts[:group],
        cat: opts[:cat],
        applies: opts[:applies],
        tags: opts[:tags],
        tag_inverts: opts[:tag_inverts],
        list_ids: (list_ids.any? ? list_ids : nil),
        list_invert_ids: opts[:list_invert] ? TestSelection.read_list_file(opts[:list_invert]) : nil
      )
      selected = sel.call(selected)
      if opts[:preview]
        selected.each { |c| puts "#{c[:id]}  #{c[:automation] || '(none)'}" }
        # An empty preview is a valid, exit-0 outcome -- e.g. a bare `exec
        # --list` against an empty catalog (#170: "discover must exit 0 with
        # []"). So report it and succeed rather than erroring.
        if selected.empty?
          puts "0 case(s) matched (the catalog has no matching cases yet)"
          exit(0)
        end
        puts "#{selected.size} case(s) matched"
        exit(0)
      end
      abort("error: exec: no catalog cases matched the selection (#{active.join(', ')})") if selected.empty?
    end

    exec_test(gh, selected, dir: opts[:dir])
  else
    abort("usage: #{$PROGRAM_NAME} {discover|start|run|record|exec} ...")
  end
end

main if __FILE__ == $PROGRAM_NAME
