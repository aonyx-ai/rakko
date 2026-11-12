//! The taplo that a project runs
//!
//! This module holds the version of taplo that the crate declares, the
//! program that mise installed at that version, and the run of one
//! operation. An action states which operation it wants, and
//! everything between the action and the process lives here.

mod error;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rakko::{Context, ProjectRoot, RequiredVersion, ToolRequirement, ToolVersion};
use rakko_tool::{Execution, Invocation, Launch, MiseFile, ResolveRequirementError, Tool};

pub use self::error::ObserveTaploError;
use crate::observation::Observation;
use crate::operation::Operation;

/// The number of times one operation starts before the crate gives up
///
/// Taplo can lose the end of its report when it exits, and a fresh run almost
/// always answers. So a few attempts separate a lost line from a report that
/// has genuinely ended.
const ATTEMPTS: u32 = 6;

/// The number of times one operation starts to get the count of the files
///
/// The count is the one part of a report that no other stream carries. A run
/// that has nothing left to do after it counts loses it most often. A caller
/// reports a pass without the count, so the count gets a small budget of its
/// own instead of the whole one.
const COUNTED_ATTEMPTS: u32 = 2;

/// The pause that grows between the attempts of one operation
///
/// The loss happens while taplo exits. A machine under load takes longer over
/// it. So attempts that follow each other immediately tend to lose together. A
/// growing pause carries the later attempts past the moment of load.
const BACKOFF: Duration = Duration::from_millis(25);

/// The name that mise knows the tool by, which is also the name of its
/// program
const TAPLO: &str = "taplo";

/// The version of taplo that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(TAPLO);

/// The variable that selects which lines of its log taplo writes
///
/// The reading needs lines at the level of information. One of them reports
/// the configuration file that taplo found, and another counts the files. A
/// filter that the environment of the run sets, such as `warn`, drops them. Like the flag for colors, the filter selects the presentation of the
/// report and not the behavior of the tool. A run without the variable writes
/// the same report.
const LOG_FILTER: &str = "RUST_LOG";

/// The filter that keeps every line of the log that the reading needs
const INFO: &str = "info";

/// The names of the configuration files that taplo looks for
const CONFIGURATION_FILES: [&str; 2] = [".taplo.toml", "taplo.toml"];

/// The subcommand of taplo that formats TOML files
const FORMAT: &str = "fmt";

/// The subcommand of taplo that validates TOML files
const LINT: &str = "lint";

/// The flag that asks the formatter to report instead of rewriting
const CHECK: &str = "--check";

/// The flag that asks the formatter to name its files on standard output
///
/// The flag makes taplo print the difference that it would write for each file
/// that is not formatted. The crate reads the header of each of those
/// differences and not the text below it. It does so because the header names
/// the file, and the stream that carries it does not lose lines.
const DIFF: &str = "--diff";

/// The flag that selects how taplo colors its report
const COLORS: &str = "--colors";

/// The value that asks taplo for a report without color codes
///
/// The report is read as data, and a color code inside a path would corrupt the
/// reading. The flag selects the presentation of the report and not the
/// behavior of the tool. What taplo does to a project comes from the
/// configuration of that project alone.
const PLAIN: &str = "never";

/// The taplo that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it, so a run
/// reaches the version that the tests of the crate cover. Nothing here
/// installs a tool: provisioning is the job of mise, and a taplo that mise
/// does not report stops the caller.
///
/// An action that runs taplo declares [`requirement`][requirement] among its
/// tools, so that the machinery can read the version.
///
/// # Examples
///
/// ```no_run
/// use rakko::{Context, ProjectRoot};
/// use rakko_taplo::{Operation, Taplo};
/// use rakko_test_utils::path;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
/// let taplo = Taplo::resolve(&context).await?;
///
/// let observation = taplo.observe(Operation::CheckFormat).await?;
///
/// assert!(observation.problems().is_empty());
/// # Ok(())
/// # }
/// ```
///
/// [requirement]: Taplo::requirement
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Taplo {
    /// The root of the project that taplo runs in
    ///
    /// A run looks for the configuration file of the project from here, the
    /// way that taplo does.
    root: ProjectRoot,

    /// The program that mise installed for the project
    tool: Tool,
}

