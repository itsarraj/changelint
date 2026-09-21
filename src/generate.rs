//! Pure skeleton generation: given a list of (tag, date) pairs, emits
//! a bare Keep a Changelog structure with empty `### Changed` stubs —
//! not commit-message-driven content (that's a different job, already
//! covered by this workspace's `commitguard changelog`), just the
//! version/date scaffolding a human then fills in by hand.

use crate::semver;

/// Sorts `tags` newest-first by parsed semver where possible; tags that
/// don't parse as semver are kept, sorted after all valid ones, in
/// their original relative order (stable sort), so nothing is silently
/// dropped.
pub fn sort_tags_newest_first(tags: &[(String, String)]) -> Vec<(String, String)> {
    let mut with_parsed: Vec<(Option<semver::SemVer>, (String, String))> = tags
        .iter()
        .map(|t| (semver::parse(&t.0), t.clone()))
        .collect();
    with_parsed.sort_by(|a, b| match (&a.0, &b.0) {
        (Some(x), Some(y)) => semver::compare(y, x), // descending
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    with_parsed.into_iter().map(|(_, t)| t).collect()
}

pub fn scaffold(tags: &[(String, String)]) -> String {
    let sorted = sort_tags_newest_first(tags);
    let mut out = String::new();
    out.push_str("# Changelog\n\n");
    out.push_str("All notable changes to this project will be documented in this file.\n\n");
    out.push_str(
        "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\n\
         and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n",
    );
    out.push_str("## [Unreleased]\n\n");

    for (tag, date) in &sorted {
        out.push_str(&format!("## [{tag}] - {date}\n\n### Changed\n- \n\n"));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_tags_orders_by_semver_descending() {
        let tags = vec![
            ("1.0.0".to_string(), "2024-01-01".to_string()),
            ("1.2.0".to_string(), "2024-03-01".to_string()),
        ];
        let sorted = sort_tags_newest_first(&tags);
        assert_eq!(sorted[0].0, "1.2.0");
        assert_eq!(sorted[1].0, "1.0.0");
    }

    #[test]
    fn sort_tags_puts_unparseable_tags_after_valid_ones_without_dropping_them() {
        let tags = vec![
            ("release-candidate".to_string(), "2024-01-01".to_string()),
            ("1.0.0".to_string(), "2024-02-01".to_string()),
        ];
        let sorted = sort_tags_newest_first(&tags);
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].0, "1.0.0");
        assert_eq!(sorted[1].0, "release-candidate");
    }

    #[test]
    fn scaffold_includes_unreleased_section_first() {
        let out = scaffold(&[]);
        assert!(out.contains("## [Unreleased]"));
        assert!(out.starts_with("# Changelog"));
    }

    #[test]
    fn scaffold_includes_one_section_per_tag_newest_first() {
        let tags = vec![
            ("1.0.0".to_string(), "2024-01-01".to_string()),
            ("1.1.0".to_string(), "2024-02-01".to_string()),
        ];
        let out = scaffold(&tags);
        let pos_110 = out.find("## [1.1.0] - 2024-02-01").unwrap();
        let pos_100 = out.find("## [1.0.0] - 2024-01-01").unwrap();
        assert!(pos_110 < pos_100);
    }

    #[test]
    fn scaffold_output_parses_back_cleanly() {
        let tags = vec![("2.0.0".to_string(), "2024-05-01".to_string())];
        let out = scaffold(&tags);
        let doc = crate::changelog::parse(&out);
        assert!(doc.has_title);
        assert_eq!(doc.releases.len(), 2); // Unreleased + 2.0.0
    }
}
