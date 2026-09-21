//! Pure Markdown parsing for the Keep a Changelog structure. Not a
//! general Markdown parser — Keep a Changelog's format is a narrow,
//! predictable subset (H1 title, H2 version headings, H3 category
//! headings, `-`/`*` bullet items, and `[ref]: url` link-reference
//! definitions at the bottom), so a line-based scan is enough and
//! keeps this dependency-free.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionHeading {
    Unreleased,
    Version(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub heading: VersionHeading,
    pub date: Option<String>,
    pub line_number: usize,
    pub sections: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedChangelog {
    pub has_title: bool,
    pub releases: Vec<Release>,
    pub link_refs: Vec<String>,
    /// `## ` headings that don't match `[Unreleased]` or `[version] -
    /// date`, e.g. `## v2.0.4 (2026-07-08)` — kept separately (line
    /// number + raw text) rather than silently dropped, since a
    /// heading that doesn't parse is itself the most basic possible
    /// format violation a linter exists to catch.
    pub malformed_headings: Vec<(usize, String)>,
}

fn parse_h2(rest: &str) -> Option<(VersionHeading, Option<String>)> {
    let rest = rest.trim();
    let bracket_end = rest.find(']')?;
    if !rest.starts_with('[') {
        return None;
    }
    let inside = &rest[1..bracket_end];
    let after = rest[bracket_end + 1..].trim();
    let date = after.strip_prefix('-').map(|d| d.trim().to_string());

    if inside.eq_ignore_ascii_case("unreleased") {
        Some((VersionHeading::Unreleased, date))
    } else {
        Some((VersionHeading::Version(inside.to_string()), date))
    }
}

pub fn parse(markdown: &str) -> ParsedChangelog {
    let mut doc = ParsedChangelog::default();
    let mut current_release: Option<Release> = None;

    let flush_release = |doc: &mut ParsedChangelog, release: Option<Release>| {
        if let Some(r) = release {
            doc.releases.push(r);
        }
    };

    for (idx, raw_line) in markdown.lines().enumerate() {
        let line_number = idx + 1;
        let line = raw_line.trim_end();

        if let Some(rest) = line.strip_prefix("# ") {
            if !rest.trim().is_empty() {
                doc.has_title = true;
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("## ") {
            flush_release(&mut doc, current_release.take());
            match parse_h2(rest) {
                Some((heading, date)) => {
                    current_release = Some(Release {
                        heading,
                        date,
                        line_number,
                        sections: Vec::new(),
                    });
                }
                None => {
                    doc.malformed_headings
                        .push((line_number, rest.trim().to_string()));
                }
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("### ") {
            if let Some(release) = current_release.as_mut() {
                release.sections.push((rest.trim().to_string(), Vec::new()));
            }
            continue;
        }

        let trimmed = line.trim_start();
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            if let Some(release) = current_release.as_mut() {
                if let Some(last) = release.sections.last_mut() {
                    last.1.push(item.trim().to_string());
                }
            }
            continue;
        }

        if let Some(bracket_end) = line.strip_prefix('[').and_then(|_| line.find("]: ")) {
            let reference = &line[1..bracket_end];
            doc.link_refs.push(reference.to_string());
        }
    }
    flush_release(&mut doc, current_release.take());

    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_the_h1_title() {
        let doc = parse("# Changelog\n\nSome text.\n");
        assert!(doc.has_title);
    }

    #[test]
    fn missing_title_is_detected() {
        let doc = parse("## [1.0.0] - 2024-01-01\n");
        assert!(!doc.has_title);
    }

    #[test]
    fn parses_unreleased_heading_with_no_date() {
        let doc = parse("# Changelog\n\n## [Unreleased]\n");
        assert_eq!(doc.releases.len(), 1);
        assert_eq!(doc.releases[0].heading, VersionHeading::Unreleased);
        assert_eq!(doc.releases[0].date, None);
    }

    #[test]
    fn parses_a_versioned_release_with_date() {
        let doc = parse("# Changelog\n\n## [1.2.3] - 2024-03-15\n");
        assert_eq!(
            doc.releases[0].heading,
            VersionHeading::Version("1.2.3".to_string())
        );
        assert_eq!(doc.releases[0].date.as_deref(), Some("2024-03-15"));
    }

    #[test]
    fn parses_sections_and_bullet_items_under_a_release() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-01-01\n\n### Added\n- new thing\n- another thing\n\n### Fixed\n- a bug\n";
        let doc = parse(md);
        let release = &doc.releases[0];
        assert_eq!(release.sections.len(), 2);
        assert_eq!(release.sections[0].0, "Added");
        assert_eq!(
            release.sections[0].1,
            vec!["new thing".to_string(), "another thing".to_string()]
        );
        assert_eq!(release.sections[1].0, "Fixed");
        assert_eq!(release.sections[1].1, vec!["a bug".to_string()]);
    }

    #[test]
    fn supports_asterisk_bullets_as_well_as_dashes() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-01-01\n\n### Added\n* new thing\n";
        let doc = parse(md);
        assert_eq!(doc.releases[0].sections[0].1, vec!["new thing".to_string()]);
    }

    #[test]
    fn parses_multiple_releases_in_order() {
        let md = "# Changelog\n\n## [Unreleased]\n\n## [1.1.0] - 2024-02-01\n\n## [1.0.0] - 2024-01-01\n";
        let doc = parse(md);
        assert_eq!(doc.releases.len(), 3);
        assert_eq!(doc.releases[0].heading, VersionHeading::Unreleased);
        assert_eq!(
            doc.releases[1].heading,
            VersionHeading::Version("1.1.0".to_string())
        );
        assert_eq!(
            doc.releases[2].heading,
            VersionHeading::Version("1.0.0".to_string())
        );
    }

    #[test]
    fn collects_link_reference_definitions() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-01-01\n\n[1.0.0]: https://example.com/releases/1.0.0\n[Unreleased]: https://example.com/compare/1.0.0...HEAD\n";
        let doc = parse(md);
        assert_eq!(
            doc.link_refs,
            vec!["1.0.0".to_string(), "Unreleased".to_string()]
        );
    }

    #[test]
    fn a_non_bracket_h2_heading_is_recorded_as_malformed_not_silently_dropped() {
        let md = "# Changelog\n\n## Paru v2.0.4 (2026-07-08)\n\n### Added\n- something\n";
        let doc = parse(md);
        assert!(doc.releases.is_empty());
        assert_eq!(doc.malformed_headings.len(), 1);
        assert_eq!(doc.malformed_headings[0].1, "Paru v2.0.4 (2026-07-08)");
    }

    #[test]
    fn a_bullet_before_any_category_heading_is_ignored_not_misattributed() {
        let md = "# Changelog\n\n## [1.0.0] - 2024-01-01\n- orphan bullet, no ### category yet\n\n### Added\n- real item\n";
        let doc = parse(md);
        assert_eq!(doc.releases[0].sections.len(), 1);
        assert_eq!(doc.releases[0].sections[0].1, vec!["real item".to_string()]);
    }
}
