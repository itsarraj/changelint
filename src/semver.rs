//! A small, from-scratch semantic-version parser and precedence
//! comparator (SemVer 2.0.0 §11) — just enough to order and validate
//! the version headings a CHANGELOG.md uses. Not a full spec
//! implementation: build metadata is parsed and discarded (it never
//! affects precedence, per spec), and malformed input is reported
//! rather than panicking.

use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreReleaseIdentifier {
    Numeric(u64),
    Alpha(String),
}

impl PreReleaseIdentifier {
    fn parse(s: &str) -> Self {
        if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
            if let Ok(n) = s.parse::<u64>() {
                return PreReleaseIdentifier::Numeric(n);
            }
        }
        PreReleaseIdentifier::Alpha(s.to_string())
    }

    fn cmp_identifier(&self, other: &Self) -> Ordering {
        match (self, other) {
            (PreReleaseIdentifier::Numeric(a), PreReleaseIdentifier::Numeric(b)) => a.cmp(b),
            (PreReleaseIdentifier::Alpha(a), PreReleaseIdentifier::Alpha(b)) => a.cmp(b),
            // Per spec: numeric identifiers always have lower precedence than alphanumeric.
            (PreReleaseIdentifier::Numeric(_), PreReleaseIdentifier::Alpha(_)) => Ordering::Less,
            (PreReleaseIdentifier::Alpha(_), PreReleaseIdentifier::Numeric(_)) => Ordering::Greater,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: Vec<PreReleaseIdentifier>,
}

/// Parses a version string, tolerating a leading `v` (as in git tags
/// like `v1.2.3`) and discarding `+build` metadata.
pub fn parse(input: &str) -> Option<SemVer> {
    let s = input.strip_prefix('v').unwrap_or(input);
    let s = s.split('+').next().unwrap_or(s);
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (s, None),
    };

    let mut parts = core.split('.');
    let major: u64 = parts.next()?.parse().ok()?;
    let minor: u64 = parts.next()?.parse().ok()?;
    let patch: u64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None; // extra dotted segments in the core version
    }

    let pre_release = match pre {
        Some(p) if !p.is_empty() => p.split('.').map(PreReleaseIdentifier::parse).collect(),
        _ => Vec::new(),
    };

    Some(SemVer {
        major,
        minor,
        patch,
        pre_release,
    })
}

/// SemVer 2.0.0 §11 precedence: core version numerically, then
/// pre-release (a version *with* pre-release identifiers has *lower*
/// precedence than the same core version with none).
pub fn compare(a: &SemVer, b: &SemVer) -> Ordering {
    a.major
        .cmp(&b.major)
        .then(a.minor.cmp(&b.minor))
        .then(a.patch.cmp(&b.patch))
        .then_with(
            || match (a.pre_release.is_empty(), b.pre_release.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => compare_pre_release(&a.pre_release, &b.pre_release),
            },
        )
}

fn compare_pre_release(a: &[PreReleaseIdentifier], b: &[PreReleaseIdentifier]) -> Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        let ord = x.cmp_identifier(y);
        if ord != Ordering::Equal {
            return ord;
        }
    }
    // All shared identifiers equal — the longer list has higher precedence.
    a.len().cmp(&b.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_plain_version() {
        let v = parse("1.2.3").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (1, 2, 3));
        assert!(v.pre_release.is_empty());
    }

    #[test]
    fn parses_and_strips_a_leading_v_prefix() {
        let v = parse("v2.0.0").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 0, 0));
    }

    #[test]
    fn parses_pre_release_identifiers() {
        let v = parse("1.0.0-alpha.1").unwrap();
        assert_eq!(
            v.pre_release,
            vec![
                PreReleaseIdentifier::Alpha("alpha".into()),
                PreReleaseIdentifier::Numeric(1)
            ]
        );
    }

    #[test]
    fn discards_build_metadata() {
        let v = parse("1.0.0+build.123").unwrap();
        assert!(v.pre_release.is_empty());
        assert_eq!((v.major, v.minor, v.patch), (1, 0, 0));
    }

    #[test]
    fn rejects_non_numeric_core_components() {
        assert!(parse("1.x.0").is_none());
    }

    #[test]
    fn rejects_too_few_core_components() {
        assert!(parse("1.2").is_none());
    }

    #[test]
    fn rejects_too_many_core_components() {
        assert!(parse("1.2.3.4").is_none());
    }

    #[test]
    fn compares_major_minor_patch_numerically_not_lexically() {
        let a = parse("1.9.0").unwrap();
        let b = parse("1.10.0").unwrap();
        assert_eq!(compare(&a, &b), Ordering::Less);
    }

    #[test]
    fn release_has_higher_precedence_than_its_own_pre_release() {
        let release = parse("1.0.0").unwrap();
        let pre = parse("1.0.0-alpha").unwrap();
        assert_eq!(compare(&release, &pre), Ordering::Greater);
    }

    #[test]
    fn numeric_pre_release_identifiers_compare_numerically() {
        let a = parse("1.0.0-alpha.2").unwrap();
        let b = parse("1.0.0-alpha.10").unwrap();
        assert_eq!(compare(&a, &b), Ordering::Less);
    }

    #[test]
    fn alpha_pre_release_identifiers_outrank_numeric_ones() {
        let numeric = parse("1.0.0-1").unwrap();
        let alpha = parse("1.0.0-alpha").unwrap();
        assert_eq!(compare(&numeric, &alpha), Ordering::Less);
    }

    #[test]
    fn a_longer_pre_release_list_outranks_a_shared_prefix() {
        let short = parse("1.0.0-alpha").unwrap();
        let long = parse("1.0.0-alpha.1").unwrap();
        assert_eq!(compare(&short, &long), Ordering::Less);
    }
}
