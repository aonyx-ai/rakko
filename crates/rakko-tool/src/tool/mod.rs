//! The external program that an action runs
//!
//! An action gives the name of the tool that it wraps, or declares a
//! requirement of it, and this module turns that into a program that a run
//! can start. The tool holds what a run needs: the
//! name that the action asked for, the location that mise reported, and the
//! root of the project that the tool works on.
//!
//! Nothing starts a program until the caller runs a command. A tool describes
//! the command, the action adds its arguments, and the run reports what the
//! tool produced.

/// What the crate reports when it cannot find a tool by its name
mod error;
/// The identifier of an external tool
mod name;
/// What the crate reports when it cannot resolve the requirement of a tool
mod requirement_error;
/// The version of a toolchain that a project pins
mod toolchain;

use getset::Getters;
use kawauso_process::invocation::Program;
use kawauso_process::{Execution, Invocation};
use rakko::{Context, ProjectRoot, RequiredVersion, ToolRequirement, ToolVersion};

pub use self::error::ResolveToolError;
pub use self::name::ToolName;
pub use self::requirement_error::ResolveRequirementError;
use self::toolchain::pinned_toolchain;

/// The program that reports where a tool is
///
/// The operating system finds it with the rules of the platform. The
/// canonical way to start a harness enters the environment of mise first, so
/// a run that reaches an action reaches mise as well.
const MISE: &str = "mise";

/// The subcommand of mise that reports where a tool is
const WHICH: &str = "which";

/// The subcommand of mise that reports the versions that a project pins
const CURRENT: &str = "current";

/// The flag that asks mise about one version of a tool, whatever the
/// project pins
const TOOL: &str = "--tool";

/// The details of a failure that mise did not explain
const NO_DIAGNOSIS: &str = "mise wrote nothing about it";

/// The details of a run of mise that ended with success and named no location
const NO_LOCATION: &str = "mise named no location";

/// The extensions that Windows starts a program by
///
/// The list holds the extensions of `PATHEXT` that carry a program of their
/// own, in the order that Windows reads them. It leaves out the ones that
/// name a script for an interpreter, such as `.ps1`, because starting one of
/// those is a setting of a machine and not a property of the project.
#[cfg(windows)]
const EXECUTABLE_EXTENSIONS: [&str; 4] = ["com", "exe", "bat", "cmd"];

/// An external program that an action runs
///
/// A tool is a program that mise installed. [`resolve_requirement`][declared]
/// asks mise for the version that an action declares, and
/// [`resolve`][resolve] asks mise for the version that the project pins.
/// [`invocation`][invocation] describes the command that runs the tool.
///
/// The value carries the root of the project, because a tool behaves the same
/// from every directory of a project only if every run of it starts in the
/// same one. An action therefore resolves a tool once and runs it as often as
/// it needs.
///
/// [declared]: Tool::resolve_requirement
/// [invocation]: Tool::invocation
/// [resolve]: Tool::resolve
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Tool {
    /// The name that mise knows the tool by
    name: ToolName,

    /// The location of the program that mise installed
    ///
    /// The path is absolute, so a command that starts the program searches no
    /// path of its own and reaches the same program from every directory.
    #[getset(get = "pub")]
    program: Program,

    /// The root of the project that the tool works on
    root: ProjectRoot,

    /// The version of the tool that a requirement resolved to
    ///
    /// A tool that [`resolve`][resolve] found carries no version, because
    /// the pin of the project chose it and the lookup did not read the pin.
    ///
    /// [resolve]: Tool::resolve
    #[getset(get = "pub")]
    version: Option<ToolVersion>,
}

impl Tool {
    /// Returns the command that runs the tool
    ///
    /// The command starts the program that [`resolve`][resolve] found, in the
    /// root of the project, and it carries no argument yet. The caller adds
    /// the arguments of the operation that it wants, and then runs the
    /// command with [`launch`][launch], which survives a launcher that
    /// Windows refused for a moment.
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
    /// use rakko_tool::{Launch, Tool, ToolName};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let tool = Tool::resolve(ToolName::new("taplo"), "/home/otter/project".into()).await?;
    ///
    /// let execution = tool.invocation().arg("check").launch().await?;
    ///
    /// println!("{}", execution.stdout());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [launch]: crate::Launch::launch
    /// [resolve]: Tool::resolve
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

