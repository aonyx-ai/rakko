//! The nextest that a project runs
//!
//! This module holds the cargo that carries nextest, the command line of a
//! run, and the consent that the experimental report of nextest needs. A
//! caller names the workspace that it wants tested, and everything between
//! the caller and the process lives here.

/// The error that leaves a run without an answer
mod error;

use rakko_action::ProjectRoot;
use rakko_cargo::{Cargo, CargoRoot};

pub use self::error::ObserveNextestError;
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
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_cargo::Cargo;
/// use rakko_nextest::Nextest;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
/// let cargo = Cargo::resolve(root.clone()).await?;
/// let roots = cargo.roots().await?;
/// let nextest = Nextest::new(cargo);
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
}

impl Nextest {
    /// Creates the nextest that the given cargo runs
    pub fn new(cargo: Cargo) -> Self {
        Self { cargo }
    }

    /// Tests one workspace of the project and reads what the run reported
    ///
    /// The run starts in the directory of the root, and it names the files
    /// of its findings relative to the project root that the caller gives.
    /// Nothing of the project changes, whatever the run finds.
    ///
    /// # Errors
    ///
    /// Returns [`CargoUnavailable`][unavailable] when cargo does not run,
    /// and the error of the reading when the reports leave the run without
    /// an answer.
    ///
    /// [unavailable]: ObserveNextestError::CargoUnavailable
    // nextest[impl run.consent]
    // nextest[impl run.operation]
    pub async fn observe(
        &self,
        root: &CargoRoot,
        project: &ProjectRoot,
    ) -> Result<Observation, ObserveNextestError> {
        let execution = self
            .cargo
            .invocation(root)
            .env(CONSENT_VARIABLE, CONSENT_VALUE)
            .args(NEXTEST)
            .run()
            .await
            .map_err(|source| ObserveNextestError::CargoUnavailable { source })?;

        Observation::read(&execution, root, project)
    }
}
