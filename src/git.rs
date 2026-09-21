//! Real `git` invocation — reads actual tag history from a real
//! repository, no libgit2 dependency needed for a read this narrow.

use std::process::Command;

use anyhow::{bail, Context, Result};

/// Lists every tag in the current repository with its creation date
/// (`YYYY-MM-DD`), newest first.
pub fn list_tags_with_dates() -> Result<Vec<(String, String)>> {
    let output = Command::new("git")
        .args([
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(refname:short)|%(creatordate:short)",
            "refs/tags",
        ])
        .output()
        .context("running 'git for-each-ref' (is git installed and is this a git repository?)")?;

    if !output.status.success() {
        bail!(
            "'git for-each-ref' failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(tag, date)| (tag.to_string(), date.to_string()))
        .collect())
}