        let named = program(&execution).ok_or_else(|| ResolveToolError::UnresolvedTool {
            tool: name.clone(),
            details: details(&execution),
        })?;

        let program = executable(named);

        Ok(Self {
            name,
            program,
            root,
            version: None,
        })
    }

    /// Returns the tool that a requirement accepts, at the location that mise
    /// reports
    ///
    /// For an exact version, the lookup asks mise for the program of the tool
    /// at that version, wherever mise installed it. The version that the
    /// project pins does not change the answer, so a run starts the version
    /// that the tests of the action cover.
    ///
    /// For a range, the tool is a toolchain, and the project pins its version.
    /// The lookup asks mise which versions the configuration of the project
    /// pins. It takes the first of them, in the order of the pins, that the
    /// range accepts. Then it asks mise for the program at that version. A
    /// range compares the versions of releases, such as `>=1.85`, or the dates
    /// of one channel, such as `>=nightly-2025-02-20`, which reads only the
    /// pins of that channel that the configuration writes with a date, such
    /// as `nightly-2026-11-02`. A pin in the global configuration of the
    /// user does not count, and neither does a pin in a directory above the
    /// project. These files are different on each machine. A local mise file
    /// inside the project, such as `mise.local.toml`, counts. So a developer
    /// can override the pin of the project on their own machine.
    ///
    /// The answer is an absolute path, and a command of the tool starts in
    /// the root of the context. The lookup starts up to three processes, so an
    /// action resolves each tool once and keeps the result for the length of
    /// the run.
    ///
    /// # Errors
    ///
    /// Returns [`MiseUnavailable`][unavailable] when mise does not run, and
    /// [`UnresolvedVersion`][unresolved] when mise reports no program of the
    /// tool at the version, for example because nothing installed that
    /// version. Rakko installs nothing, so a missing version is a failure and
    /// not a step that this method takes.
    ///
    /// For a toolchain, returns [`InvalidRange`][invalid] when the range is
    /// not a range, [`UnresolvedToolchain`][toolchain] when no configuration
    /// inside the project pins the toolchain, or no dated version of the
    /// channel of a range of dates, and [`UnrecognizedReport`][unrecognized]
    /// when mise lists the pins in a form that the crate does not read. When
    /// the range accepts no pin, returns [`VersionOutOfRange`][outside] for
    /// the first pin with three numbers or a date to compare, or
    /// [`UnreadableVersion`][unreadable] when a range of releases finds no
    /// pin with three numbers.
    ///
    /// Returns [`UnknownRequirement`][unknown] for a kind of version that this
    /// release does not know.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion};
    /// use rakko_tool::Tool;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let requirement = ToolRequirement::builder()
    ///     .tool("taplo")
    ///     .executable("taplo")
    ///     .version(RequiredVersion::Exact(ToolVersion::new("0.10.0")))
    ///     .build();
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let tool = Tool::resolve_requirement(&requirement, &context).await?;
    ///
    /// println!("{}", tool.program());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [invalid]: ResolveRequirementError::InvalidRange
    /// [outside]: ResolveRequirementError::VersionOutOfRange
    /// [toolchain]: ResolveRequirementError::UnresolvedToolchain
    /// [unavailable]: ResolveRequirementError::MiseUnavailable
    /// [unknown]: ResolveRequirementError::UnknownRequirement
    /// [unreadable]: ResolveRequirementError::UnreadableVersion
    /// [unrecognized]: ResolveRequirementError::UnrecognizedReport
    /// [unresolved]: ResolveRequirementError::UnresolvedVersion
    pub async fn resolve_requirement(
        requirement: &ToolRequirement,
        context: &Context,
    ) -> Result<Self, ResolveRequirementError> {
        let tool = requirement.tool();
        let root = context.root();

        let version = match requirement.version() {
            RequiredVersion::Exact(version) => version.clone(),
            RequiredVersion::Range(range) => pinned_toolchain(tool, range, root).await?,
            _ => {
                return Err(ResolveRequirementError::UnknownRequirement { tool: tool.clone() });
            }
        };

        let bin = requirement.executable();

        let execution = Invocation::new(MISE)
            .arg(WHICH)
            .arg(bin.get())
            .arg(TOOL)
            .arg(format!("{tool}@{version}"))
            .in_directory(root.get())
            .run()
            .await
            .map_err(|source| ResolveRequirementError::MiseUnavailable {
                tool: tool.clone(),
                source,
            })?;

        let named =
            program(&execution).ok_or_else(|| ResolveRequirementError::UnresolvedVersion {
                tool: tool.clone(),
                executable: bin.clone(),
                version: version.clone(),
                details: details(&execution),
            })?;

        Ok(Self {
            name: ToolName::new(bin.get()),
            program: executable(named),
            root: root.clone(),
            version: Some(version),
        })
    }
}

