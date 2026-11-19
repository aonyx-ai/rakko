//! Which of the Rust toolchains of a project a job runs on

use rakko::{ToolchainRequirement, VersionRange};

use crate::cargo::RUST;
use crate::version::RustVersion;

/// The range that accepts every release, which passes over a channel
const RELEASES: &str = "*";

/// The range that accepts every dated nightly, which passes over a release
pub(crate) const DATED_NIGHTLIES: &str = "nightly-*";

/// The start of the date of a nightly in a range of dates, such as
/// `>=nightly-2025-02-20`
const NIGHTLY_DATE: &str = "nightly-";

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
///
/// A run that gives no selection gets one from the requirements of the
/// action. An action that requires a dated nightly of `rust`, such as
/// [`Toolchain::nightly_requirement`][nightly], runs on the first dated
/// nightly that the project pins. Every other action runs on the first
/// stable release. So an action that needs the nightly states it once, in
/// its requirements.
///
/// [nightly]: crate::Toolchain::nightly_requirement
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
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum ToolchainSelection {
    /// The first pin of a release, such as `1.99.1`
    ///
    /// The lookup passes over a pin without a version, such as the channel
    /// `stable` or a dated nightly. It is the selection of a job that builds
    /// the project as a contributor does.
    Stable,

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
            Self::Release(version) => VersionRange::new(format!("={}", version.get())),
        }
    }

    /// Returns the range of the pins that a run picks from, with the
    /// selection of the run or without one
    ///
    /// Without a selection, a requirement of `rust` whose range gives the
    /// dates of the nightly picks the dated nightly. Otherwise the run picks
    /// the stable release.
    pub(crate) fn range_of(
        selection: Option<&Self>,
        requirements: &[ToolchainRequirement],
    ) -> VersionRange {
        if let Some(selection) = selection {
            return selection.range();
        }

        let nightly = requirements.iter().any(|requirement| {
            requirement.toolchain().get() == RUST
                && requirement.versions().get().contains(NIGHTLY_DATE)
        });

        if nightly {
            VersionRange::new(DATED_NIGHTLIES)
        } else {
            Self::Stable.range()
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::MiseTool;

    use super::*;

    /// Returns a requirement of a toolchain with the given range
    fn requirement(toolchain: MiseTool, versions: VersionRange) -> ToolchainRequirement {
        ToolchainRequirement::builder()
            .toolchain(toolchain)
            .versions(versions)
            .build()
    }

    #[test]
    fn range_of_a_release_accepts_the_patches_of_the_version() {
        let selection = ToolchainSelection::Release(RustVersion::new("1.88"));

        assert_eq!(selection.range(), VersionRange::new("=1.88"));
    }

    #[test]
    fn range_of_a_selection_ignores_a_requirement_of_the_nightly() {
        let range = ToolchainSelection::range_of(
            Some(&ToolchainSelection::Release(RustVersion::new("1.88"))),
            &[requirement(
                MiseTool::new("rust"),
                VersionRange::new("nightly-*"),
            )],
        );

        assert_eq!(range, VersionRange::new("=1.88"));
    }

    #[test]
    fn range_of_the_stable_toolchain_accepts_every_release() {
        let selection = ToolchainSelection::Stable;

        assert_eq!(selection.range(), VersionRange::new("*"));
    }

    #[test]
    fn range_of_without_a_selection_and_a_requirement_of_another_toolchain_picks_the_stable_release()
     {
        let range = ToolchainSelection::range_of(
            None,
            &[requirement(
                MiseTool::new("node"),
                VersionRange::new("nightly-*"),
            )],
        );

        assert_eq!(range, VersionRange::new("*"));
    }

    #[test]
    fn range_of_without_a_selection_and_a_requirement_of_dates_picks_the_nightly() {
        let range = ToolchainSelection::range_of(
            None,
            &[requirement(
                MiseTool::new("rust"),
                VersionRange::new(">=nightly-2025-02-20"),
            )],
        );

        assert_eq!(range, VersionRange::new("nightly-*"));
    }

    #[test]
    fn range_of_without_a_selection_and_a_requirement_of_releases_picks_the_stable_release() {
        let range = ToolchainSelection::range_of(
            None,
            &[requirement(
                MiseTool::new("rust"),
                VersionRange::new(">=1.88"),
            )],
        );

        assert_eq!(range, VersionRange::new("*"));
    }

    #[test]
    fn range_of_without_a_selection_and_a_requirement_of_the_nightly_picks_the_nightly() {
        let range = ToolchainSelection::range_of(
            None,
            &[requirement(
                MiseTool::new("rust"),
                VersionRange::new("nightly-*"),
            )],
        );

        assert_eq!(range, VersionRange::new("nightly-*"));
    }

    #[test]
    fn range_of_without_a_selection_or_a_requirement_picks_the_stable_release() {
        let range = ToolchainSelection::range_of(None, &[]);

        assert_eq!(range, VersionRange::new("*"));
    }

    #[test]
    fn toolchain_selection_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolchainSelection>();
    }
}
