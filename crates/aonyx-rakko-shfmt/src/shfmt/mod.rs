//! The shfmt that a project runs
//!
//! This module holds the version of shfmt that the crate declares, the program
//! that mise installed at that version, and the run of one operation. An
//! action states which operation it wants, and everything between the action
//! and the process lives here.

mod error;

use rakko::{Action, Context, ProjectRoot, ToolOutput, Tools};
use rakko_tool::{Invocation, Launch, ResolveToolError, ShownStreams, Tool};

pub use self::error::ObserveShfmtError;
use crate::observation::{Exit, Observation};
use crate::operation::Operation;

/// The name that mise knows the tool by, which is also the name of its
/// program
const SHFMT: &str = "shfmt";

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
/// An action that runs shfmt adds [`TOOLS`][tools] to its tools, so that the
/// machinery can read the version, and a run can resolve shfmt.
///
/// # Examples
///
/// ```no_run
/// use rakko::{Context, ProjectRoot};
/// use aonyx_rakko_shfmt::{Operation, Shfmt};
/// use rakko_test_utils::path;
///
/// # async fn run(action: &impl rakko::Action) -> Result<(), Box<dyn std::error::Error>> {
/// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
/// let shfmt = Shfmt::resolve()
///     .context(&context)
///     .action(action)
///     .call()
///     .await?;
///
/// let observation = shfmt.observe(Operation::CheckFormat).await?;
///
/// assert!(observation.files().is_empty());
/// # Ok(())
/// # }
/// ```
///
/// [tools]: Shfmt::TOOLS
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

#[bon::bon]
impl Shfmt {
    /// The tools that the mise file of the crate pins, which is shfmt alone
    ///
    /// An action that runs shfmt through this crate returns the constant from
    /// [`Action::additional_tools`]. The crate keeps the pin, so
    /// every action that uses it runs the version that the tests of the crate
    /// cover.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{Action, Context, Documented, Name, Outcome, Tools, action_name};
    /// use aonyx_rakko_shfmt::Shfmt;
    ///
    /// /// Formats the shell scripts of the project
    /// #[derive(Documented)]
    /// struct FormatScripts;
    ///
    /// impl Action for FormatScripts {
    ///     type Args = ();
    ///
    ///     fn name(&self) -> Name {
    ///         action_name!("format-scripts")
    ///     }
    ///
    ///     fn additional_tools(&self) -> Vec<Tools> {
    ///         vec![Shfmt::TOOLS]
    ///     }
    ///
    ///     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
    ///         Outcome::Passed { summary: None }
    ///     }
    /// }
    ///
    /// assert_eq!(FormatScripts.tools()[0].tool().get(), "shfmt");
    /// ```
    pub const TOOLS: Tools = rakko::bundled_tools!();

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
    /// # async fn run(action: &impl rakko::Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
    /// let shfmt = Shfmt::resolve()
    ///     .context(&context)
    ///     .action(action)
    ///     .call()
    ///     .await?;
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

    /// Returns the shfmt that mise installed at the version that an action
    /// declares
    ///
    /// The lookup reads the tools that the action declares, which hold
    /// [`TOOLS`][tools]. It asks mise for shfmt at the version that they give,
    /// so a pin of shfmt in the project does not change the answer. A run of shfmt starts in the root of the context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, ProjectRoot};
    /// use aonyx_rakko_shfmt::Shfmt;
    /// use rakko_test_utils::path;
    ///
    /// # async fn run(action: &impl rakko::Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
    /// let shfmt = Shfmt::resolve().context(&context).action(action).call().await?;
    ///
    /// assert_eq!(shfmt, Shfmt::resolve().context(&context).action(action).call().await?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when the action does not declare shfmt,
    /// or when mise does not run. It also returns one when mise reports no shfmt
    /// at that version, for example because nothing installed it.
    ///
    /// [tools]: Shfmt::TOOLS
    #[builder]
    pub async fn resolve<A: Action>(
        context: &Context,
        action: &A,
    ) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve()
            .action(action)
            .tool(SHFMT)
            .executable(SHFMT)
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

    use rakko::{Documented, Name, Outcome, action_name};

    use super::*;

    /// An action that declares no tool
    #[derive(Documented)]
    struct Bare;

    impl Action for Bare {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("bare")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

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

    #[tokio::test]
    async fn resolve_for_an_action_without_shfmt_reports_it_undeclared() {
        let context = Context::builder()
            .root(ProjectRoot::new(std::env::temp_dir()))
            .build();

        let result = Shfmt::resolve()
            .context(&context)
            .action(&Bare)
            .call()
            .await;

        assert!(
            matches!(result, Err(ResolveToolError::UndeclaredTool { .. })),
            "expected shfmt to be undeclared, got {result:?}"
        );
    }

    /// Shows that the lookup of `rakko-tool` for a requirement with a program
    /// refuses the requirement of a pin
    ///
    /// This crate holds a mise file, so its pins are pins of a real file. The
    /// test lives here, because `rakko-tool` holds no mise file of its own.
    #[tokio::test]
    async fn resolve_requirement_of_a_pin_reports_it_without_a_program() {
        let context = Context::builder()
            .root(ProjectRoot::new(std::env::temp_dir()))
            .build();

        let result = Tool::resolve_requirement()
            .requirement(&Shfmt::TOOLS.requirements()[0])
            .context(&context)
            .call()
            .await;

        assert!(
            matches!(
                result,
                Err(rakko_tool::ResolveRequirementError::UnstatedExecutable { .. })
            ),
            "expected a requirement without a program, got {result:?}"
        );
    }

    #[test]
    fn shfmt_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Shfmt>();
    }

    #[test]
    fn tools_declare_shfmt_alone() {
        let tools = Shfmt::TOOLS.requirements();

        let names: Vec<&str> = tools.iter().map(|tool| tool.tool().get()).collect();

        assert_eq!(names, ["shfmt"]);
    }
}
