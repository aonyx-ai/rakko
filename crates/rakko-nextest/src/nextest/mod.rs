//! The nextest that a project runs
//!
//! This module holds the cargo that carries nextest, the command line of a
//! run, and the consent that the experimental report of nextest needs. A
//! caller names the workspace that it wants tested, and everything between
//! the caller and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// What a run does with the lockfile of the workspace
mod lockfile;

use rakko_action::ProjectRoot;
use rakko_cargo::{Cargo, CargoRoot};

pub use self::error::ObserveNextestError;
pub use self::lockfile::Lockfile;
use crate::observation::Observation;

/// The arguments that ask nextest to run every test with every feature
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
/// The value holds the cargo that mise installed for the project, because
/// nextest is a plugin of cargo: cargo finds it on the path of the
/// environment that mise sets, at the version that the project pinned.
/// Nothing here installs a tool, and nothing here resolves one. The caller
/// resolves cargo for the project and hands it over, so that an action which
/// also runs cargo for a job of its own asks mise once.
///
/// A run tests one workspace root and reports what nextest and cargo said
/// about it. The caller discovers the roots of the project and decides what
/// the answers mean for its outcome.
///
/// The caller also decides what a run does with the lockfile of the
/// workspace. A run that answers for the project as it stands lets cargo
/// resolve the dependencies, and a run that answers for a resolution which
/// another job produced holds cargo to it.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_cargo::Cargo;
/// use rakko_nextest::{Lockfile, Nextest};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
/// let cargo = Cargo::resolve(root.clone()).await?;
/// let roots = cargo.roots().await?;
/// let nextest = Nextest::new(cargo, Lockfile::Writable);
///
/// for workspace in &roots {
///     let observation = nextest.observe(workspace, &root).await?;
///
///     println!("{} tests ran", observation.ran());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Nextest {
    /// The cargo that mise installed for the project
    cargo: Cargo,

    /// What a run does with the lockfile of the workspace
    lockfile: Lockfile,
}

impl Nextest {
    /// Creates the nextest that the given cargo runs
    ///
    /// The lockfile tells a run whether cargo may resolve the dependencies
    /// of the build, or whether it builds the versions that the lockfile
    /// holds and refuses every other one.
    pub fn new(cargo: Cargo, lockfile: Lockfile) -> Self {
        Self { cargo, lockfile }
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
    /// Returns [`CargoUnavailable`][unavailable] when cargo does not run,
    /// and the error of the reading when the reports leave the run without
    /// an answer.
    ///
    /// [unavailable]: ObserveNextestError::CargoUnavailable
    // nextest[impl run.consent]
    // nextest[impl run.operation+2]
    pub async fn observe(
        &self,
        root: &CargoRoot,
        project: &ProjectRoot,
    ) -> Result<Observation, ObserveNextestError> {
        let invocation = self
            .cargo
            .invocation(root)
            .env(CONSENT_VARIABLE, CONSENT_VALUE)
            .args(NEXTEST);

        // nextest[impl run.lockfile]
        let invocation = match self.lockfile {
            Lockfile::Writable => invocation,
            Lockfile::Locked => invocation.arg(LOCKED),
        };

        let execution = invocation
            .run()
            .await
            .map_err(|source| ObserveNextestError::CargoUnavailable { source })?;

        Observation::read(&execution, root, project)
    }
}
