//! Pure lint rules over an already-parsed [`crate::changelog::ParsedChangelog`].

use std::cmp::Ordering;

use crate::changelog::{ParsedChangelog, VersionHeading};
use crate::date;
use crate::semver;

pub const STANDARD_CATEGORIES: &[&str] = &[
    "Added",
    "Changed",
    "Deprecated",
    "Removed",
    "Fixed",
    "Security",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub line: Option<usize>,
    pub message: String,
}

fn error(line: Option<usize>, message: impl Into<String>) -> Issue {
    Issue {
        severity: Severity::Error,
        line,
        message: message.into(),
    }
}
fn warning(line: Option<usize>, message: impl Into<String>) -> Issue {
    Issue {
        severity: Severity::Warning,
        line,
        message: message.into(),
    }
}

pub fn lint(doc: &ParsedChangelog) -> Vec<Issue> {
    let mut issues = Vec::new();

    if !doc.has_title {
        issues.push(warning(None, "missing a top-level '# ' title"));
    }

    for (line, raw) in &doc.malformed_headings {
        issues.push(error(
            Some(*line),
            format!("'## {raw}' does not match '[Unreleased]' or '[version] - YYYY-MM-DD' — its content is not being linted at all"),
        ));
    }

    if doc.releases.is_empty() && doc.malformed_headings.is_empty() {
        issues.push(warning(None, "no release sections ('## [...]') found"));
    }

    let mut seen_unreleased = false;
    let mut prior_versioned: Option<(&str, semver::SemVer, Option<date::Date>, usize)> = None;
    let mut seen_versions: Vec<(semver::SemVer, String)> = Vec::new();

    for (idx, release) in doc.releases.iter().enumerate() {
        for (category, _items) in &release.sections {
            if !STANDARD_CATEGORIES.contains(&category.as_str()) {
                issues.push(error(
                    Some(release.line_number),
                    format!(
                        "unknown category '{category}' (expected one of {})",
                        STANDARD_CATEGORIES.join(", ")
                    ),
                ));
            }
        }

        match &release.heading {
            VersionHeading::Unreleased => {
                if seen_unreleased {
                    issues.push(error(
                        Some(release.line_number),
                        "more than one '[Unreleased]' section",
                    ));
                }
                seen_unreleased = true;
                if idx != 0 {
                    issues.push(error(
                        Some(release.line_number),
                        "'[Unreleased]' must be the first release section",
                    ));
                }
                if release.date.is_some() {
                    issues.push(warning(
                        Some(release.line_number),
                        "'[Unreleased]' should not have a release date",
                    ));
                }
            }
            VersionHeading::Version(raw) => {
                let parsed_version = semver::parse(raw);
                if parsed_version.is_none() {
                    issues.push(error(
                        Some(release.line_number),
                        format!("'{raw}' is not a valid semantic version"),
                    ));
                }

                let parsed_date = match &release.date {
                    None => {
                        issues.push(error(
                            Some(release.line_number),
                            format!("version '{raw}' is missing a release date"),
                        ));
                        None
                    }
                    Some(d) => {
                        let parsed = date::parse(d);
                        if parsed.is_none() {
                            issues.push(error(
                                Some(release.line_number),
                                format!("'{d}' is not a valid YYYY-MM-DD date"),
                            ));
                        }
                        parsed
                    }
                };

                if !doc.link_refs.iter().any(|r| r == raw) {
                    issues.push(warning(
                        Some(release.line_number),
                        format!("no '[{raw}]: <url>' link reference definition found"),
                    ));
                }

                if let Some(v) = &parsed_version {
                    if let Some((prev_raw, prev_v, prev_date, prev_line)) = &prior_versioned {
                        match semver::compare(v, prev_v) {
                            Ordering::Greater | Ordering::Equal => {
                                issues.push(error(
                                    Some(release.line_number),
                                    format!("version '{raw}' is not lower than the preceding version '{prev_raw}' (line {prev_line}) — releases must be listed newest-first"),
                                ));
                            }
                            Ordering::Less => {}
                        }
                        if let (Some(d), Some(pd)) = (&parsed_date, prev_date) {
                            if d > pd {
                                issues.push(error(
                                    Some(release.line_number),
                                    format!("version '{raw}' has a later date ({d:?}) than the preceding, newer version '{prev_raw}' ({pd:?})"),
                                ));
                            }
                        }
                    }
                    prior_versioned = Some((raw, v.clone(), parsed_date, release.line_number));

                    if let Some((_, dup)) = seen_versions
                        .iter()
                        .find(|(sv, _)| semver::compare(sv, v) == Ordering::Equal)
                    {
                        let _ = dup;
                        issues.push(error(
                            Some(release.line_number),
                            format!("duplicate version '{raw}'"),
                        ));
                    } else {
                        seen_versions.push((v.clone(), raw.clone()));
                    }
                }
            }
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changelog::parse;

    fn has_message_containing(issues: &[Issue], needle: &str) -> bool {
        issues.iter().any(|i| i.message.contains(needle))
    }

    #[test]
    fn clean_changelog_produces_no_errors() {
        let md = "# Changelog\n\n## [Unreleased]\n\n## [1.1.0] - 2024-02-01\n\n### Added\n- thing\n\n## [1.0.0] - 2024-01-01\n\n### Added\n- initial\n\n[Unreleased]: https://example.com/compare/1.1.0...HEAD\n[1.1.0]: https://example.com/compare/1.0.0...1.1.0\n[1.0.0]: https://example.com/releases/1.0.0\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(
            issues.iter().all(|i| i.severity == Severity::Warning),
            "unexpected errors: {issues:?}"
        );
    }

    #[test]
    fn flags_unknown_category_name() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-01-01\n\n### Improvements\n- thing\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(has_message_containing(
            &issues,
            "unknown category 'Improvements'"
        ));
    }

    #[test]
    fn flags_invalid_semver() {
        let md = "# Changelog\n\n## [1.0] - 2024-01-01\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(has_message_containing(
            &issues,
            "not a valid semantic version"
        ));
    }

    #[test]
    fn flags_invalid_date() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-13-01\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(has_message_containing(
            &issues,
            "not a valid YYYY-MM-DD date"
        ));
    }

    #[test]
    fn flags_out_of_order_versions() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-01-01\n\n## [1.1.0] - 2024-02-01\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(has_message_containing(
            &issues,
            "not lower than the preceding version"
        ));
    }

    #[test]
    fn flags_duplicate_versions() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-02-01\n\n## [1.0.0] - 2024-01-01\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(has_message_containing(&issues, "duplicate version"));
    }

    #[test]
    fn flags_date_out_of_order_even_when_versions_are_correctly_ordered() {
        // Newer version (1.1.0) dated before the older one (1.0.0) — contradictory history.
        let md = "# Changelog\n\n## [1.1.0] - 2024-01-01\n\n## [1.0.0] - 2024-02-01\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(has_message_containing(&issues, "has a later date"));
    }

    #[test]
    fn flags_missing_release_date() {
        let md = "# Changelog\n\n## [1.0.0]\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(has_message_containing(&issues, "missing a release date"));
    }

    #[test]
    fn flags_unreleased_not_first() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-01-01\n\n## [Unreleased]\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(has_message_containing(
            &issues,
            "must be the first release section"
        ));
    }

    #[test]
    fn flags_duplicate_unreleased_sections() {
        let md = "# Changelog\n\n## [Unreleased]\n\n## [Unreleased]\n";
        let doc = parse(md);
        let issues = lint(&doc);
        assert!(has_message_containing(&issues, "more than one"));
    }

    #[test]
    fn warns_about_missing_link_reference() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-01-01\n";
        let doc = parse(md);
        let issues = lint(&doc);
        let warning = issues
            .iter()
            .find(|i| i.message.contains("link reference"))
            .unwrap();
        assert_eq!(warning.severity, Severity::Warning);
    }

    #[test]
    fn flags_malformed_non_bracket_headings_as_errors() {
        let md = "# Changelog\n\n## Paru v2.0.4 (2026-07-08)\n\n### Added\n- something\n";
        let doc = parse(md);
        let issues = lint(&doc);
        let issue = issues
            .iter()
            .find(|i| i.message.contains("does not match"))
            .unwrap();
        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.line, Some(3));
    }

    #[test]
    fn warns_about_missing_title_but_does_not_error() {
        let md = "## [1.0.0] - 2024-01-01\n\n[1.0.0]: https://example.com\n";
        let doc = parse(md);
        let issues = lint(&doc);
        let issue = issues.iter().find(|i| i.message.contains("title")).unwrap();
        assert_eq!(issue.severity, Severity::Warning);
    }
}
