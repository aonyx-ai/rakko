//! The nextest that a project runs
//!
//! This module holds the version of nextest that the crate declares, the
//! program that mise installed at that version, the cargo that its builds
//! start, the command line of a run, and the consent that the experimental
//! report of nextest needs. A caller names the workspace that it wants
//! tested, and everything between the caller and the process lives here.

mod error;
mod lockfile;

use rakko::{Context, ProjectRoot, RequiredVersion, ToolRequirement, ToolVersion};
use rakko_cargo::{Cargo, CargoRoot};
use rakko_tool::{MiseFile, ResolveRequirementError, Tool};

pub use self::error::ObserveNextestError;
pub use self::lockfile::Lockfile;
use crate::observation::Observation;

/// The name that mise knows the tool by
const CARGO_NEXTEST: &str = "cargo:cargo-nextest";

/// The program of the tool
const PROGRAM: &str = "cargo-nextest";

/// The version of nextest that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str =
    MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(CARGO_NEXTEST);

/// The arguments that ask nextest to run every test with every feature
///
/// The first argument is the subcommand of cargo that nextest provides.
/// Cargo hands it to the program of a plugin, and the program expects it
/// when it starts without cargo as well.
///
/// The reports of nextest and of cargo arrive as JSON, because the crate
/// reads them as data. The formats select the presentation of the reports
/// and not the behavior of the tools: how the tests run comes from the
/// configuration of the project alone.
const NEXTEST: [&str; 8] = [
    "nextest",
    "run",
    "--all-targets",
    "--all-features",
    "--message-format",
    "libtest-json-plus",
    "--cargo-message-format",
    "json",
];

/// The argument that asks cargo to build the versions of the lockfile
///
/// Cargo refuses the build when it would have to write the lockfile, so a
/// dependency that arrived since the lockfile was written ends the run
/// without success instead of joining the build unannounced.
const LOCKED: &str = "--locked";

/// The variable that gives consent to the experimental report of nextest
const CONSENT_VARIABLE: &str = "NEXTEST_EXPERIMENTAL_LIBTEST_JSON";

/// The value that gives consent
const CONSENT_VALUE: &str = "1";

/// The nextest that a project runs
///
/// The value holds the program of nextest that mise installed at the version
/// that the crate declares. The pin of the project does not change it, so a
/// run reaches the version that the tests of the crate cover. Nextest is a
/// plugin of cargo, so the value also holds the cargo that the builds of a
/// run start, on the stable toolchain that the project pins. Nothing here
/// installs a tool. The caller resolves cargo for the project and hands it
/// over, so that an action which also runs cargo for a job of its own asks
/// mise once.
///
/// An action that runs nextest declares [`requirement`][requirement] among
/// its tools, so that the machinery can read the version.
///
/// A run tests one workspace root and reports what nextest and cargo said
/// about it. The caller discovers the roots of the project and decides what
/// the answers mean for its outcome.
///
/// The caller also decides what a run does with the lockfile of the
/// workspace. A run that only examines the project holds cargo to the
/// lockfile, and a run that may change the project can let cargo resolve
/// the dependencies and write it.
///
/// # Examples
///
/// ```no_run
/// use rakko::Context;
/// use rakko_cargo::Cargo;
/// use rakko_nextest::{Lockfile, Nextest};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let context = Context::builder().root("/home/otter/project").build();
/// let cargo = Cargo::resolve(&context).await?;
/// let roots = cargo.roots().await?;
/// let nextest = Nextest::resolve().cargo(cargo).lockfile(Lockfile::Locked).context(&context).call().await?;
///
/// for workspace in &roots {
///     let observation = nextest.observe(workspace, context.root()).await?;
///
///     println!("{} tests ran", observation.ran());
/// }
/// # Ok(())
/// # }
/// ```
///
/// [requirement]: Nextest::requirement
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Nextest {
    /// The cargo that the builds of a run start
    cargo: Cargo,

    /// The program of nextest that mise installed at the declared version
    tool: Tool,

    /// What a run does with the lockfile of the workspace
    lockfile: Lockfile,
}

#[bon::bon]
impl Nextest {
    /// Returns the requirement of the nextest that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins. An action
    /// that runs nextest returns it from its declaration of tools.
    pub fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(CARGO_NEXTEST)
            .executable(PROGRAM)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the nextest that mise installed at the version of the crate,
    /// with the cargo that its builds start
    ///
    /// The lookup asks mise for the version that
    /// [`requirement`][requirement] holds, so a pin of nextest in the project
    /// does not change the answer. The lockfile tells a run whether cargo may
    /// resolve the dependencies of the build, or whether it builds the
    /// versions that the lockfile holds and refuses every other one.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run, or when
    /// it reports no nextest at that version, for example because nothing
    /// installed it.
    ///
    /// [requirement]: Nextest::requirement
    #[builder]
    pub async fn resolve(
        cargo: Cargo,
        lockfile: Lockfile,
        context: &Context,
    ) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement()
            .requirement(&Self::requirement())
            .context(context)
            .call()
            .await?;

        Ok(Self {
            cargo,
            tool,
            lockfile,
        })
    }

    /// Tests one workspace of the project and reads what the run reported
    ///
    /// The run starts in the directory of the root, and it names the files
    /// of its findings relative to the project root that the caller gives.
    /// The two need not be the directories of the contributor: a caller that
    /// tests a copy of the project names the roots of the copy, and the
    /// findings then carry the paths that the project has.
    ///
    /// Nothing outside the workspace of the root changes, whatever the run
    /// finds. Cargo builds in the target directory of that workspace, and it
    /// writes the lockfile of the workspace unless the value forbids it.
    ///
    /// # Errors
    ///
    /// Returns [`NextestUnavailable`][unavailable] when nextest does not run,
    /// and the error of the reading when the reports leave the run without
    /// an answer.
    ///
    /// [unavailable]: ObserveNextestError::NextestUnavailable
    pub async fn observe(
        &self,
        root: &CargoRoot,
        project: &ProjectRoot,
    ) -> Result<Observation, ObserveNextestError> {
        let invocation = self
            .cargo
            .plugin_invocation(root, &self.tool)
            .env(CONSENT_VARIABLE, CONSENT_VALUE)
            .args(NEXTEST);

        let invocation = match self.lockfile {
            Lockfile::Writable => invocation,
            Lockfile::Locked => invocation.arg(LOCKED),
        };

        let execution = invocation
            .run()
            .await
            .map_err(|source| ObserveNextestError::NextestUnavailable { source })?;

        Observation::read()
            .execution(&execution)
            .root(root)
            .project(project)
            .call()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nextest_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Nextest>();
    }
}
