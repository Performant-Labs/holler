# frozen_string_literal: true

# automation.rb -- the `Automation` grammar of the single-binary catalog
# (master testing issue #168), as a PURE function: one ';'-separated
# segment in, a resolved Segment (crate + cargo command) out.
#
# Kept separate from test_selection.rb (the selection core) on purpose:
# this file is exercised by a pure-function unit test (the
# grammar_crate_forms case in test_selection_test.rb) that must run with
# no GitHub client and no files on disk. test-run.rb `require_relative`s
# it so `run` and `exec` share one grammar implementation and cannot drift.
#
# The grammar (exact; #168 §5). `dir` is the checkout to run in (one
# `--dir` for the whole repo; default '.'). `<crate>` is derived from the
# path form -- a `tests/<file>.rs` with NO crate prefix means the
# `holler-cli` crate:
#
#   tests/<file>.rs                      -> cargo test -p holler-cli --test <file>
#   tests/<file>.rs (<fn>)               -> cargo test -p holler-cli --test <file> <fn>
#   crates/<crate>/tests/<file>.rs (<fn>)-> cargo test -p <crate> --test <file> <fn>
#   crates/<crate>/src/<path>.rs (<fn>) -> cargo test -p <crate> --lib <fn>  (unit)
#
# `;`-separated segments all must pass (handled by the caller, which splits
# on '; '). A leading "manual" segment is a manual procedure, never
# executed (the caller checks the raw field for this before parsing).
# A segment matching none of the forms above falls back to a whole-workspace
# `cargo test` and flags fallback_used -- the case's issue body must say so.
#
# interop: a case labeled test-tag-interop runs with `-- --ignored` (real
# cross-process hub+body runs). The single binary finds `holler` via
# cargo_bin, so NO sibling binary is built and NO env var is threaded --
# that was the two-repo holler-server layout. env is always {}.
#
# Ruby 2.6-compatible, stdlib-only.
#
# NOTE on portability: `String#match?` is Ruby 2.4+, fine here, but to stay
# safely inside the 2.6 floor used elsewhere in the runner we still use
# `!~` / `.match` / `.start_with?` (all 2.0/1.9-era) below.

module Automation
  Segment = Struct.new(
    :crate, :file, :fn, :lib_test, :fallback_used, :dir, :cmd, :error, :env,
    keyword_init: true
  )

  module_function

  # Parses one ';'-separated Automation segment into a Segment. Returns a
  # Segment with error set (and cmd nil) if the segment names an unknown
  # crate prefix that we can't map to a dir; returns a whole-workspace
  # fallback Segment (fallback_used true) if the segment matches none of the
  # grammar forms. See the file header for the exact forms.
  def parse_segment(seg, dir: '.', interop: false)
    seg = seg.to_s.strip
    return Segment.new(dir: dir, error: '[empty automation segment]') if seg.empty?

    crate = file = fn = nil
    lib_test = false

    if (m = seg.match(%r{\Acrates/([A-Za-z0-9_-]+)/tests/([A-Za-z0-9_]+)\.rs(?:\s*\(([A-Za-z0-9_]+)\))?\z}))
      crate, file, fn = m[1], m[2], m[3]
    elsif (m = seg.match(%r{\Acrates/([A-Za-z0-9_-]+)/src/[A-Za-z0-9_/]+\.rs\s*\(([A-Za-z0-9_]+)\)\z}))
      crate, fn = m[1], m[2]
      lib_test = true
    elsif (m = seg.match(%r{\Atests/([A-Za-z0-9_]+)\.rs(?:\s*\(([A-Za-z0-9_]+)\))?\z}))
      crate = 'holler-cli'
      file, fn = m[1], m[2]
    else
      # #168: anything not matching the grammar runs the whole workspace as a
      # conservative fallback; the case's body must note that a fallback ran.
      return Segment.new(dir: dir, fallback_used: true, cmd: 'cargo test', env: {})
    end

    env = {}
    cmd =
      if lib_test
        "cargo test -p #{crate} --lib #{fn}"
      elsif file
        if interop
          fn ? "cargo test -p #{crate} --test #{file} -- --ignored #{fn}" : "cargo test -p #{crate} --test #{file} -- --ignored"
        else
          fn ? "cargo test -p #{crate} --test #{file} #{fn}" : "cargo test -p #{crate} --test #{file}"
        end
      else
        'cargo test'
      end

    Segment.new(crate: crate, file: file, fn: fn, lib_test: lib_test,
                 fallback_used: false, dir: dir, cmd: cmd, env: env)
  end

  # Splits a full Automation field into its '; '-separated segments. The
  # separator is '; ' (semicolon + space) per #168 -- but we split on ';'
  # and trim so a stray ';'-without-space in authoring still parses rather
  # than silently falling through to the whole-workspace fallback.
  def split_segments(automation)
    automation.to_s.split(';').map(&:strip).reject(&:empty?)
  end

  # True if the whole Automation field is a manual procedure (leading
  # "manual"), which the runner never executes.
  def manual?(automation)
    automation.to_s.strip.match?(/\Amanual/i)
  end
end
