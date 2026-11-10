//! The operating system of a platform

use std::fmt::{self, Display, Formatter};

/// The operating system of a platform
///
/// The text of each value is the name that the `os` option of mise uses, such
/// as `macos`. [`Platform`][platform] combines an operating system with an
/// optional architecture.
///
/// A later release can add an operating system. So a match on
/// `OperatingSystem` outside this crate needs an arm for a value that it does
/// not know.
///
/// [platform]: crate::Platform
///
/// # Examples
///
/// ```
/// use rakko::OperatingSystem;
///
/// assert_eq!(OperatingSystem::Macos.to_string(), "macos");
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[non_exhaustive]
pub enum OperatingSystem {
    /// Linux, which mise calls `linux`
    Linux,
    /// macOS, which mise calls `macos`
    Macos,
    /// Windows, which mise calls `windows`
    Windows,
}

impl Display for OperatingSystem {
    /// Writes the name that the `os` option of mise uses
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
            Self::Windows => "windows",
        };

        formatter.write_str(name)
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
    fn fmt_writes_the_names_of_mise() {
        let names = [
            OperatingSystem::Linux,
            OperatingSystem::Macos,
            OperatingSystem::Windows,
        ]
        .map(|system| system.to_string());

        assert_eq!(names, ["linux", "macos", "windows"]);
    }

    #[test]
    fn operating_system_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<OperatingSystem>();
    }
}
