//! The platform that a run takes place on

use std::env::consts;
use std::fmt::{self, Display, Formatter};

use bon::bon;

use super::Platform;

/// The platform of the process that runs an action
///
/// The machinery compares the host with the platforms of a tool, to learn
/// whether the tool has a build where the run takes place. The comparison
/// follows the rules of the `os` option of mise. The operating system must be
/// the same. A platform without an architecture covers every architecture of
/// its operating system, and a platform with an architecture covers only that
/// architecture.
///
/// The host holds the names that the `os` option of mise uses, such as `macos`
/// and `arm64`. The standard library calls the architectures `x86_64` and
/// `aarch64`, and mise calls them `x64` and `arm64`. A name that has no other
/// name in mise stays as the standard library gives it, such as `freebsd`. No
/// platform of a tool covers such a name, because a platform holds only the
/// names that Rakko knows.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct Host {
    /// The name of the operating system, in the form of mise
    operating_system: &'static str,

    /// The name of the architecture, in the form of mise
    architecture: &'static str,
}

#[bon]
impl Host {
    /// Returns the platform of the process that runs
    ///
    /// The answer is the platform that the harness was compiled for, because
    /// the standard library records it when the harness compiles.
    pub(crate) fn current() -> Self {
        Self::from_names()
            .operating_system(consts::OS)
            .architecture(consts::ARCH)
            .call()
    }

    /// Creates the host from the names that the standard library gives
    #[builder]
    fn from_names(operating_system: &'static str, architecture: &'static str) -> Self {
        let architecture = match architecture {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            other => other,
        };

        Self {
            operating_system,
            architecture,
        }
    }

    /// Returns whether the platform covers the host
    pub(crate) fn is_on(&self, platform: &Platform) -> bool {
        let same_system = platform.operating_system().to_string() == self.operating_system;

        same_system
            && platform
                .architecture()
                .is_none_or(|architecture| architecture.to_string() == self.architecture)
    }
}

impl Display for Host {
    /// Writes the host in the form that the `os` option of mise reads, such
    /// as `macos/arm64`
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.operating_system, self.architecture)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;
    use crate::tool::{Architecture, OperatingSystem};

    /// Returns an Apple Silicon Mac, with the names of the standard library
    fn apple_silicon() -> Host {
        Host::from_names()
            .operating_system("macos")
            .architecture("aarch64")
            .call()
    }

    #[test]
    fn fmt_writes_a_name_that_mise_does_not_rename_as_it_is() {
        let host = Host::from_names()
            .operating_system("freebsd")
            .architecture("riscv64")
            .call();

        assert_eq!(host.to_string(), "freebsd/riscv64");
    }

    #[test]
    fn fmt_writes_the_names_of_mise() {
        let host = Host::from_names()
            .operating_system("windows")
            .architecture("x86_64")
            .call();

        assert_eq!(host.to_string(), "windows/x64");
    }

    #[test]
    fn host_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Host>();
    }

    #[test]
    fn is_on_a_platform_of_another_architecture_is_false() {
        let platform = Platform::new(OperatingSystem::Macos).with_architecture(Architecture::X64);

        assert!(!apple_silicon().is_on(&platform));
    }

    #[test]
    fn is_on_a_platform_of_another_operating_system_is_false() {
        let platform = Platform::new(OperatingSystem::Linux);

        assert!(!apple_silicon().is_on(&platform));
    }

    #[test]
    fn is_on_a_platform_of_its_architecture_is_true() {
        let platform = Platform::new(OperatingSystem::Macos).with_architecture(Architecture::Arm64);

        assert!(apple_silicon().is_on(&platform));
    }

    #[test]
    fn is_on_a_platform_without_an_architecture_is_true() {
        let platform = Platform::new(OperatingSystem::Macos);

        assert!(apple_silicon().is_on(&platform));
    }

    #[test]
    fn is_on_with_an_operating_system_that_rakko_does_not_know_is_false() {
        let host = Host::from_names()
            .operating_system("freebsd")
            .architecture("x86_64")
            .call();

        assert!(!host.is_on(&Platform::new(OperatingSystem::Linux)));
    }
}
