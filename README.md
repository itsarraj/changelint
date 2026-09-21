# changelint

Validates a `CHANGELOG.md` against the [Keep a
Changelog](https://keepachangelog.com/) format, and scaffolds a fresh
skeleton from real git tags.

This is a different job from this workspace's `commitguard`, which
already generates changelog *content* from conventional-commit
messages, grouped by commit type. `changelint` doesn't read commit
messages at all — it either **lints** a changelog file that already
exists (checking its structure, not its prose) or **scaffolds** a bare
version/date skeleton from tag history for a human to fill in by hand.

## Usage

```bash
changelint                          # lint ./CHANGELOG.md (default)
changelint lint path/to/CHANGELOG.md
changelint scaffold                 # print a skeleton built from `git tag` history
changelint scaffold -o CHANGELOG.md # write it to a file
```

Exit code `0` means no errors (warnings are still printed but don't
fail the run); `1` means at least one error; `2` means the file
couldn't be read or `git` failed.

## What gets checked

- A top-level `# ` title (warning if missing).
- Every `## ` heading must be `[Unreleased]` or `[version] -
  YYYY-MM-DD` — anything else (a heading that doesn't start with `[`)
  is flagged as an error naming exactly which line, rather than
  silently ignored.
- `[Unreleased]` must appear at most once, and must be the first
  release section if present.
- Every version must be valid [SemVer
  2.0.0](https://semver.org/spec/v2.0.0.html) (hand-rolled parser and
  precedence comparator — handles pre-release identifiers like
  `1.0.0-alpha.1` correctly, including numeric-vs-alphanumeric
  precedence rules).
- Every release date must be a real calendar date (`YYYY-MM-DD`,
  including leap-year rules) and present.
- Releases must be listed newest-first, both by version precedence and
  by date — a newer version dated *before* an older one is flagged
  even if the version ordering itself looks fine, since that's
  internally contradictory history.
- No duplicate versions.
- Every `### ` category must be one of Keep a Changelog's six:
  Added, Changed, Deprecated, Removed, Fixed, Security.
- Each version should have a matching `[version]: <url>` link
  reference definition (warning, not an error — plenty of real
  changelogs skip these).

## Status: built, unit-tested, and live-verified against real-world CHANGELOG.md files with genuinely different compliance levels, plus a real git-tag scaffold round-trip

- **51 unit tests** (`cargo test --lib`) across semver parsing/precedence
  (core-version numeric comparison, pre-release identifier precedence
  including the numeric-vs-alphanumeric and shorter-vs-longer-list
  rules from the spec), calendar-date validation (leap years, century
  non-leap years, the 400-divisible exception, invalid months/days),
  Markdown structure parsing, and every lint rule in isolation.
  **A real, serious bug caught by live-testing against real files, not
  by the unit tests** (see below): a `## ` heading that didn't match
  the expected `[version]` bracket format was silently dropped instead
  of flagged, so a badly malformed real changelog reported "looks
  good" with zero issues — the worst possible failure mode for a
  linter. Fixed by tracking malformed headings separately and emitting
  an error naming the exact line and raw heading text, with a unit
  test (`a_non_bracket_h2_heading_is_recorded_as_malformed_not_silently_dropped`)
  added to lock in the fix.
- **Live-verified against real, unmodified `CHANGELOG.md` files found
  on this machine** from unrelated real-world projects, not fixtures
  written for this tool:
  - `paru`'s (a real AUR helper written in Rust) real CHANGELOG.md uses
    headings like `## Paru v2.0.4 (2026-07-08)` — a genuinely different,
    non-Keep-a-Changelog convention — across roughly 40 releases. This
    is exactly the file that surfaced the bug above: after the fix,
    `changelint` correctly reported **39 errors, one per malformed
    heading** (including a real, existing inconsistency in that file
    where a subsection uses `##` instead of `###`, e.g. `## Fixed` on
    line 335), and exited with status 1.
  - A real VS Code extension's (`hyshine.markdown-lint`) CHANGELOG.md
    *does* follow the Keep a Changelog bracket convention correctly
    across 10 releases: `changelint` reported **zero errors**, only the
    expected "no link reference" warnings (the file doesn't use link
    references), and exited 0.
- **Live-verified `scaffold` against a real git repository with real
  annotated tags**: created three real commits and three real
  `git tag -a` annotated tags (`v1.0.0`, `v1.1.0`, `v2.0.0` with real,
  distinct commit dates) in a throwaway repo, ran `changelint scaffold`
  against it, and got a correctly newest-first-ordered skeleton with
  the real tag names and real dates from `git for-each-ref`. Then
  **round-tripped that generated file back through `changelint lint`**:
  zero errors, only the expected missing-link-reference warnings —
  proving the generator's own output satisfies the linter's rules, not
  just eyeballing the two features in isolation.

**Not done / deliberately deferred**: no auto-fix mode (reports issues,
doesn't rewrite the file); doesn't validate that link reference URLs
actually resolve or point at the right comparison range, only that a
reference with the matching name exists; doesn't cross-check `##
[Unreleased]` bullet content against actual unreleased commits (that
would mean re-implementing `commitguard`'s commit-message-driven
generator, which already exists in this workspace).
