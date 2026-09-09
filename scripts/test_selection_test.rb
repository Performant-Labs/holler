#!/usr/bin/env ruby
# frozen_string_literal: true

# test_selection_test.rb -- unit tests for scripts/test_selection.rb
# (issue #170's selection core; ported from holler-server#304).
#
# Deliberately stdlib-only (minitest, no octokit) so it runs on a runner's
# preinstalled Ruby (2.6 on macOS, ruby >= 3.2 on ubuntu-latest) with no gem
# install -- see the CI step that runs it. `parse_segment` lives in
# automation.rb (kept out of test_selection.rb so that file stays a pure
# selection core) and is covered here by a pure-function test.

require 'minitest/autorun'
require 'tempfile'
require 'stringio'

require_relative 'test_selection'
require_relative 'automation'

class TestSelectionTest < Minitest::Test
  # A synthetic discover()-shaped catalog. Single binary: hlr-NNNN IDs,
  # applies: hub/body/both (NOT the old server/client), and the 11th `protocol`
  # group (issue #168 Q1) is present. Mixed groups + tags, out-of-group IDs to
  # prove filtering must not reorder.
  CATALOG = [
    # Note: each entry's `labels` array mirrors what discover() returns from a
    # real test-case issue's label set -- it carries BOTH the test-grp-<g>
    # group label AND the test-cat-<c> category label (plus any test-tag-*/
    # test-hub/test-body). The selection core's group and cat axes both filter
    # on those labels, so the synthetic catalog must include the cat labels too
    # or the cat axis has nothing to select.
    { id: 'hlr-1120', applies: 'hub',    group: 'concurrency', cat: 'regression',  labels: %w[test-grp-concurrency test-cat-regression test-tag-alters-db],  title: 'srv A' },
    { id: 'hlr-1130', applies: 'body',   group: 'concurrency', cat: 'smoke',       labels: %w[test-grp-concurrency test-cat-smoke test-tag-remote],     title: 'srv B' },
    { id: 'hlr-1140', applies: 'both',   group: 'concurrency', cat: 'acceptance',  labels: %w[test-grp-concurrency test-cat-acceptance test-tag-alters-db test-tag-slow test-hub test-body], title: 'both C' },
    { id: 'hlr-1000', applies: 'body',   group: 'invocation',  cat: 'unit',        labels: %w[test-grp-invoc test-cat-unit test-tag-alters-db],        title: 'invocation, alters-db' },
    { id: 'hlr-2010', applies: 'both',   group: 'protocol',    cat: 'unit',        labels: %w[test-grp-protocol test-cat-unit test-tag-interop],       title: 'protocol wire' },
    { id: 'hlr-1900', applies: 'hub',    group: 'load',        cat: 'regression',  labels: %w[test-grp-load test-cat-regression test-tag-slow],              title: 'load' }
  ].freeze

  def ids(result)
    result.map { |c| c[:id] }
  end

  # (a) No conjuncts -> all entries, in order.
  def test_no_conjuncts_selects_all
    sel = TestSelection.new
    assert_operator(sel, :kind_of?, TestSelection)
    assert_equal(CATALOG, sel.call(CATALOG))
  end

  # (b) --group by full name, label stem, and raw label -> same subset; an
  #     unknown group aborts listing the valid groups.
  def test_group_by_full_name_stem_and_label
    sel_full = TestSelection.new(group: 'concurrency')
    sel_label = TestSelection.new(group: 'test-grp-concurrency')
    # 'invocation' -> stem 'invoc' proves full-name input goes through the map.
    sel_invoc_stem = TestSelection.new(group: 'invoc')
    sel_invoc_full = TestSelection.new(group: 'invocation')
    sel_protocol = TestSelection.new(group: 'protocol')

    assert_equal(%w[hlr-1120 hlr-1130 hlr-1140], ids(sel_full.call(CATALOG)))
    assert_equal(%w[hlr-1120 hlr-1130 hlr-1140], ids(sel_label.call(CATALOG)))
    assert_equal(%w[hlr-1000], ids(sel_invoc_full.call(CATALOG)))
    assert_equal(%w[hlr-1000], ids(sel_invoc_stem.call(CATALOG)))
    # The 11th group (#168 Q1) resolves and selects its own case.
    assert_equal(%w[hlr-2010], ids(sel_protocol.call(CATALOG)))
  end

  def test_unknown_group_aborts_listing_valid_groups
    cap = StringIO.new
    original = $stderr
    $stderr = cap
    assert_raises(SystemExit) { TestSelection.new(group: 'bogus').call(CATALOG) }
  ensure
    $stderr = original
  end

  # The aborted stderr output must name the bad value and list every valid group.
  def test_unknown_group_error_names_value_and_lists_valid_groups
    cap = StringIO.new
    original = $stderr
    $stderr = cap
    assert_raises(SystemExit) { TestSelection.new(group: 'bogus').call(CATALOG) }
  ensure
    $stderr = original
    msg = cap.string
    assert_match(/unknown group 'bogus'/, msg)
    TestSelection.valid_groups.each { |g| assert_match(g, msg) }
  end

  # (c) --applies filters; nil is no filter; 'all' keeps everything.
  #     Single binary: 'both' is the Applies-to value (NOT the old server/client).
  def test_applies
    assert_equal(%w[hlr-1120 hlr-1900], ids(TestSelection.new(applies: 'hub').call(CATALOG)))
    assert_equal(%w[hlr-1130 hlr-1000], ids(TestSelection.new(applies: 'body').call(CATALOG)))
    assert_equal(%w[hlr-1140 hlr-2010], ids(TestSelection.new(applies: 'both').call(CATALOG)))
    assert_equal(CATALOG, TestSelection.new(applies: 'all').call(CATALOG))
    assert_equal(CATALOG, TestSelection.new.call(CATALOG))
  end

  # (d) --tag single, --tag a b OR-union, unknown tag -> empty,
  #      --tag-invert -> complement, unknown tag-invert -> everything.
  def test_tag_single_and_or_union
    assert_equal(%w[hlr-1120 hlr-1140 hlr-1000], ids(TestSelection.new(tags: %w[alters-db]).call(CATALOG)))
    assert_equal(%w[hlr-1120 hlr-1130 hlr-1140 hlr-1000], ids(TestSelection.new(tags: %w[alters-db remote]).call(CATALOG)))
  end

  def test_unknown_tag_selects_nothing
    assert_equal([], ids(TestSelection.new(tags: %w[never-applied]).call(CATALOG)))
  end

  def test_tag_invert_selects_complement
    # Complement of {alters-db} = the entries NOT carrying test-tag-alters-db:
    # 1130 (remote), 2010 (interop), 1900 (slow).
    assert_equal(%w[hlr-1130 hlr-2010 hlr-1900], ids(TestSelection.new(tag_inverts: %w[alters-db]).call(CATALOG)))
  end

  def test_unknown_tag_invert_selects_everything
    assert_equal(CATALOG, TestSelection.new(tag_inverts: %w[never-applied]).call(CATALOG))
  end

  # (e) --list file subset / --list-invert file (with a # comment + blank line).
  def test_list_and_list_invert_files
    t = Tempfile.new('list170')
    t.write("hlr-1120\n\n# a comment line\nhlr-1130\n")
    t.close

    assert_equal(%w[hlr-1120 hlr-1130], ids(TestSelection.new(list_ids: TestSelection.read_list_file(t.path)).call(CATALOG)))
    assert_equal(%w[hlr-1140 hlr-1000 hlr-2010 hlr-1900], ids(TestSelection.new(list_invert_ids: TestSelection.read_list_file(t.path)).call(CATALOG)))
  ensure
    t&.close!
  end

  # (cat) NEW: --cat filters on the (CLOSED) category axis; 'all' is no
  #      constraint. #168: test-cat-* is the closed set smoke|regression|
  #      acceptance|unit.
  def test_cat_filters
    assert_equal(%w[hlr-1130], ids(TestSelection.new(cat: 'smoke').call(CATALOG)))
    assert_equal(%w[hlr-1120 hlr-1900], ids(TestSelection.new(cat: 'regression').call(CATALOG)))
    assert_equal(%w[hlr-1140], ids(TestSelection.new(cat: 'acceptance').call(CATALOG)))
    assert_equal(%w[hlr-1000 hlr-2010], ids(TestSelection.new(cat: 'unit').call(CATALOG)))
    # Unknown category selects nothing (closed set: not OR-open like tags).
    assert_equal([], ids(TestSelection.new(cat: 'bogus').call(CATALOG)))
    # No cat constraint -> unchanged.
    assert_equal(CATALOG, TestSelection.new(cat: 'all').call(CATALOG))
  end

  # (cat∧group) NEW: the category axis ANDs with the group axis (conjunction).
  def test_cat_and_group_intersect
    # concurrency ∩ regression = only hlr-1120 (hlr-1130 is smoke, hlr-1140 acceptance).
    assert_equal(%w[hlr-1120], ids(TestSelection.new(group: 'concurrency', cat: 'regression').call(CATALOG)))
    # concurrency ∩ unit = none of the three concurrency cases are unit.
    assert_equal([], ids(TestSelection.new(group: 'concurrency', cat: 'unit').call(CATALOG)))
    # protocol ∩ unit = the one protocol case.
    assert_equal(%w[hlr-2010], ids(TestSelection.new(group: 'protocol', cat: 'unit').call(CATALOG)))
  end

  # (last-failed) NEW: last_failed reads a test-run issue body and returns the
  #   IDs of the rows whose Status cell contains a fail marker (❌) only -- the
  #   ✅ pass / ⏳ pending rows are dropped. This is what `exec --last-failed N`
  #   feeds to the selection list_ids axis.
  def test_last_failed_rows_parse_only_fail_rows
    body = <<~BODY
      some intro prose
      <!-- test-run-fields:start -->
      | Field | Value |
      |---|---|
      | Commit | abc1234 |
      | Overall | 1/3 passed (1 pending) |

      | Test Case | Type | Status | Evidence |
      |---|---|---|---|
      | [hlr-1000](https://github.com/Performant-Labs/holler/issues/90) | auto | ✅ pass | local run 2026-09-08 |
      | [hlr-1130](https://github.com/Performant-Labs/holler/issues/91) | auto | ❌ fail | local run 2026-09-08 — see comment |
      | [hlr-1140](https://github.com/Performant-Labs/holler/issues/92) | auto | ⏳ pending |  |
      | [hlr-2010](https://github.com/Performant-Labs/holler/issues/93) | auto | ❌ fail | batched unit run |
      <!-- test-run-fields:end -->
      trailing prose
    BODY
    assert_equal(%w[hlr-1130 hlr-2010], TestSelection.last_failed(body))
  end

  # A test-run body with no failing rows -> empty (no last-failed to re-run).
  def test_last_failed_none_returns_empty
    body = <<~BODY
      <!-- test-run-fields:start -->
      | Test Case | Type | Status | Evidence |
      |---|---|---|---|
      | [hlr-1000](x) | auto | ✅ pass | run |
      | [hlr-1010](x) | auto | ⏳ pending |  |
      <!-- test-run-fields:end -->
    BODY
    assert_equal([], TestSelection.last_failed(body))
  end

  # (f) Composition: group AND cat AND tag AND tag-invert AND list-invert ->
  #     exact intersection across the new axes.
  def test_composition_is_exact_intersection
    t = Tempfile.new('list170')
    t.write("hlr-1130\n")
    t.close

    sel = TestSelection.new(
      group: 'concurrency',
      cat: 'regression',
      tags: %w[alters-db remote],
      tag_inverts: %w[slow],
      list_invert_ids: TestSelection.read_list_file(t.path)
    )
    # concurrency ∩ regression = {1120}; (alters-db|remote) keeps 1120; not slow; not 1130.
    assert_equal(%w[hlr-1120], ids(sel.call(CATALOG)))
  ensure
    t&.close!
  end

  # (g) Ordering: call() must NOT reorder what discover() gives it.
  def test_result_order_preserved
    ordered = [CATALOG[0], CATALOG[1], CATALOG[2]]
    assert_equal(%w[hlr-1120 hlr-1130 hlr-1140], ids(TestSelection.new(group: 'concurrency').call(ordered)))
    shuffled = [CATALOG[2], CATALOG[0], CATALOG[1]]
    assert_equal(%w[hlr-1140 hlr-1120 hlr-1130], ids(TestSelection.new(group: 'concurrency').call(shuffled)))
  end

  # (i) No conjuncts => the whole catalog. Distinguishable via the any? gate.
  def test_no_selection_means_all_pending
    assert_equal(false, TestSelection.new.any?)
    assert_equal(CATALOG, TestSelection.new.call(CATALOG))
    assert_equal(true, TestSelection.new(group: 'concurrency').any?)
    assert_equal(true, TestSelection.new(cat: 'unit').any?)
  end

  # (h) read_list_file missing file -> SystemExit.
  def test_read_list_file_missing_aborts
    assert_raises(SystemExit) { TestSelection.read_list_file('/nonexistent/test-selection-170') }
  end

  # valid_groups is the 11 authoritative full names (#168: the 10 behavioral
  # groups PLUS the 11th `protocol` group).
  def test_valid_groups_is_eleven_full_names_including_protocol
    assert_equal(11, TestSelection.valid_groups.size)
    assert_includes(TestSelection.valid_groups, 'protocol')
    assert_includes(TestSelection.valid_groups, 'concurrency')
    assert_includes(TestSelection.valid_groups, 'invocation')
    assert_includes(TestSelection.valid_groups, 'load')
  end

  # (automation) parse_segment is crate-aware (#168 grammar). The `<crate>` is
  #   derived from the path form; a `tests/<file>.rs` with NO crate prefix
  #   means the `holler-cli` crate. dir defaults to '.'. interop appends
  #   `-- --ignored` and does NOT build any sibling binary.
  def test_grammar_crate_forms_resolve_to_expected_cargo_commands
    # (1) tests/<file>.rs            -> -p holler-cli --test <file>
    s1 = Automation.parse_segment('tests/cli_invocation_test.rs')
    assert_equal('holler-cli', s1.crate)
    assert_equal('cli_invocation_test', s1.file)
    assert_nil(s1.fn)
    assert_equal('cargo test -p holler-cli --test cli_invocation_test', s1.cmd)
    assert_equal('.', s1.dir)
    assert_equal({}, s1.env)
    assert_nil(s1.error)

    # (2) tests/<file>.rs (<fn>)     -> -p holler-cli --test <file> <fn>
    s2 = Automation.parse_segment('tests/cli_invocation_test.rs (version_flag_prints_crate_version)')
    assert_equal('holler-cli', s2.crate)
    assert_equal('version_flag_prints_crate_version', s2.fn)
    assert_equal('cargo test -p holler-cli --test cli_invocation_test version_flag_prints_crate_version', s2.cmd)

    # (3) crates/<crate>/tests/...   -> -p <crate> --test <file> <fn>
    s3 = Automation.parse_segment('crates/holler-hub/tests/token_store_test.rs (acquire_lock_serializes)')
    assert_equal('holler-hub', s3.crate)
    assert_equal('token_store_test', s3.file)
    assert_equal('acquire_lock_serializes', s3.fn)
    assert_equal('cargo test -p holler-hub --test token_store_test acquire_lock_serializes', s3.cmd)

    # (4) crates/<crate>/src/...     -> -p <crate> --lib <fn>  (unit)
    s4 = Automation.parse_segment('crates/holler-proto/src/envelope/frame.rs (frame_round_trips)')
    assert_equal('holler-proto', s4.crate)
    assert_equal('frame_round_trips', s4.fn)
    assert_equal('cargo test -p holler-proto --lib frame_round_trips', s4.cmd)
    assert_equal(true, s4.lib_test)

    # interop on a tests/ form: appends `-- --ignored`, NO env (single binary).
    s5 = Automation.parse_segment('tests/interop_test.rs (round_trip)', interop: true)
    assert_equal('cargo test -p holler-cli --test interop_test -- --ignored round_trip', s5.cmd)
    assert_equal({}, s5.env)

    # unparseable -> whole-workspace fallback, error-free.
    s6 = Automation.parse_segment('something the runner does not recognize')
    assert_equal('cargo test', s6.cmd)
    assert_equal(true, s6.fallback_used)
    assert_nil(s6.error)

    # the interop/whole-workspace/none distinctions
    s7 = Automation.parse_segment('tests/interop_test.rs', interop: true)
    assert_equal('cargo test -p holler-cli --test interop_test -- --ignored', s7.cmd)
  end
end

# ---------------------------------------------------------------------------
# Mechanism verification (#170 acceptance: "verify the mechanism against a
# synthetic/fixture case if needed"). The selection tests above cover the
# selection core in isolation; this covers the RUN mechanism -- the code that
# takes a test-run issue's pending rows and turns them into cargo invocations
# plus pass/fail statuses. The catalog is empty (0 real cases exist yet), so
# the mechanism is driven against a SYNTHETIC fixture through the real
# resolve_run_rows core, with the two things that touch the outside world
# (the per-crate `cargo test -p <crate> --lib` batch and each non-batched
# segment's captured cargo run) stubbed out -- exactly the hermetic fixture the
# acceptance clause calls for.
#
# Deliberately stdlib-only and octokit-free (like the selection tests above),
# so it runs on any runner's preinstalled Ruby. test-run.rb is required only
# for its top-level resolve_run_rows; the three functions it calls into are
# redefined here (last-definition wins), which keeps the test hermetic (no real
# cargo, no GitHub).
# ---------------------------------------------------------------------------

# Load the runner first (for its pure, top-level resolve_run_rows core), then
# redefine the three functions it calls into -- last definition wins, so these
# stubs shadow test-run.rb's real cargo/GitHub versions and the test stays
# hermetic. The stubs MUST be defined AFTER `require_relative 'test-run'`: if
# they came first, the require would overwrite them with the real (network/
# cargo) implementations and the fixture would run real cargo.
require_relative 'test-run'

# Stub stand-ins for the outside world, defined at top level (AFTER the
# require above) so test-run.rb's top-level resolve_run_rows resolves to them
# (last definition wins). They shadow the real -- network/cargo -- versions so
# the test is hermetic: no real cargo, no GitHub.
#
# FakeSegment mirrors the real Segment interface (crate/file/fn/lib_test/
# fallback_used/dir/cmd/error/env) and adds the three values a captured cargo
# run reports back: ok, output, exitstatus.
FakeSegment = Struct.new(:crate, :file, :fn, :lib_test, :fallback_used, :dir, :cmd, :error, :env, :ok, :output, :exitstatus, keyword_init: true)

# Fake cargo output for the unit-batch step: a { crate => output } map, where
# the output's `test <path> ... ok|FAILED` lines are what run_unit_batch parses
# into its { bare_fn => passed? } result map.
BATCH_OUTPUTS = {}

def run_unit_batch(_dir, crate)
  out = BATCH_OUTPUTS[crate].to_s
  results = {}
  out.each_line do |line|
    m = line.match(/^test (\S+) \.\.\. (ok|FAILED)/)
    next unless m

    results[m[1].split('::').last] = (m[2] == 'ok')
  end
  { out: out, results: results }
end

# The non-batched path's per-segment captured cargo run, keyed by crate.
SEG_RUN = {}

def exec_segment_captured(seg)
  run = SEG_RUN[seg.crate]
  raise "mechanism test: no stubbed segment run for crate #{seg.crate.inspect}" if run.nil?

  [run[:ok], run[:output], run[:exitstatus]]
end

# Which case IDs get unit-batched (normally driven by the test-cat-unit label).
BATCHABLE_IDS = []

def partition_unit_batchable(_rows, catalog, dir:, interop:)
  out = {}
  BATCHABLE_IDS.each do |id|
    cat = catalog.find { |c| c[:id] == id }
    next unless cat && cat[:automation]
    seg = Automation.parse_segment(cat[:automation], dir: dir, interop: interop)
    out[id] = [seg.crate, seg.fn] unless seg.error
  end
  out
end

class RunMechanismTest < Minitest::Test
  # A discover()-shaped synthetic catalog (the fixture). Three cases:
  #   hlr-1000 -> unit-batched (test-cat-unit), lib form.
  #   hlr-1130 -> NOT batched (no test-cat-unit), integration form.
  #   hlr-2010 -> unit-batched (test-cat-unit), lib form.
  def fixture_catalog
    [
      { id: 'hlr-1000', issue: 90, applies: 'body', group: 'invocation',
        automation: 'crates/holler-hub/src/sessions/session.rs (session_starts)',
        labels: %w[test-grp-invoc test-cat-unit test-hub test-body test-auto] },
      { id: 'hlr-1130', issue: 91, applies: 'body', group: 'concurrency',
        automation: 'crates/holler-cli/tests/token_cli_test.rs (list_prints_table)',
        labels: %w[test-grp-concurrency test-cat-regression test-auto] },
      { id: 'hlr-2010', issue: 92, applies: 'both', group: 'protocol',
        automation: 'crates/holler-proto/src/codec/encode.rs (encode_round_trips)',
        labels: %w[test-grp-protocol test-cat-unit test-hub test-body test-auto] }
    ]
  end

  # A test-run issue body whose rows are all pending auto rows (the input the
  # run mechanism resolves). include_failed controls whether the third (failing)
  # case is present.
  def fixture_body(include_failed:)
    rows = [
      "| [hlr-1000](https://github.com/Performant-Labs/holler/issues/90) | auto | ⏳ pending |  |",
      "| [hlr-1130](https://github.com/Performant-Labs/holler/issues/91) | auto | ⏳ pending |  |",
      "| [hlr-2010](https://github.com/Performant-Labs/holler/issues/92) | auto | ⏳ pending |  |"
    ]
    rows.pop if !include_failed
    <<~BODY
      <!-- test-run-fields:start -->
      | Field | Value |
      |---|---|
      | Commit | abc1234 |

      | Test Case | Type | Status | Evidence |
      |---|---|---|---|
      #{rows.join("\n")}
      <!-- test-run-fields:end -->
    BODY
  end

  # { row.id => row }. Inject (not Array#to_h-with-block, which needs Ruby
  # 2.6.0+) so the suite runs on macOS's preinstalled Ruby too.
  def rows_by_id(rows)
    rows.inject({}) { |h, r| h[r.id] = r; h }
  end

  # The real run mechanism against a synthetic fixture: a passing batched unit
  # case -> ✅; a non-batched integration case -> ✅; a FAILING batched unit
  # case -> ❌ with a result comment that posts the cargo output.
  def test_run_mechanism_pass_and_fail_against_fixture
    # Fake cargo --lib output, keyed by BARE fn name (the last `::` segment) --
    # exactly what run_unit_batch parses. holler-hub's session_starts passes;
    # holler-proto's encode_round_trips fails. (clear + []=: Hash#replace would
    # wipe the whole hash, so the first crate's entry would be lost.)
    BATCH_OUTPUTS.clear
    BATCH_OUTPUTS['holler-hub'] = "test session_starts ... ok\n"
    BATCH_OUTPUTS['holler-proto'] = "test encode_round_trips ... FAILED\n"
    BATCHABLE_IDS.replace(%w[hlr-1000 hlr-2010])
    SEG_RUN.clear
    SEG_RUN['holler-cli'] = { ok: true, output: "test result: ok. 1 passed; 0 failed; 0 filtered out", exitstatus: 0 }

    rows = extract_rows(fixture_body(include_failed: true))
    comments = resolve_run_rows(rows, fixture_catalog, dir: '.')

    by_id = rows_by_id(rows)
    assert_includes(by_id['hlr-1000'].status, '✅')
    assert_includes(by_id['hlr-1130'].status, '✅')
    assert_includes(by_id['hlr-2010'].status, '❌')

    # Exactly ONE comment (the failing case) is returned to post, and it
    # carries the failing test's cargo output and names the batched crate.
    assert_equal(1, comments.size)
    assert_match(/hlr-2010/, comments[0])
    assert_match(/cargo test -p holler-proto --lib/, comments[0])
    assert_match(/encode_round_trips \.\.\. FAILED/, comments[0])
  end

  # The "not found" sub-case: a unit-batched test the crate no longer defines
  # (renamed/deleted) -> ❌ with an explanatory note, not a silent pass.
  def test_run_mechanism_batched_case_missing_from_lib_output
    # holler-hub's --lib output contains a DIFFERENT test -- session_starts is
    # absent, so the batched case is "not found". (clear + []=: Hash#replace
    # would wipe the whole hash.)
    BATCH_OUTPUTS.clear
    BATCH_OUTPUTS['holler-hub'] = "test other_unit_test ... ok\n"
    BATCHABLE_IDS.replace(%w[hlr-1000])
    SEG_RUN.clear
    SEG_RUN['holler-cli'] = { ok: true, output: "test result: ok. 1 passed; 0 failed; 0 filtered out", exitstatus: 0 }

    rows = extract_rows(fixture_body(include_failed: false)) # only hlr-1000 + hlr-1130
    comments = resolve_run_rows(rows, fixture_catalog, dir: '.')

    by_id = rows_by_id(rows)
    # Not found -> ❌ (the evidence points to the comment; the "not found"
    # explanation lives in the comment itself, since the run otherwise "passed").
    assert_includes(by_id['hlr-1000'].status, '❌')
    assert_includes(by_id['hlr-1130'].status, '✅')
    assert_equal(1, comments.size)
    assert_match(/not found in --lib output/, comments[0])
    assert_match(/session_starts/, comments[0])
  end
end