/// Returns what mise wrote to its standard error stream, or a sentence that
/// says that it wrote nothing
fn diagnosis(execution: &Execution) -> String {
    let diagnosis = execution.stderr().to_string_lossy();
    let text = diagnosis.trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
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

    diagnosis(execution)
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

/// Returns the program that the platform starts for what mise named
///
/// Every platform but Windows starts the file that mise named, and the answer
/// is the one that arrived.
#[cfg(not(windows))]
fn executable(program: Program) -> Program {
    program
}

/// Returns the program that the platform starts for what mise named
///
/// Windows decides what it can start from the extension of a file. A package
/// manager for Node writes a script without an extension and a launcher with
/// one, side by side under the same name, and mise names the script, which
/// Windows refuses to start. The lookup therefore adds the extensions of the
/// platform to the name and answers with the first file that exists.
///
/// A file that already carries an extension is the answer, and so is one that
/// no launcher sits beside, so that a tool which mise resolved and this
/// platform cannot start fails where a run starts it, with the location that
/// mise reported in the message.
#[cfg(windows)]
fn executable(program: Program) -> Program {
    let path = program.get();

    if path.extension().is_some() {
        return program;
    }

    EXECUTABLE_EXTENSIONS
        .iter()
        .map(|extension| path.with_extension(extension))
        .find(|candidate| candidate.is_file())
        .map_or(program, Program::from)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;
    use std::process::Command;

    use kawauso_process::invocation::WorkingDirectory;
    use rakko::VersionRange;
    use tempfile::TempDir;

    use super::*;
    use crate::pin::pinned_version;

    /// The tool that this repository pins and that the tests resolve
    ///
    /// The tests need a tool that mise installs for this project, and one
    /// that reads an argument and writes it back, so that a run can show what
    /// reached the tool.
    const PINNED: &str = "jq";

    /// A tool of this repository that a package manager for Node installed
    ///
    /// Such a package writes a script without an extension and a launcher
    /// with one under the same name, and mise names the script. The name is
    /// therefore the one that shows whether resolution answers with a program
    /// that the platform can start.
    const SCRIPTED: &str = "prettier";

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

    /// Returns the context of a run in the root of this repository
    fn context() -> Context {
        Context::builder().root(root()).build()
    }

    /// Returns the context of a run in a directory
    fn context_of(directory: &Path) -> Context {
        Context::builder().root(directory).build()
    }

    /// Returns a requirement of the tool that this repository pins
    fn requirement(version: RequiredVersion) -> ToolRequirement {
        ToolRequirement::builder()
            .tool(PINNED)
            .executable(PINNED)
            .version(version)
            .build()
    }

    /// Returns the requirement of the exact version that this repository
    /// pins
    fn exact() -> ToolRequirement {
        let version = ToolVersion::new(pinned_by_the_repository());

        requirement(RequiredVersion::Exact(version))
    }

    /// Returns the version of the tool that this repository pins
    ///
    /// The version comes from the `mise.toml` of the repository, so that an
    /// update of the pin reaches the tests without a change to them.
    fn pinned_by_the_repository() -> String {
        let pins = std::fs::read_to_string(root().get().join("mise.toml"))
            .expect("the test reads the mise.toml of the repository");

        pinned_version(&pins, PINNED).to_owned()
    }

    /// Returns whether a tool starts the program of a version
    ///
    /// Mise installs each version in a directory that carries the version in
    /// its name, so the path of the program shows the version.
    fn runs_version(tool: &Tool, version: &str) -> bool {
        tool.program().get().to_string_lossy().contains(version)
    }

    /// Returns the requirement of a toolchain with the given range
    fn toolchain(range: &str) -> ToolRequirement {
        requirement(RequiredVersion::Range(VersionRange::new(range)))
    }

    /// A project in a temporary directory whose mise file pins the tool
    struct Project {
        /// The directory that holds the project
        directory: TempDir,
    }

    impl Project {
        /// Creates a project that pins the tool at each version, in order
        ///
        /// Mise ignores a configuration that nobody trusts, so the file is
        /// trusted right away.
        fn pinning(versions: &[&str]) -> Self {
            let directory = tempfile::tempdir().expect("the test creates a temporary directory");
            let pins = directory.path().join("mise.toml");
            let quoted: Vec<String> = versions
                .iter()
                .map(|version| format!("\"{version}\""))
                .collect();

            std::fs::write(
                &pins,
                format!("[tools]\n{PINNED} = [{}]\n", quoted.join(", ")),
            )
            .expect("the test writes the mise.toml of the project");

            let trusted = Command::new("mise")
                .arg("trust")
                .arg("--quiet")
                .arg(&pins)
                .status()
                .expect("the test runs mise");
            assert!(trusted.success(), "expected mise to trust the project");

            Self { directory }
        }

        /// Returns the context of a run in the project
        fn context(&self) -> Context {
            context_of(self.directory.path())
        }

        /// Returns the context of a run in a project in a directory of this
        /// one, which pins nothing itself
        fn context_below(&self) -> Context {
            let below = self.directory.path().join("below");
            std::fs::create_dir(&below).expect("the test creates a directory in the project");

            context_of(&below)
        }
    }

    impl Drop for Project {
        /// Withdraws the trust that the project received
        ///
        /// Mise records the trust of a configuration in the state of the
        /// user, and the temporary directory never comes back, so the record
        /// would pile up there. A failure stays quiet, because the test
        /// already reported what it was about.
        fn drop(&mut self) {
            let _ = Command::new("mise")
                .arg("trust")
                .arg("--quiet")
                .arg("--untrust")
                .arg(self.directory.path().join("mise.toml"))
                .status();
        }
    }

    /// Returns the tool that this repository pins, resolved
    async fn pinned() -> Tool {
        Tool::resolve(ToolName::new(PINNED), root())
            .await
            .expect("expected mise to report the location of a tool that this project pins")
    }

    #[tokio::test]
    async fn invocation_runs_in_the_root_of_the_project() {
        let tool = pinned().await;

        let invocation = tool.invocation();

        assert_eq!(
            invocation.working_directory().map(WorkingDirectory::get),
            Some(root().get())
        );
    }

    #[tokio::test]
    async fn invocation_starts_the_program_that_resolution_reported() {
        let tool = pinned().await;

        let invocation = tool.invocation();

        assert_eq!(invocation.program(), tool.program());
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_range_of_dates_of_an_unpinned_channel_reports_it_unresolved()
     {
        let beta = ToolRequirement::builder()
            .tool("rust")
            .executable("cargo")
            .version(RequiredVersion::Range(VersionRange::new(
                ">=beta-2025-02-20",
            )))
            .build();

        let result = Tool::resolve_requirement(&beta, &context()).await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::UnresolvedToolchain { .. })
            ),
            "expected no dated pin of the channel, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_range_of_dates_reports_the_dated_pin_of_its_channel() {
        let nightly = ToolRequirement::builder()
            .tool("rust")
            .executable("cargo")
            .version(RequiredVersion::Range(VersionRange::new(
                ">=nightly-2025-02-20",
            )))
            .build();

        let result = Tool::resolve_requirement(&nightly, &context()).await;

        assert!(
            result.as_ref().is_ok_and(|tool| tool
                .version()
                .as_ref()
                .is_some_and(|version| version.get().starts_with("nightly-"))),
            "expected the dated nightly that this repository pins, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_range_of_dates_that_no_pin_reaches_stops() {
        let nightly = ToolRequirement::builder()
            .tool("rust")
            .executable("cargo")
            .version(RequiredVersion::Range(VersionRange::new(
                ">=nightly-2999-01-01",
            )))
            .build();

        let result = Tool::resolve_requirement(&nightly, &context()).await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::VersionOutOfRange { .. })
            ),
            "expected a nightly outside the range, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_an_installed_version_reports_the_version() {
        let tool = Tool::resolve_requirement(&exact(), &context())
            .await
            .expect("expected mise to report the version that this project pins");

        assert_eq!(
            tool.version().as_ref().map(ToolVersion::get),
            Some(pinned_by_the_repository().as_str())
        );
    }

    #[tokio::test]
    async fn resolve_with_a_pinned_tool_reports_an_installed_program() {
        let tool = pinned().await;

        assert!(tool.program().get().is_file());
    }

    // A file that mise names for a package of Node is a script on every
    // platform, and Windows starts a program by the extension of its file, so
    // this run reaches the tool only when resolution answered with the
    // launcher beside the script.
    #[tokio::test]
    async fn resolve_with_a_tool_of_a_package_manager_reports_a_program_that_runs() {
        let tool = Tool::resolve(ToolName::new(SCRIPTED), root())
            .await
            .expect("expected mise to report the location of a tool that this project pins");

        let execution = tool.invocation().arg("--version").run().await;

        assert!(
            execution.is_ok(),
            "expected the tool to run, got {execution:?}"
        );
    }

    #[tokio::test]
    async fn resolve_with_an_unknown_tool_names_it() {
        let name = ToolName::new("rakko-pins-no-such-tool");

        let Err(error) = Tool::resolve(name, root()).await else {
            panic!("expected the lookup to report an error");
        };

        assert!(error.to_string().contains("rakko-pins-no-such-tool"));
    }

    #[tokio::test]
    async fn resolve_outside_the_project_reports_no_location() {
        let outside = tempfile::tempdir().expect("the test creates a temporary directory");

        let result = Tool::resolve(ToolName::new(PINNED), ProjectRoot::from(outside.path())).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn resolve_requirement_ignores_the_pin_of_the_project() {
        let project = Project::pinning(&["0.0.1"]);

        let tool = Tool::resolve_requirement(&exact(), &project.context()).await;

        assert!(tool.is_ok(), "expected the declared version, got {tool:?}");
    }

    #[tokio::test]
    async fn resolve_requirement_runs_the_tool_in_the_root_of_the_context() {
        let tool = Tool::resolve_requirement(&exact(), &context())
            .await
            .expect("expected mise to report the version that this project pins");

        let invocation = tool.invocation();

        assert_eq!(
            invocation.working_directory().map(WorkingDirectory::get),
            Some(root().get())
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_pin_that_is_no_version_reports_it() {
        let project = Project::pinning(&["ref:main"]);

        let result = Tool::resolve_requirement(&toolchain(">=1"), &project.context()).await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::UnreadableVersion { .. })
            ),
            "expected an unreadable version, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_toolchain_in_its_range_reports_the_pinned_version() {
        let pinned = pinned_by_the_repository();

        let result = Tool::resolve_requirement(&toolchain(">=1"), &context()).await;

        assert!(
            result
                .as_ref()
                .is_ok_and(|tool| runs_version(tool, &pinned)),
            "expected the program of the pinned version, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_toolchain_outside_its_range_stops() {
        let result = Tool::resolve_requirement(&toolchain("<1"), &context()).await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::VersionOutOfRange { .. })
            ),
            "expected a version outside the range, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_toolchain_pinned_above_the_project_reports_it_unresolved() {
        let above = Project::pinning(&[&pinned_by_the_repository()]);

        let result = Tool::resolve_requirement(&toolchain(">=1"), &above.context_below()).await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::UnresolvedToolchain { .. })
            ),
            "expected a pin outside the project to count for nothing, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_toolchain_pinned_three_times_reports_the_first_pin() {
        let pinned = pinned_by_the_repository();
        let project = Project::pinning(&[&pinned, "99.0.0", "0.0.1"]);

        let result = Tool::resolve_requirement(&toolchain(">=0"), &project.context()).await;

        assert!(
            result
                .as_ref()
                .is_ok_and(|tool| runs_version(tool, &pinned)),
            "expected the program of the first pin, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_toolchain_pinned_twice_reports_the_first_pin_in_its_range()
    {
        let pinned = pinned_by_the_repository();
        let project = Project::pinning(&["99.0.0", &pinned]);

        let result = Tool::resolve_requirement(&toolchain("<99"), &project.context()).await;

        assert!(
            result
                .as_ref()
                .is_ok_and(|tool| runs_version(tool, &pinned)),
            "expected the program of the pin in the range, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_toolchain_that_mise_does_not_know_reports_it_unresolved() {
        let unknown = ToolRequirement::builder()
            .tool("rakko-pins-no-such-tool")
            .executable("rakko-pins-no-such-tool")
            .version(RequiredVersion::Range(VersionRange::new(">=1")))
            .build();

        let result = Tool::resolve_requirement(&unknown, &context()).await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::UnresolvedToolchain { .. })
            ),
            "expected an unresolved toolchain, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_version_that_is_not_installed_carries_what_mise_said() {
        let missing = requirement(RequiredVersion::Exact(ToolVersion::new("0.0.1")));

        let Err(error) = Tool::resolve_requirement(&missing, &context()).await else {
            panic!("expected the lookup to report an error");
        };

        assert!(
            error.to_string().contains("is not installed"),
            "expected the diagnosis of mise, got {error}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_version_that_is_not_installed_gives_the_tool_and_the_version()
     {
        let missing = requirement(RequiredVersion::Exact(ToolVersion::new("0.0.1")));

        let Err(error) = Tool::resolve_requirement(&missing, &context()).await else {
            panic!("expected the lookup to report an error");
        };

        assert!(
            error.to_string().contains("`jq` 0.0.1"),
            "expected the tool and the version, got {error}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_an_installed_version_reports_its_program() {
        let tool = Tool::resolve_requirement(&exact(), &context())
            .await
            .expect("expected mise to report the version that this project pins");

        assert!(tool.program().get().is_file());
    }

    #[tokio::test]
    async fn resolve_requirement_with_an_invalid_range_reports_it() {
        let result = Tool::resolve_requirement(&toolchain("not a range"), &context()).await;

        assert!(
            matches!(result, Err(ResolveRequirementError::InvalidRange { .. })),
            "expected an invalid range, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_an_unpinned_toolchain_reports_it() {
        let outside = tempfile::tempdir().expect("the test creates a temporary directory");
        let unpinned = ToolRequirement::builder()
            .tool("npm:rakko-pins-no-such-package")
            .executable("rakko-pins-no-such-package")
            .version(RequiredVersion::Range(VersionRange::new(">=1")))
            .build();

        let result = Tool::resolve_requirement(&unpinned, &context_of(outside.path())).await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::UnresolvedToolchain { .. })
            ),
            "expected an unpinned toolchain, got {result:?}"
        );
    }

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
