//! The architecture of a platform

use std::fmt::{self, Display, Formatter};

/// The architecture of the processor of a platform
///
/// The text of each value is the name that the `os` option of mise uses, such
/// as `arm64`. [`Platform`][platform] combines an operating system with an
/// optional architecture.
///
/// Mise also accepts an alias for some architectures, such as `aarch64` for
/// `arm64`. Rakko uses only the names of this type, so that the pin of an
/// action and the generated file hold the same text.
///
/// A later release can add an architecture. So a match on `Architecture`
/// outside this crate needs an arm for a value that it does not know.
///
/// [platform]: crate::Platform
///
/// # Examples
///
/// ```
/// use rakko::Architecture;
///
/// assert_eq!(Architecture::Arm64.to_string(), "arm64");
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[non_exhaustive]
pub enum Architecture {
    /// The 64-bit architecture of Intel and AMD, which mise calls `x64`
    X64,
    /// The 64-bit architecture of ARM, which mise calls `arm64`
    Arm64,
}

impl Display for Architecture {
    /// Writes the name that the `os` option of mise uses
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::X64 => "x64",
            Self::Arm64 => "arm64",
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
    fn architecture_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Architecture>();
    }

    #[test]
    fn fmt_writes_the_names_of_mise() {
        let names =
            [Architecture::X64, Architecture::Arm64].map(|architecture| architecture.to_string());

        assert_eq!(names, ["x64", "arm64"]);
    }
}