impl Taplo {
    /// Runs one operation and reads what taplo reported
    ///
    /// Taplo can lose the end of its report when it exits. The answer of a run
    /// survives that loss. The exit status carries whether taplo found
    /// anything. A formatting run names the files that it would rewrite on its
    /// standard output stream. A file that taplo read and could not accept gets
    /// a diagnostic that the same loss never reaches. The reading of a failed
    /// validating run reads the files of the run itself to find the ones that
    /// taplo could not read.
    ///
    /// A report that holds no answer at all lost the lines that carried one.
    /// Such a run starts again, a few times, before the crate gives up.
    ///
    /// Taplo warns about a configuration file that it rejected, and then runs
    /// with its defaults and can end with success. The warning comes only on
    /// the stream that can lose lines. So when the project holds a
    /// configuration file, a report must say that taplo found a configuration
    /// file. A report that does not say so cannot show whether taplo rejected
    /// the file, and the run starts again in the same way. The crate looks for
    /// the file in the places where taplo looks. That is a `.taplo.toml` or a
    /// `taplo.toml` in the root of the project or in a directory above it.
    ///
    /// The count of the files is the one part that no other stream carries. An
    /// operation starts again to get it, with a budget of its own, and answers
    /// without the count instead of failing over it. That is because a run that
    /// ended with success found nothing, whatever its report lost.
    ///
    /// Repeating is safe for every operation, because a report reads the
    /// project again, and a rewrite formats files that a previous attempt
    /// already formatted.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, ProjectRoot};
    /// use rakko_taplo::{Operation, Taplo};
    /// use rakko_test_utils::path;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
    /// let taplo = Taplo::resolve(&context).await?;
    /// let observation = taplo.observe(Operation::CheckFormat).await?;
    ///
    /// assert!(observation.problems().is_empty());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`TaploUnavailable`][unavailable] when taplo does not run. When
    /// no attempt answered, it returns
    /// [`UnreportedConfiguration`][unreported] if the last attempt did not
    /// report the configuration file of the project, and
    /// [`IncompleteReport`][incomplete] otherwise.
    ///
    /// [incomplete]: ObserveTaploError::IncompleteReport
    /// [unavailable]: ObserveTaploError::TaploUnavailable
    /// [unreported]: ObserveTaploError::UnreportedConfiguration
    pub async fn observe(&self, operation: Operation) -> Result<Observation, ObserveTaploError> {
        let configuration = configuration_file(self.root.get()).await;
        let mut unanswered = ObserveTaploError::IncompleteReport {
            stderr: String::new(),
        };
        let mut answered: Option<Observation> = None;

        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(BACKOFF * attempt).await;
            }

            let observation = Observation::read()
                .execution(&self.start(operation).await?)
                .operation(operation)
                .call()
                .await;

            if let Some(error) = observation.unanswered(configuration.as_deref()) {
                unanswered = error;
                continue;
            }

            if observation.checked().is_some() || attempt + 1 >= COUNTED_ATTEMPTS {
                return Ok(observation);
            }

            answered.get_or_insert(observation);
        }

        answered.ok_or(unanswered)
    }

    /// Returns the requirement of the taplo that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins. An action
    /// that runs taplo returns it from its declaration of tools.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_taplo::Taplo;
    ///
    /// let requirement = Taplo::requirement();
    ///
    /// assert_eq!(requirement.executable().get(), "taplo");
    /// ```
    pub fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(TAPLO)
            .executable(TAPLO)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the taplo that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of taplo in the project does not change the answer.
    /// A run of taplo starts in the root of the context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, ProjectRoot};
    /// use rakko_taplo::Taplo;
    /// use rakko_test_utils::path;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
    /// let taplo = Taplo::resolve(&context).await?;
    ///
    /// assert_eq!(taplo, Taplo::resolve(&context).await?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run, or when
    /// it reports no taplo at that version, for example because nothing
    /// installed it.
    ///
    /// [requirement]: Taplo::requirement
    pub async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement()
            .requirement(&Self::requirement())
            .context(context)
            .call()
            .await?;

        Ok(Self {
            root: context.root().clone(),
            tool,
        })
    }

    /// Starts one operation and collects what it produced
    ///
    /// # Errors
    ///
    /// Returns [`TaploUnavailable`][unavailable] when taplo does not run.
    ///
    /// [unavailable]: ObserveTaploError::TaploUnavailable
    async fn start(&self, operation: Operation) -> Result<Execution, ObserveTaploError> {
        self.invocation(operation)
            .launch()
            .await
            .map_err(|source| ObserveTaploError::TaploUnavailable { source })
    }

    /// Returns the command that runs one operation
    fn invocation(&self, operation: Operation) -> Invocation {
        self.tool
            .invocation()
            .args(arguments(operation).iter().copied())
            .arg(COLORS)
            .arg(PLAIN)
            .env(LOG_FILTER, INFO)
    }
}

