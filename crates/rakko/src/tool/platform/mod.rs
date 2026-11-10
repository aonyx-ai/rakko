//! The platforms where a tool exists

mod architecture;
mod host;
mod operating_system;

use std::fmt::{self, Display, Formatter};

pub use self::architecture::Architecture;
pub(crate) use self::host::Host;
pub use self::operating_system::OperatingSystem;

/// A platform where a tool has a build
///
/// A platform is an operating system, or an operating system with an
/// architecture. A platform without an architecture covers every
/// architecture of its operating system.
///
/// The text of a platform is the form that the `os` option of mise reads,
/// such as `linux` or `macos/arm64`. Mise accepts a value that it does not
/// know and matches no platform with it. This type allows only the values
/// that it knows, so such a typo cannot reach mise.
///
/// # Examples
///
/// ```
/// use rakko::{Architecture, OperatingSystem, Platform};
///
/// let platform = Platform::new(OperatingSystem::Macos).with_architecture(Architecture::Arm64);
///
/// assert_eq!(platform.to_string(), "macos/arm64");
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Platform {
    /// The operating system of the platform
    operating_system: OperatingSystem,

    /// The architecture of the platform, or `None` for every architecture of
    /// the operating system
    architecture: Option<Architecture>,
}

impl Platform {
    /// Creates the platform of an operating system on every architecture
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{OperatingSystem, Platform};
    ///
    /// let platform = Platform::new(OperatingSystem::Linux);
    ///
    /// assert_eq!(platform.to_string(), "linux");
    /// ```
    #[must_use]
    pub const fn new(operating_system: OperatingSystem) -> Self {
        Self {
            operating_system,
            architecture: None,
        }
    }

    /// Returns the platform limited to one architecture of its operating
    /// system
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{Architecture, OperatingSystem, Platform};
    ///
    /// let platform = Platform::new(OperatingSystem::Windows).with_architecture(Architecture::X64);
    ///
    /// assert_eq!(platform.to_string(), "windows/x64");
    /// ```
    #[must_use]
    pub const fn with_architecture(self, architecture: Architecture) -> Self {
        Self {
            operating_system: self.operating_system,
            architecture: Some(architecture),
        }
    }

    /// Returns the operating system of the platform
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{OperatingSystem, Platform};
    ///
    /// let platform = Platform::new(OperatingSystem::Macos);
    ///
    /// assert_eq!(platform.operating_system(), OperatingSystem::Macos);
    /// ```
    #[must_use]
    pub const fn operating_system(&self) -> OperatingSystem {
        self.operating_system
    }

    /// Returns the architecture of the platform, or `None` when the platform
    /// covers every architecture of its operating system
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{Architecture, OperatingSystem, Platform};
    ///
    /// let platform = Platform::new(OperatingSystem::Macos).with_architecture(Architecture::Arm64);
    ///
    /// assert_eq!(platform.architecture(), Some(Architecture::Arm64));
    /// ```
    #[must_use]
    pub const fn architecture(&self) -> Option<Architecture> {
        self.architecture
    }
}

impl Display for Platform {
    /// Writes the platform in the form that the `os` option of mise reads
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.architecture {
            Some(architecture) => write!(formatter, "{}/{architecture}", self.operating_system),
            None => write!(formatter, "{}", self.operating_system),
        }
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
    fn fmt_with_an_architecture_writes_both_parts() {
        let platform = Platform::new(OperatingSystem::Windows).with_architecture(Architecture::X64);

        assert_eq!(platform.to_string(), "windows/x64");
    }

    #[test]
    fn fmt_without_an_architecture_writes_the_operating_system() {
        let platform = Platform::new(OperatingSystem::Linux);

        assert_eq!(platform.to_string(), "linux");
    }

    #[test]
    fn platform_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Platform>();
    }
}
