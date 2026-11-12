//! The test that ties the manifest on the landing page to this crate
//!
//! The landing page shows the dependencies of a harness as text, above the
//! code of this crate. Cargo cannot compile that text, so this test checks
//! that it lists the same dependency names as the manifest of this crate. It
//! does not compare the versions.

// A test reports a failure with a panic.
#![allow(clippy::missing_panics_doc)]

/// The manifest of this crate
const MANIFEST: &str = include_str!("../Cargo.toml");

/// The landing page, whose front matter holds the manifest that it shows
const LANDING_PAGE: &str = include_str!("../../../content/_index.md");

/// Returns the names in the first `[dependencies]` table of a text
///
/// The table ends at the next table or at the end of the string that holds
/// it in the front matter of the landing page.
fn dependencies(text: &str) -> Vec<&str> {
    text.lines()
        .skip_while(|line| line.trim() != "[dependencies]")
        .skip(1)
        .take_while(|line| !line.starts_with('[') && !line.starts_with("\"\"\""))
        .filter_map(|line| line.split_once(" = ").map(|(name, _)| name.trim()))
        .collect()
}

#[test]
fn manifest_on_the_landing_page_lists_the_dependencies_of_this_crate() {
    let shown = dependencies(LANDING_PAGE);

    assert_eq!(shown, dependencies(MANIFEST));
}
