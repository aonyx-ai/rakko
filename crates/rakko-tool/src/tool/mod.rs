//! The external program that an action runs
//!
//! An action names the tool that it wraps, and this module turns that name
//! into a program that a run can start. The tool holds what a run needs: the
//! name that the action asked for, the location that mise reported, and the
//! root of the project that the tool works on.
//!
//! Nothing starts a program until the caller runs a command. A tool describes
//! the command, the action adds its arguments, and the run reports what the
//! tool produced.

/// What the crate reports when it cannot find a tool
mod error;
/// The identifier of an external tool
mod name;

use getset::Getters;
use kawauso_process::invocation::Program;
use kawauso_process::{Execution, Invocation};
use rakko_action::ProjectRoot;

pub use self::error::ResolveToolError;
pub use self::name::ToolName;

/// The program that reports where a tool is
///
/// The operating system finds it with the rules of the platform. The
/// canonical way to start a harness enters the environment of mise first, so
/// a run that reaches an action reaches mise as well.
const MISE: &str = "mise";

/// The subcommand of mise that reports where a tool is
const WHICH: &str = "which";

/// The details of a failure that mise did not explain
const NO_DIAGNOSIS: &str = "mise wrote nothing about it";

/// The details of a run of mise that ended with success and named no location
const NO_LOCATION: &str = "mise named no location";

/// An external program that an action runs
///
/// A tool is a program that mise installed for a project, at the version that
/// the project pinned. [`resolve`][resolve] asks mise for one, and
/// [`invocation`][invocation] describes the command that runs it.
///
/// The value carries the root of the project, because a tool behaves the same
/// from every directory of a project only if every run of it starts in the
/// same one. An action therefore resolves a tool once and runs it as often as
/// it needs.
///
/// [invocation]: Tool::invocation
/// [resolve]: Tool::resolve
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Tool {
    /// The name that mise knows the tool by
    #[getset(get = "pub")]
    name: ToolName,

    /// The location of the program that mise installed
    ///
    /// The path is absolute, so a command that starts the program searches no
    /// path of its own and reaches the same program from every directory.
    #[getset(get = "pub")]
    program: Program,

    /// The root of the project that the tool works on
    root: ProjectRoot,
}

impl Tool {
    /// Returns the command that runs the tool
    ///
    /// The command starts the program that [`resolve`][resolve] found, in the
    /// root of the project, and it carries no argument yet. The caller adds
    /// the arguments of the operation that it wants, and then runs the
    /// command.
    ///
    /// No shell reads the command. Nothing splits an argument at a space,
    /// removes a quotation mark, or expands a character such as `*`, so the
    /// tool receives every argument as the caller wrote it.
    ///
    /// A command that ends without success is no failure of the run. The exit
    /// status travels in the result, because a tool that reports a problem of
    /// the project ends without success, and that is the answer that the
    /// caller asked for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_tool::{Tool, ToolName};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let tool = Tool::resolve(ToolName::new("taplo"), "/home/otter/project".into()).await?;
    ///
    /// let execution = tool.invocation().arg("check").run().await?;
    ///
    /// println!("{}", execution.stdout());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [resolve]: Tool::resolve
    // tool[impl run.program]
    // tool[impl run.root]
    // tool[impl run.vector]
    // tool[impl run.capture]
    pub fn invocation(&self) -> Invocation {
        Invocation::new(self.program.clone()).in_directory(self.root.get())
    }

    /// Returns the tool of a project, at the location that mise reports
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path. The answer is an absolute
    /// path, and a run of the tool therefore searches nothing.
    ///
    /// The lookup starts a process, so a caller that resolves several tools
    /// pays for each of them. An action resolves the tool that it wraps once
    /// and keeps the result for the length of the run.
    ///
    /// # Errors
    ///
    /// Returns [`MiseUnavailable`][unavailable] when mise does not run, and
    /// [`UnresolvedTool`][unresolved] when mise runs and reports no location
    /// for the tool. Rakko installs nothing, so a tool that no one installed
    /// is a failure and not a step that this method takes.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_tool::{Tool, ToolName};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let tool = Tool::resolve(ToolName::new("zizmor"), "/home/otter/project".into()).await?;
    ///
    /// println!("{}", tool.program());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [unavailable]: ResolveToolError::MiseUnavailable
    /// [unresolved]: ResolveToolError::UnresolvedTool
    // tool[impl resolve]
    // tool[impl resolve.root]
    // tool[impl resolve.missing]
    pub async fn resolve(name: ToolName, root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let execution = Invocation::new(MISE)
            .arg(WHICH)
            .arg(name.get())
            .in_directory(root.get())
            .run()
            .await
            .map_err(|source| ResolveToolError::MiseUnavailable {
                tool: name.clone(),
                source,
            })?;

        let program = program(&execution).ok_or_else(|| ResolveToolError::UnresolvedTool {
            tool: name.clone(),
            details: details(&execution),
        })?;

        Ok(Self {
            name,
            program,
            root,
        })
    }
}

