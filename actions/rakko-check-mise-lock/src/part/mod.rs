//! One part of a lockfile of mise
//!
//! A lockfile holds a list of entries for each tool, and each entry records
//! one version. An entry holds a table for each platform that it records, and
//! fields that all of its platforms share. The action compares a lockfile one
//! part at a time, and a part is the record of one tool, one version, and one
//! platform. A finding names the part, so a reader knows which download is
//! missing or changed.

/// The platform of a part
mod platform;
/// The name of the tool of a part
mod tool_name;
/// The version of the tool of a part
mod version;

use std::fmt;

use bon::Builder;
use getset::Getters;

pub use self::platform::Platform;
pub use self::tool_name::ToolName;
pub use self::version::Version;

/// The identity of one part of a lockfile of mise
///
/// A part is the record of one tool, one version, and one platform. An entry
/// that records no platform, such as the entry of a tool that mise builds from
/// source, has one part without a platform.
///
/// Parts order by tool, then by version, then by platform, so the findings of
/// a run come in an order that does not change from one run to the next.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, Getters)]
pub struct Part {
    /// The tool that the part records
    #[getset(get = "pub")]
    tool: ToolName,

    /// The version of the tool that the part records
    #[getset(get = "pub")]
    version: Version,

    /// The platform that the part records, or `None` for an entry that
    /// records no platform
    #[getset(get = "pub")]
    platform: Option<Platform>,
}

/// Shows the part as a reader names it
///
/// The text names the tool and the version, and the platform when the part
/// has one, such as `jq 1.8.1 for linux-x64`.
impl fmt::Display for Part {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.tool, self.version)?;

        match &self.platform {
            Some(platform) => write!(formatter, " for {platform}"),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn display_of_a_part_with_a_platform_names_the_platform() {
        let part = Part::builder()
            .tool(ToolName::new("jq"))
            .version(Version::new("1.8.1"))
            .platform(Platform::new("linux-x64"))
            .build();

        assert_eq!(part.to_string(), "jq 1.8.1 for linux-x64");
    }

    #[test]
    fn display_of_a_part_without_a_platform_names_the_tool_and_the_version() {
        let part = Part::builder()
            .tool(ToolName::new("cargo:cargo-deny"))
            .version(Version::new("0.20.2"))
            .build();

        assert_eq!(part.to_string(), "cargo:cargo-deny 0.20.2");
    }
}
