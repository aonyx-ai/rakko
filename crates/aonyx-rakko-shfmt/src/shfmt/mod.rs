//! The shfmt that a project runs
//!
//! This module holds the version of shfmt that the crate declares, the program
//! that mise installed at that version, and the run of one operation. An
//! action states which operation it wants, and everything between the action
//! and the process lives here.

mod error;

use rakko::{Context, ProjectRoot, RequiredVersion, ToolOutput, ToolRequirement, ToolVersion};
use rakko_tool::{Invocation, Launch, MiseFile, ResolveRequirementError, ShownStreams, Tool};

pub use self::error::ObserveShfmtError;
use crate::observation::{Exit, Observation};
use crate::operation::Operation;

/// The name that mise knows the tool by, which is also the name of its
/// program
const SHFMT: &str = "shfmt";

/// The version of shfmt that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(SHFMT);

/// The flag that asks shfmt to name the files that it would rewrite
const LIST: &str = "--list";

/// The flag that asks shfmt to rewrite the files that it formats
const WRITE: &str = "--write";

/// The flag that asks shfmt to name the shell scripts that it finds
const FIND: &str = "--find";

/// The directory that every operation walks, which is the project root
///
/// A run starts in the root. Shfmt writes each path that it reports relative
/// to the directory that it walks, so the paths are relative to the root.
const ROOT: &str = ".";

/// The shfmt that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it, so a run
/// reaches the version that the tests of the crate cover. Nothing here
/// installs a tool: provisioning is the job of mise, and a shfmt that mise
/// does not report stops the caller.
///
/// Shfmt takes its options from the `.editorconfig` files of the project when
/// the command line gives none. The command lines of this crate give none. So
/// a run formats in the same way as an editor, and as a contributor that runs
/// shfmt on the project with no flag.
///
/// An action that runs shfmt declares [`requirement`][requirement] among its
/// tools, so that the machinery can read the version.
///
/// # Examples
///
/// ```no_run
/// use rakko::{Context, ProjectRoot};
/// use aonyx_rakko_shfmt::{Operation, Shfmt};
/// use rakko_test_utils::path;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
/// let shfmt = Shfmt::resolve(&context).await?;
///
/// let observation = shfmt.observe(Operation::CheckFormat).await?;
///
/// assert!(observation.files().is_empty());
/// # Ok(())
/// # }
/// ```
///
/// [requirement]: Shfmt::requirement
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Shfmt {
    /// The root of the project that shfmt runs in
    root: ProjectRoot,

    /// The program that mise installed for the project
    tool: Tool,

    /// The tool output of the context that resolved shfmt
    ///
    /// Each run sends what shfmt writes to it.
    output: ToolOutput,
}

impl Shfmt {
    /// Runs one operation and reads what shfmt reported
    ///
    /// The run sends what shfmt writes to the tool output of the context that
    /// resolved the shfmt. Every operation walks the whole project from its
    /// root.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, ProjectRoot};
    /// use aonyx_rakko_shfmt::{Operation, Shfmt};
    /// use rakko_test_utils::path;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
    /// let shfmt = Shfmt::resolve(&context).await?;
    ///
    /// let observation = shfmt.observe(Operation::Find).await?;
    ///
    /// assert!(observation.diagnostics().is_empty());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ShfmtUnavailable`][unavailable] when shfmt does not run, and
    /// [`UnrecognizedReport`][unrecognized] when the crate cannot read what it
    /// wrote.
    ///
    /// [unavailable]: ObserveShfmtError::ShfmtUnavailable
    /// [unrecognized]: ObserveShfmtError::UnrecognizedReport
    pub async fn observe(&self, operation: Operation) -> Result<Observation, ObserveShfmtError> {
        let execution = self
            .invocation(operation)
            .launch_with(&self.output, ShownStreams::Both)
            .await
            .map_err(|source| ObserveShfmtError::ShfmtUnavailable { source })?;

        let exit = if execution.status().success() {
            Exit::Succeeded
        } else {
            Exit::Failed
        };

        Observation::read()
            .exit(exit)
            .stdout(&execution.stdout().to_string_lossy())
            .stderr(&execution.stderr().to_string_lossy())
            .root(self.root.get())
            .call()
    }

    /// Returns the requirement of the shfmt that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins. An action
    /// that runs shfmt returns it from its declaration of tools.
    ///
    /// # Examples
    ///
    /// ```
    /// use aonyx_rakko_shfmt::Shfmt;
    ///
    /// let requirement = Shfmt::requirement();
    ///
    /// assert_eq!(requirement.executable().get(), "shfmt");
    /// ```
    pub fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(SHFMT)
            .executable(SHFMT)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the shfmt that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of shfmt in the project does not change the answer.
    /// A run of shfmt starts in the root of the context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, ProjectRoot};
    /// use aonyx_rakko_shfmt::Shfmt;
    /// use rakko_test_utils::path;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
    /// let shfmt = Shfmt::resolve(&context).await?;
    ///
    /// assert_eq!(shfmt, Shfmt::resolve(&context).await?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run, or when
    /// it reports no shfmt at that version, for example because nothing
    /// installed it.
    ///
    /// [requirement]: Shfmt::requirement
    pub async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement()
            .requirement(&Self::requirement())
            .context(context)
            .call()
            .await?;

        Ok(Self {
            root: context.root().clone(),
            tool,
            output: context.tool_output().clone(),
        })
    }

    /// Returns the command that runs one operation
    fn invocation(&self, operation: Operation) -> Invocation {
        self.tool
            .invocation()
            .args(arguments(operation).iter().copied())
            .arg(ROOT)
    }
}

/// Returns the flags of one operation of shfmt
fn arguments(operation: Operation) -> &'static [&'static str] {
    match operation {
        Operation::CheckFormat => &[LIST],
        Operation::Find => &[FIND],
        Operation::Format => &[LIST, WRITE],
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
    fn arguments_of_a_format_check_list_and_rewrite_nothing() {
        let selected = arguments(Operation::CheckFormat);

        assert_eq!(selected, ["--list"]);
    }

    #[test]
    fn arguments_of_a_format_rewrite_and_list_the_files() {
        let selected = arguments(Operation::Format);

        assert_eq!(selected, ["--list", "--write"]);
    }

    #[test]
    fn arguments_of_a_search_find_the_scripts() {
        let selected = arguments(Operation::Find);

        assert_eq!(selected, ["--find"]);
    }

    #[test]
    fn shfmt_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Shfmt>();
    }
}