/// Returns what mise said about a tool that it reported no location for
///
/// Mise writes its diagnosis to the standard error stream, and that diagnosis
/// names the step that installs a tool which nothing installed yet. The text
/// travels into the message of the error, so whoever reads the failure reads
/// the answer of mise instead of a sentence that Rakko wrote about it.
///
/// A run that ended with success and named no location leaves that stream
/// empty, and the details then state what the crate observed.
fn details(execution: &Execution) -> String {
    if execution.status().success() {
        return NO_LOCATION.to_owned();
    }

    let diagnosis = execution.stderr().to_string_lossy();
    let text = diagnosis.trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
    }
}

/// Returns the program that mise reported, if it reported one
///
/// Mise ends without success when it knows no tool by the name. When it knows
/// one, it writes the path of the program and ends the line, so the first
/// line that holds something is the answer. The end of a line carries two
/// characters on Windows, and the trim removes both, so no invisible
/// character reaches a command.
///
/// An unsuccessful run and an answer without a path mean the same thing to an
/// action, so both answer `None` here, and the caller turns them into one
/// error.
fn program(execution: &Execution) -> Option<Program> {
    if !execution.status().success() {
        return None;
    }

    let output = execution.stdout().to_string_lossy();

    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(Program::from)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use kawauso_process::invocation::WorkingDirectory;

    use super::*;

    /// The tool that this repository pins and that the tests resolve
    ///
    /// The tests need a tool that mise installs for this project, and one
    /// that reads an argument and writes it back, so that a run can show what
    /// reached the tool.
    const PINNED: &str = "jq";

    /// Returns the root of the project that the tests run against
    ///
    /// The tests resolve the tools that this repository pins, so the project
    /// is the repository itself. The crate lives two directories below its
    /// root.
    fn root() -> ProjectRoot {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("the crate lives two directories below the root of the repository");

        ProjectRoot::from(root)
    }

    /// Returns the tool that this repository pins, resolved
    async fn pinned() -> Tool {
        Tool::resolve(ToolName::new(PINNED), root())
            .await
            .expect("expected mise to report the location of a tool that this project pins")
    }

    // tool[verify run.root]
    #[tokio::test]
    async fn invocation_runs_in_the_root_of_the_project() {
        let tool = pinned().await;

        let invocation = tool.invocation();

        assert_eq!(
            invocation.working_directory().map(WorkingDirectory::get),
            Some(root().get())
        );
    }

    // tool[verify run.program]
    #[tokio::test]
    async fn invocation_starts_the_program_that_resolution_reported() {
        let tool = pinned().await;

        let invocation = tool.invocation();

        assert_eq!(invocation.program(), tool.program());
    }

    // tool[verify resolve]
    #[tokio::test]
    async fn resolve_with_a_pinned_tool_reports_an_installed_program() {
        let tool = pinned().await;

        assert!(tool.program().get().is_file());
    }

    // tool[verify resolve.missing]
    #[tokio::test]
    async fn resolve_with_an_unknown_tool_names_it() {
        let name = ToolName::new("rakko-pins-no-such-tool");

        let Err(error) = Tool::resolve(name, root()).await else {
            panic!("expected the lookup to report an error");
        };

        assert!(error.to_string().contains("rakko-pins-no-such-tool"));
    }

    // tool[verify resolve.root]
    #[tokio::test]
    async fn resolve_outside_the_project_reports_no_location() {
        let outside = tempfile::tempdir().expect("the test creates a temporary directory");

        let result = Tool::resolve(ToolName::new(PINNED), ProjectRoot::from(outside.path())).await;

        assert!(result.is_err());
    }

    // tool[verify run.capture]
    #[tokio::test]
    async fn run_reports_a_tool_that_ended_without_success() {
        let tool = pinned().await;

        let execution = tool
            .invocation()
            .arg("--null-input")
            .arg("error(\"the tool stops here\")")
            .run()
            .await
            .expect("expected the tool to run");

        assert!(!execution.status().success());
    }

    // tool[verify run.capture]
    #[tokio::test]
    async fn run_reports_what_the_tool_wrote_to_its_standard_error() {
        let tool = pinned().await;

        let execution = tool
            .invocation()
            .arg("--null-input")
            .arg("error(\"the tool stops here\")")
            .run()
            .await
            .expect("expected the tool to run");

        assert!(
            execution
                .stderr()
                .to_string_lossy()
                .contains("the tool stops here")
        );
    }

    // tool[verify run.capture]
    // tool[verify run.vector]
    #[tokio::test]
    async fn run_reports_what_the_tool_wrote_to_its_standard_output() {
        let tool = pinned().await;

        let execution = tool
            .invocation()
            .arg("--null-input")
            .arg("--raw-output")
            .arg("--arg")
            .arg("value")
            .arg("two words *")
            .arg("$value")
            .run()
            .await
            .expect("expected the tool to run");

        assert_eq!(execution.stdout().to_string_lossy().trim(), "two words *");
    }

    // A scheduler runs actions in parallel and can move a run to a different
    // thread, so a tool that an action holds across a wait travels with it.
    // This test holds the tool to the auto traits that make this possible,
    // because a field of a later version could take them away without a word
    // from the compiler.
    #[test]
    fn tool_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<Tool>();
    }
}
