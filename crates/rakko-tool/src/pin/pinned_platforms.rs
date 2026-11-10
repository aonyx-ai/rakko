//! The platforms that the pin of a tool lists

use std::collections::BTreeSet;

use rakko::Platform;

use super::{ByteEnd, ByteStart, Element, element};

/// The platforms that the pin of a tool lists in a mise file
///
/// In a constant context, Rust cannot build a set. So
/// [`MiseFile::pinned_platforms`][pinned] returns the text of the list, after
/// it checked each platform of the list. Convert the list into a set of
/// [`Platform`] values to give it to a [`ToolRequirement`][requirement].
///
/// # Examples
///
/// ```
/// use std::collections::BTreeSet;
///
/// use rakko::{OperatingSystem, Platform, RequiredVersion, ToolRequirement, ToolVersion};
/// use rakko_tool::{MiseFile, PinnedPlatforms};
///
/// const FILE: MiseFile =
///     MiseFile::new("[tools]\nzizmor = { version = \"1.32.0\", os = [\"linux\", \"macos\"] }\n");
/// const PLATFORMS: Option<PinnedPlatforms> = FILE.pinned_platforms("zizmor");
///
/// let version = ToolVersion::new(FILE.pinned_version("zizmor"));
/// let builder = ToolRequirement::builder().tool("zizmor").executable("zizmor");
/// let requirement = match PLATFORMS {
///     Some(platforms) => builder.exact_version_on(version, platforms).build(),
///     None => builder.version(RequiredVersion::Exact(version)).build(),
/// };
///
/// assert_eq!(
///     requirement.platforms(),
///     Some(&BTreeSet::from([
///         Platform::new(OperatingSystem::Linux),
///         Platform::new(OperatingSystem::Macos),
///     ]))
/// );
/// ```
///
/// [pinned]: crate::MiseFile::pinned_platforms
/// [requirement]: rakko::ToolRequirement
#[derive(Copy, Clone, Debug)]
pub struct PinnedPlatforms<'a>(&'a str);

impl<'a> PinnedPlatforms<'a> {
    /// Wraps the text of a list that the reader of the mise file checked
    ///
    /// The text starts after the opening bracket of the list, and it holds
    /// the closing bracket.
    pub(super) const fn new(text: &'a str) -> Self {
        Self(text)
    }
}

impl From<PinnedPlatforms<'_>> for BTreeSet<Platform> {
    /// Returns each platform of the list once
    fn from(pinned: PinnedPlatforms<'_>) -> Self {
        let text = pinned.0.as_bytes();
        let last = ByteEnd(text.len());
        let mut cursor = ByteStart(0);
        let mut platforms = Self::new();

        while let Element::Platform(platform, next) = element(text, cursor, last) {
            platforms.insert(platform);
            cursor = next;
        }

        platforms
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn pinned_platforms_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<PinnedPlatforms<'_>>();
    }
}
