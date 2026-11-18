//! Which of the Rust toolchains of a project a job runs on

use rakko::VersionRange;

use crate::version::RustVersion;

/// The range that accepts every release, which passes over a channel
const RELEASES: &str = "*";

/// The range that accepts every dated nightly, which passes over a release
const DATED_NIGHTLIES: &str = "nightly-*";

/// Which of the Rust toolchains that a project pins a job runs on
///
/// A project can pin several Rust toolchains. Examples are a release to build
/// with, a dated nightly for unstable options, and the oldest release that
/// its manifests promise to compile on. A selection picks one of them for a
/// run. The lookup reads the pins in the order of the `rust` entry, and it
/// takes the first one that the selection picks.
///
/// A selection is no requirement. It belongs to the run, and nothing reads it
/// before a run starts. An action that accepts only some toolchains declares
/// that with [`Action::toolchains`][toolchains], and the lookup applies both.
/// A selection of the stable toolchain finds no pin that a requirement of the
/// nightly also accepts, and the run stops with an error. So an action that
/// requires a nightly also selects the nightly.
///
/// [toolchains]: rakko::Action::toolchains
///
/// # Examples
///
/// A job that must run on the release that a manifest declares selects that
/// release:
///
/// ```no_run
/// use rakko::{Action, Context};
/// use aonyx_rakko_cargo::{RustVersion, Toolchain, ToolchainSelection};
///
/// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
/// let context = Context::builder().root("/home/otter/project").build();
///
/// let toolchain = Toolchain::resolve()
///     .action(action)
///     .selection(ToolchainSelection::Release(RustVersion::new("1.88")))
///     .context(&context)
///     .call()
///     .await?;
///
/// println!("{toolchain}");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
#[non_exhaustive]
pub enum ToolchainSelection {
    /// The first pin of a release, such as `1.99.1`
    ///
    /// The lookup passes over a pin without a version, such as the channel
    /// `stable` or a dated nightly. It is the selection of a job that builds
    /// the project as a contributor does.
    #[default]
    Stable,

    /// The first pin of a dated nightly, such as `nightly-2026-11-02`
    ///
    /// The lookup passes over a release and a bare `nightly`. Mise can resolve
    /// a bare channel to a dated nightly that only one machine holds. So only
    /// a pin that writes the date counts.
    Nightly,

    /// The first pin that is a patch of a release, such as the `rust-version`
    /// of a manifest
    ///
    /// The version `1.88` accepts the pins `1.88.0` and `1.88.1`, and the
    /// version `1.88.0` accepts the pin `1.88.0` alone.
    Release(RustVersion),
}

impl ToolchainSelection {
    /// Returns the range of the pins that the selection picks from
    pub(crate) fn range(&self) -> VersionRange {
        match self {
            Self::Stable => VersionRange::new(RELEASES),
            Self::Nightly => VersionRange::new(DATED_NIGHTLIES),
            Self::Release(version) => VersionRange::new(format!("={}", version.get())),
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
    fn range_of_a_release_accepts_the_patches_of_the_version() {
        let selection = ToolchainSelection::Release(RustVersion::new("1.88"));

        assert_eq!(selection.range(), VersionRange::new("=1.88"));
    }

    #[test]
    fn range_of_the_nightly_accepts_every_dated_nightly() {
        let selection = ToolchainSelection::Nightly;

        assert_eq!(selection.range(), VersionRange::new("nightly-*"));
    }

    #[test]
    fn range_of_the_stable_toolchain_accepts_every_release() {
        let selection = ToolchainSelection::Stable;

        assert_eq!(selection.range(), VersionRange::new("*"));
    }

    #[test]
    fn toolchain_selection_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolchainSelection>();
    }
}