/// Returns a configuration file that taplo would find when it starts in a
/// directory
///
/// Taplo looks for a file of either name in the directory where it starts,
/// and then in each directory above it. An entry of either name that exists
/// counts, also one that taplo cannot read, because taplo reports such an
/// entry before it reads it. A symbolic link counts when its target exists,
/// so this look counts a broken link as no file. Taplo reports a broken link
/// as a configuration file that it found, and its run then fails on its own.
/// Where one directory holds both names, taplo can pick the other one. The
/// path of the answer then differs, and the report must still hold the line
/// about a configuration file.
///
/// Taplo also reads a file that the variable `TAPLO_CONFIG` gives. This look
/// does not. So when a project holds no file of either name, a report does not
/// have to hold that line, whatever the variable says.
async fn configuration_file(directory: &Path) -> Option<PathBuf> {
    for ancestor in directory.ancestors() {
        for name in CONFIGURATION_FILES {
            let path = ancestor.join(name);

            if tokio::fs::metadata(&path).await.is_ok() {
                return Some(path);
            }
        }
    }

    None
}

/// Returns the command line of one operation of taplo
fn arguments(operation: Operation) -> &'static [&'static str] {
    match operation {
        Operation::CheckFormat => &[FORMAT, CHECK, DIFF],
        Operation::Format => &[FORMAT],
        Operation::Lint => &[LINT],
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Returns a directory that holds an entry with the given name
    fn directory_with(name: &str) -> tempfile::TempDir {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        std::fs::write(directory.path().join(name), "").expect("the test writes a file");

        directory
    }

    #[test]
    fn arguments_of_a_format_check_report_instead_of_rewriting() {
        let selected = arguments(Operation::CheckFormat);

        assert_eq!(selected, ["fmt", "--check", "--diff"]);
    }

    #[test]
    fn arguments_of_a_format_rewrite_the_project() {
        let selected = arguments(Operation::Format);

        assert_eq!(selected, ["fmt"]);
    }

    #[test]
    fn arguments_of_a_lint_validate_the_project() {
        let selected = arguments(Operation::Lint);

        assert_eq!(selected, ["lint"]);
    }

    #[tokio::test]
    async fn configuration_file_above_the_directory_finds_the_file() {
        let directory = directory_with(".taplo.toml");
        let nested = directory.path().join("crates").join("otter");
        std::fs::create_dir_all(&nested).expect("the test creates a directory");

        let found = configuration_file(&nested).await;

        assert_eq!(found, Some(directory.path().join(".taplo.toml")));
    }

    #[tokio::test]
    async fn configuration_file_in_the_directory_finds_the_file() {
        let directory = directory_with(".taplo.toml");

        let found = configuration_file(directory.path()).await;

        assert_eq!(found, Some(directory.path().join(".taplo.toml")));
    }

    #[tokio::test]
    async fn configuration_file_of_a_directory_without_one_finds_nothing() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let found = configuration_file(directory.path()).await;

        assert_eq!(found, None);
    }

    #[tokio::test]
    async fn configuration_file_with_the_name_without_a_dot_finds_the_file() {
        let directory = directory_with("taplo.toml");

        let found = configuration_file(directory.path()).await;

        assert_eq!(found, Some(directory.path().join("taplo.toml")));
    }

    #[tokio::test]
    async fn invocation_asks_for_the_log_at_the_level_of_information() {
        let context = Context::builder()
            .root(ProjectRoot::new(PathBuf::from(env!("CARGO_MANIFEST_DIR"))))
            .build();
        let taplo = Taplo::resolve(&context)
            .await
            .expect("expected mise to report the taplo that the crate declares");

        let invocation = taplo.invocation(Operation::CheckFormat);

        assert!(
            invocation
                .environment()
                .iter()
                .any(|variable| variable.to_string() == "RUST_LOG=info"),
            "expected the log at the level of information, got {:?}",
            invocation.environment()
        );
    }

    #[test]
    fn taplo_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Taplo>();
    }
}
