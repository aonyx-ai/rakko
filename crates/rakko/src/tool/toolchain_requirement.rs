//! What an action requires of a toolchain that the project pins

use bon::bon;
use getset::Getters;

use crate::tool::{MiseTool, VersionRange};

/// What an action requires of a toolchain that the project pins
///
/// A toolchain is a compiler or a runtime of the project, such as Rust or
/// Node.js. The project pins its toolchains, and mise installs them. An
/// action does not pin a toolchain. It can only require the versions of a
/// toolchain that it accepts, such as a minimum version, a range, or a dated
/// Rust nightly.
///
/// The requirement is optional. An action declares one only for a need of
/// its own, such as a release that a result of the action depends on. An
/// action without a requirement runs on the toolchain that the project pins.
/// A requirement adds no pin to the generated file of a project.
///
/// The machinery reads the requirements of an action from
/// [`Action::toolchains`][toolchains] before any run. A run compares them with
/// the pin of the project when the action resolves the toolchain. When the
/// project pins no version that every requirement accepts, only that action
/// stops with an error, which gives the pin to change.
///
/// The range uses the syntax of [`VersionRange`]. A range of dates of one
/// channel, such as `>=nightly-2025-02-20`, accepts only the pins of that
/// channel that the project writes with a date.
///
/// [toolchains]: crate::Action::toolchains
///
/// # Examples
///
/// A minimum version, a range with two alternatives, and a dated nightly:
///
/// ```
/// use rakko::{ToolchainRequirement, VersionRange};
///
/// let minimum = ToolchainRequirement::builder()
///     .toolchain("node")
///     .versions(VersionRange::new(">=22.18"))
///     .build();
/// let range = ToolchainRequirement::builder()
///     .toolchain("node")
///     .versions(VersionRange::new("^22.18 || >=23.6"))
///     .build();
/// let nightly = ToolchainRequirement::builder()
///     .toolchain("rust")
///     .versions(VersionRange::new(">=nightly-2025-02-20"))
///     .build();
///
/// assert_eq!(minimum.toolchain().get(), "node");
/// assert_eq!(range.versions().get(), "^22.18 || >=23.6");
/// assert_eq!(nightly.toolchain().get(), "rust");
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct ToolchainRequirement {
    /// The toolchain as the configuration of mise identifies it, such as
    /// `node` or `rust`
    #[getset(get = "pub")]
    toolchain: MiseTool,

    /// The versions of the toolchain that the action accepts
    #[getset(get = "pub")]
    versions: VersionRange,
}

#[bon]
impl ToolchainRequirement {
    /// Creates the requirement of the versions of a toolchain that an action
    /// accepts
    ///
    /// Nothing reads the range here. The resolution of the toolchain reads
    /// it, and a text that is not a range stops the action with an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{ToolchainRequirement, VersionRange};
    ///
    /// let requirement = ToolchainRequirement::builder()
    ///     .toolchain("node")
    ///     .versions(VersionRange::new(">=22.18"))
    ///     .build();
    ///
    /// assert_eq!(requirement.versions().get(), ">=22.18");
    /// ```
    #[builder]
    pub fn new(#[builder(into)] toolchain: MiseTool, versions: VersionRange) -> Self {
        Self {
            toolchain,
            versions,
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
    fn toolchain_requirement_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolchainRequirement>();
    }

    #[test]
    fn toolchain_returns_the_given_toolchain() {
        let requirement = ToolchainRequirement::builder()
            .toolchain("node")
            .versions(VersionRange::new(">=22.18"))
            .build();

        assert_eq!(requirement.toolchain().get(), "node");
    }

    #[test]
    fn versions_returns_the_given_range() {
        let requirement = ToolchainRequirement::builder()
            .toolchain("node")
            .versions(VersionRange::new(">=22.18"))
            .build();

        assert_eq!(requirement.versions(), &VersionRange::new(">=22.18"));
    }
}
