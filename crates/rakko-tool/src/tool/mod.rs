//! The external program that an action runs
//!
//! An action declares a requirement of each tool that it runs, and this module
//! turns that requirement into a program that a run can start. The tool holds
//! what a run needs. That is the location that mise reported, the version that
//! the requirement resolved to, and the root of the project that the tool works
//! on.
//!
//! Nothing starts a program until the caller runs a command. A tool describes
//! the command, the action adds its arguments, and the run reports what the
//! tool produced.

mod requirement_error;
mod resolve_tool_error;
mod resolve_toolchain_error;
mod toolchain;

use std::collections::BTreeSet;
use std::path::PathBuf;

use bon::bon;
use getset::Getters;
use kawauso_process::error::RunCommandError;
use kawauso_process::invocation::Program;
use kawauso_process::{Execution, Invocation};
use rakko::{
    Action, Context, Executable, MiseTool, ProjectRoot, RequiredVersion, ToolRequirement,
    ToolVersion, ToolchainRequirement, VersionRange,
};

pub use self::requirement_error::ResolveRequirementError;
pub use self::resolve_tool_error::ResolveToolError;
pub use self::resolve_toolchain_error::ResolveToolchainError;
use self::toolchain::select;

/// The program that reports where a tool is
///
/// The operating system finds it with the rules of the platform. The
/// canonical way to start a harness enters the environment of mise first, so
/// a run that reaches an action reaches mise as well.
const MISE: &str = "mise";

/// The subcommand of mise that reports where a tool is
const WHICH: &str = "which";

/// The subcommand of mise that reports where it installed a version of a
/// tool
const WHERE: &str = "where";

/// The subcommand of mise that reports the versions that a project pins
const CURRENT: &str = "current";

/// The flag that asks mise about one version of a tool, whatever the
/// project pins
const TOOL: &str = "--tool";

/// The variable that holds the directories that a program searches for the
/// programs that it starts
///
/// A tool that runs on a toolchain, such as a package for Node, starts the
/// program of the toolchain by its name. So this variable decides which version
/// of the toolchain runs the tool.
const PATH: &str = "PATH";

/// The details of a failure that mise did not explain
const NO_DIAGNOSIS: &str = "mise wrote nothing about it";

/// The details of a run of mise that ended with success and named no location
const NO_LOCATION: &str = "mise named no location";

/// The extensions that Windows starts a program by
///
/// The list holds the extensions of `PATHEXT` that carry a program of their
/// own, in the order that Windows reads them. It leaves out the ones that name
/// a script for an interpreter, such as `.ps1`. It does so because starting one
/// of those is a setting of a machine and not a property of the project.
#[cfg(windows)]
const EXECUTABLE_EXTENSIONS: [&str; 4] = ["com", "exe", "bat", "cmd"];

/// An external program that an action runs
///
/// A tool is a program that mise installed. [`resolve_requirement`][declared]
/// asks mise for the version that an action declares, and
/// [`invocation`][invocation] describes the command that runs the tool.
///
/// The value carries the root of the project. That is because a tool behaves
/// the same from every directory of a project only if every run of it starts in
/// the same one. An action therefore resolves a tool once and runs it as often
/// as it needs.
///
/// # Examples
///
/// ```no_run
/// use rakko::{Context, ProjectRoot, RequiredVersion, ToolRequirement, ToolVersion};
/// use rakko_tool::Tool;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let requirement = ToolRequirement::builder()
///     .tool("zizmor")
///     .executable("zizmor")
///     .version(RequiredVersion::Exact(ToolVersion::new("1.32.0")))
///     .build();
/// let context = Context::builder().root(ProjectRoot::new(std::env::temp_dir())).build();
/// let tool = Tool::resolve_requirement().requirement(&requirement).context(&context).call().await?;
/// let execution = tool.invocation().arg("--version").run().await?;
///
/// assert!(execution.status().success());
/// # Ok(())
/// # }
/// ```
///
/// [declared]: Tool::resolve_requirement
/// [invocation]: Tool::invocation
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Tool {
    /// The location of the program that mise installed
    ///
    /// The path is absolute, so a command that starts the program searches no
    /// path of its own and reaches the same program from every directory.
    #[getset(get = "pub")]
    program: Program,

    /// The root of the project that the tool works on
    root: ProjectRoot,

    /// The version of the tool that the requirement resolved to
    ///
    /// For an exact version, the requirement gives it. For a range, it is the
    /// version that the project pins and that the range accepts.
    #[getset(get = "pub")]
    version: ToolVersion,

    /// The directories of the toolchains that the tool runs on, in the order
    /// that a run searches them
    ///
    /// A tool that received no toolchain from [`with_toolchain`][toolchain]
    /// holds none, and a run of it searches the path that the process
    /// inherited.
    ///
    /// [toolchain]: Tool::with_toolchain
    toolchains: Vec<PathBuf>,
}

#[bon]
impl Tool {
    /// Returns the command that runs the tool
    ///
    /// The command starts the program that
    /// [`resolve_requirement`][resolve] found, in the root of the project,
    /// and it carries no argument yet. The caller adds the arguments of the
    /// operation that it wants. Then it runs the command with
    /// [`launch_with`][launch-with] and the tool output of its context. The run
    /// sends what the tool writes to that tool output, and it survives a
    /// launcher that Windows refused for a moment.
    ///
    /// No shell reads the command. Nothing splits an argument at a space,
    /// removes a quotation mark, or expands a character such as `*`. So the
    /// tool receives every argument as the caller wrote it.
    ///
    /// A command that ends without success is no failure of the run. The exit
    /// status travels in the result. That is because a tool that reports a
    /// problem of the project ends without success, and that is the answer that
    /// the caller asked for.
    ///
    /// A tool that runs on a toolchain from [`with_toolchain`][toolchain]
    /// gets a command that puts the directory of that toolchain first on the
    /// path. The command then never fails to build. A toolchain directory
    /// that cannot be an entry of `PATH` is refused when the caller adds the
    /// toolchain. An entry of the inherited path holds no separator, so joining
    /// the directories cannot fail. If it did, the command would keep the
    /// inherited path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion};
    /// use rakko_tool::{Launch, ShownStreams, Tool};
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
    /// let tool = Tool::resolve_requirement().requirement(&requirement).context(&context).call().await?;
    ///
    /// let execution = tool
    ///     .invocation()
    ///     .arg("check")
    ///     .launch_with(context.tool_output(), ShownStreams::Both)
    ///     .await?;
    ///
    /// println!("{}", execution.stdout());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [launch-with]: crate::Launch::launch_with
    /// [resolve]: Tool::resolve_requirement
    /// [toolchain]: Tool::with_toolchain
    pub fn invocation(&self) -> Invocation {
        let invocation = Invocation::new(self.program.clone()).in_directory(self.root.get());

        if self.toolchains.is_empty() {
            return invocation;
        }

        let inherited = std::env::var_os(PATH).unwrap_or_default();
        let directories = self
            .toolchains
            .iter()
            .cloned()
            .chain(std::env::split_paths(&inherited));

        match std::env::join_paths(directories) {
            Ok(path) => invocation.env(PATH, path),
            Err(_) => invocation,
        }
    }

    /// Returns the tool that runs on the program of a toolchain
    ///
    /// Some tools start a toolchain by its name. For example, a package for
    /// Node starts the `node` program that it finds first on the path. The
    /// toolchain that the environment puts first can then be another version
    /// than the one that the action resolved. That toolchain can also be a
    /// version outside the range of the action. A command of the returned tool
    /// puts the directory of the program of the toolchain before the path that
    /// the process inherited. So the tool runs on the toolchain that the caller
    /// resolved, and the inherited path follows that directory.
    ///
    /// A tool can run on several toolchains. A run searches their
    /// directories in the order that the caller added the toolchains.
    ///
    /// # Errors
    ///
    /// Returns [`UnplaceableToolchain`][unplaceable] when the directory of the
    /// program of the toolchain cannot be an entry of `PATH`. That happens
    /// because the directory holds the separator of the entries, such as `:` on
    /// Unix. It also returns that error when the program has no directory. A
    /// run then stops. Else the tool runs on a toolchain that the environment
    /// chose.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion, VersionRange};
    /// use rakko_tool::{Launch, Tool};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    /// let node = ToolRequirement::builder()
    ///     .tool("node")
    ///     .executable("node")
    ///     .version(RequiredVersion::Range(VersionRange::new(">=22")))
    ///     .build();
    /// let prettier = ToolRequirement::builder()
    ///     .tool("npm:prettier")
    ///     .executable("prettier")
    ///     .version(RequiredVersion::Exact(ToolVersion::new("3.9.10")))
    ///     .build();
    ///
    /// let node = Tool::resolve_requirement().requirement(&node).context(&context).call().await?;
    /// let prettier = Tool::resolve_requirement()
    ///     .requirement(&prettier)
    ///     .context(&context)
    ///     .call()
    ///     .await?
    ///     .with_toolchain(&node)?;
    ///
    /// let execution = prettier.invocation().arg("--version").launch().await?;
    ///
    /// println!("{}", execution.stdout());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [unplaceable]: ResolveRequirementError::UnplaceableToolchain
    pub fn with_toolchain(mut self, toolchain: &Tool) -> Result<Self, ResolveRequirementError> {
        let program = toolchain.program();
        let directory = program
            .get()
            .parent()
            .filter(|directory| std::env::join_paths([directory]).is_ok())
            .ok_or_else(|| ResolveRequirementError::UnplaceableToolchain {
                program: program.clone(),
            })?;

        self.toolchains.push(directory.to_path_buf());

        Ok(self)
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
    /// The lookup asks mise which versions the project configuration pins.
    /// It takes the first one that the range accepts, in the order of the pins.
    /// Then it asks mise for the program at that version.
    /// A range compares release versions such as `>=1.85`, or dates of one
    /// channel such as `>=nightly-2025-02-20`.
    /// A range of dates reads only the pins of that channel that the
    /// configuration writes with a date, such as `nightly-2026-11-02`.
    /// A range can hold alternatives joined by `||`, such as `^22.18 || >=23.6`.
    /// It accepts a pin that one of them accepts.
    /// A pin in the global user configuration does not count.
    /// Neither does a pin in a directory above the project.
    /// These files differ on each machine.
    /// A local mise file inside the project, such as `mise.local.toml`,
    /// counts. So a developer can override the pin of the project on their own
    /// machine.
    ///
    /// The answer is an absolute path, and a command of the tool starts in
    /// the root of the context. The lookup starts up to three processes, so an
    /// action resolves each tool once and keeps the result for the length of
    /// the run.
    ///
    /// # Errors
    ///
    /// Returns [`MiseUnavailable`][unavailable] when mise does not run.
    /// Returns [`UnresolvedVersion`][unresolved] when mise reports no program
    /// of the tool at the version, for example because nothing installed that
    /// version.
    /// Rakko installs nothing, so a missing version is a failure and not a
    /// step this method takes.
    ///
    /// For a toolchain, returns [`InvalidRange`][invalid] when the range is
    /// not a range.
    /// Returns [`UnresolvedToolchain`][toolchain] when no configuration inside
    /// the project pins the toolchain, or no dated version of the channel of a
    /// range of dates.
    /// Returns [`UnrecognizedReport`][unrecognized] when mise lists pins in a
    /// form the crate does not read.
    /// When the range accepts no pin, returns [`VersionOutOfRange`][outside]
    /// for the first pin with three numbers or a date to compare.
    /// Returns [`UnreadableVersion`][unreadable] when a range of releases
    /// finds no pin with three numbers.
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
    /// let tool = Tool::resolve_requirement().requirement(&requirement).context(&context).call().await?;
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
    #[builder]
    pub async fn resolve_requirement(
        requirement: &ToolRequirement,
        context: &Context,
    ) -> Result<Self, ResolveRequirementError> {
        let tool = requirement.tool();
        let root = context.root();

        let version = match requirement.version() {
            RequiredVersion::Exact(version) => version.clone(),
            RequiredVersion::Range(range) => select(tool, std::slice::from_ref(range), root)
                .await
                .map_err(|unselected| unselected.requirement_error(tool))?,
            _ => {
                return Err(ResolveRequirementError::UnknownRequirement { tool: tool.clone() });
            }
        };

        let Some(bin) = requirement.executable() else {
            return Err(ResolveRequirementError::UnstatedExecutable { tool: tool.clone() });
        };

        let execution = which(tool, &version, bin, root).await.map_err(|source| {
            ResolveRequirementError::MiseUnavailable {
                tool: tool.clone(),
                source,
            }
        })?;

        let named =
            program(&execution).ok_or_else(|| ResolveRequirementError::UnresolvedVersion {
                tool: tool.clone(),
                executable: bin.clone(),
                version: version.clone(),
                details: details(&execution),
            })?;

        Ok(Self {
            program: executable(named),
            root: root.clone(),
            version,
            toolchains: Vec::new(),
        })
    }

    /// Returns a program of a tool that an action declares, at the location
    /// that mise reports
    ///
    /// The lookup reads the declarations of the action from
    /// [`Action::tools`], and it takes the exact version that they give
    /// for the tool. So a run starts only a tool that the action declares. A
    /// tool that the action declares only with a range, as a toolchain, is not
    /// such a tool, and [`resolve_requirement`][requirement] resolves it.
    ///
    /// Then the lookup asks mise for the program of the tool at that version,
    /// wherever mise installed it. The version that the project pins does not
    /// change the answer, and the lookup searches no `PATH` of its own. So a
    /// run starts the version that the tests of the action cover. One tool
    /// can provide several programs, such as `renovate-config-validator` of
    /// the tool `npm:renovate`, so the caller gives the program to start.
    ///
    /// The answer is an absolute path, and a command of the tool starts in
    /// the root of the context. The lookup starts up to two processes, so an
    /// action resolves each tool once and keeps the result for the length of
    /// the run.
    ///
    /// # Errors
    ///
    /// Returns [`UndeclaredTool`][undeclared] when the declarations hold no
    /// exact version of the tool, and [`AmbiguousVersion`][ambiguous] when
    /// they hold more than one. Returns [`MiseUnavailable`][unavailable] when
    /// mise does not run. Returns [`UninstalledVersion`][uninstalled] when
    /// mise did not install the tool at that version. Rakko installs nothing,
    /// so a missing version is a failure and not a step this method takes.
    /// Returns [`MissingExecutable`][missing] when the installed tool provides
    /// no program by that name.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Action, Context};
    /// use rakko_tool::Tool;
    ///
    /// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let validator = Tool::resolve()
    ///     .action(action)
    ///     .tool("npm:renovate")
    ///     .executable("renovate-config-validator")
    ///     .context(&context)
    ///     .call()
    ///     .await?;
    ///
    /// println!("{}", validator.program());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [ambiguous]: ResolveToolError::AmbiguousVersion
    /// [missing]: ResolveToolError::MissingExecutable
    /// [requirement]: Tool::resolve_requirement
    /// [unavailable]: ResolveToolError::MiseUnavailable
    /// [undeclared]: ResolveToolError::UndeclaredTool
    /// [uninstalled]: ResolveToolError::UninstalledVersion
    #[builder]
    pub async fn resolve<A: Action>(
        action: &A,
        #[builder(into)] tool: MiseTool,
        #[builder(into)] executable: Executable,
        context: &Context,
    ) -> Result<Self, ResolveToolError> {
        let version = declared_version(&action.tools(), &tool)?;
        let root = context.root();

        let unavailable = |source| ResolveToolError::MiseUnavailable {
            tool: tool.clone(),
            source,
        };

        let execution = which(&tool, &version, &executable, root)
            .await
            .map_err(unavailable)?;

        if let Some(named) = program(&execution) {
            return Ok(Self {
                program: self::executable(named),
                root: root.clone(),
                version,
                toolchains: Vec::new(),
            });
        }

        if installed(&tool, &version, root)
            .await
            .map_err(unavailable)?
        {
            return Err(ResolveToolError::MissingExecutable {
                tool,
                version,
                executable,
            });
        }

        Err(ResolveToolError::UninstalledVersion {
            details: details(&execution),
            tool,
            version,
        })
    }

    /// Returns a program of a toolchain that the project pins, at the
    /// location that mise reports
    ///
    /// A toolchain is a compiler or a runtime of the project, such as Rust or
    /// Node.js, and the project pins it. The lookup reads the requirements of
    /// the action from [`Action::toolchains`], and it keeps those of the
    /// toolchain. A run can add a range that only the run knows with
    /// `accepting`, such as the `rust-version` of a manifest. The lookup takes
    /// the first pin of the project that every one of these ranges accepts, in
    /// the order of the pins, as [`select_toolchain`][select] does. Without a
    /// range, it takes the first pin of the project. So an action that
    /// requires nothing still runs on the toolchain that the project pins,
    /// and the lookup never searches the `PATH` of the environment.
    ///
    /// The action writes no check of its own, because the lookup compares the
    /// pins with the requirements before anything starts. A run that skips
    /// because a tool has no build on its platform never reaches the lookup,
    /// so it checks no toolchain.
    ///
    /// Then the lookup asks mise for the program at that version. One
    /// toolchain can provide several programs, such as `cargo` and `rustc` of
    /// `rust`, so the caller gives the program to start. A tool that starts
    /// the toolchain by name runs on it with [`with_toolchain`][with].
    ///
    /// The answer is an absolute path, and a command of the toolchain starts
    /// in the root of the context. The lookup starts up to four processes, so
    /// an action resolves each toolchain once and keeps the result for the
    /// length of the run.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`select_toolchain`][select] when the project
    /// pins no version that the ranges accept. Returns
    /// [`UninstalledToolchain`][uninstalled] when mise did not install the
    /// version that the project pins. Rakko installs nothing, so an install of
    /// the pins of the project repairs it, and the pins stay. Returns
    /// [`MissingExecutable`][missing] when the installed toolchain provides no
    /// program by that name, and [`MiseUnavailable`][unavailable] when mise
    /// does not run.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Action, Context};
    /// use rakko_tool::Tool;
    ///
    /// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let node = Tool::resolve_toolchain()
    ///     .action(action)
    ///     .toolchain("node")
    ///     .executable("node")
    ///     .context(&context)
    ///     .call()
    ///     .await?;
    ///
    /// println!("{} {}", node.program(), node.version());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [missing]: ResolveToolchainError::MissingExecutable
    /// [select]: Tool::select_toolchain
    /// [unavailable]: ResolveToolchainError::MiseUnavailable
    /// [uninstalled]: ResolveToolchainError::UninstalledToolchain
    /// [with]: Tool::with_toolchain
    #[builder]
    pub async fn resolve_toolchain<A: Action>(
        action: &A,
        #[builder(into)] toolchain: MiseTool,
        #[builder(into)] executable: Executable,
        accepting: Option<VersionRange>,
        context: &Context,
    ) -> Result<Self, ResolveToolchainError> {
        let mut requirements = action.toolchains();
        requirements.extend(accepting.map(|range| {
            ToolchainRequirement::builder()
                .toolchain(toolchain.clone())
                .versions(range)
                .build()
        }));

        let version = Self::select_toolchain()
            .toolchain(&toolchain)
            .requirements(&requirements)
            .context(context)
            .call()
            .await?;
        let root = context.root();

        let unavailable = |source| ResolveToolchainError::MiseUnavailable {
            toolchain: toolchain.clone(),
            source,
        };

        let execution = which(&toolchain, &version, &executable, root)
            .await
            .map_err(unavailable)?;

        if let Some(named) = program(&execution) {
            return Ok(Self {
                program: self::executable(named),
                root: root.clone(),
                version,
                toolchains: Vec::new(),
            });
        }

        if installed(&toolchain, &version, root)
            .await
            .map_err(unavailable)?
        {
            return Err(ResolveToolchainError::MissingExecutable {
                toolchain,
                version,
                executable,
            });
        }

        Err(ResolveToolchainError::UninstalledToolchain {
            details: details(&execution),
            toolchain,
            version,
        })
    }

    /// Returns the version of a toolchain that the project pins and that
    /// every requirement of the toolchain accepts
    ///
    /// The lookup reads only the requirements of the toolchain, so a caller
    /// can give every requirement of an action. It asks mise which versions
    /// the configuration of the project pins. It takes the first one that
    /// every range accepts, in the order of the pins. Without a requirement,
    /// it takes the first pin. A range compares release versions such as
    /// `>=1.85`, or dates of one channel such as `>=nightly-2025-02-20`. A
    /// range of dates reads only the pins of that channel that the
    /// configuration writes with a date, such as `nightly-2026-11-02`. The
    /// range `nightly-*` accepts every such pin. A range can hold
    /// alternatives joined by `||`, such as `^22.18 || >=23.6`.
    ///
    /// A pin in the global user configuration does not count. Neither does a
    /// pin in a directory above the project, because these files differ on
    /// each machine. A local mise file inside the project, such as
    /// `mise.local.toml`, counts. So a developer can override the pin of the
    /// project on their own machine.
    ///
    /// The lookup does not ask whether mise installed the version, because
    /// the answer is about the pins. `doctor` reads it to report a requirement
    /// that the pins of the project do not meet.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRange`][invalid] when a requirement is not a range,
    /// and [`MiseUnavailable`][unavailable] when mise does not run. Returns
    /// [`UnpinnedToolchain`][unpinned] when no configuration inside the
    /// project pins the toolchain, or no dated version of the channel of a
    /// range of dates. When a requirement accepts no pin, returns
    /// [`UnmetRequirement`][unmet]. The error gives the first pin with three
    /// numbers or a date to compare, and the requirement that refused it.
    /// Returns [`UnreadableVersion`][unreadable] when a range of releases
    /// finds no pin with three numbers. Returns
    /// [`UnrecognizedReport`][unrecognized] when mise lists the pins in a form
    /// that the crate does not read.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, MiseTool, ToolchainRequirement, VersionRange};
    /// use rakko_tool::Tool;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    /// let requirement = ToolchainRequirement::builder().toolchain("node").versions(VersionRange::new(">=22.18")).build();
    ///
    /// let version = Tool::select_toolchain()
    ///     .toolchain(&MiseTool::new("node"))
    ///     .requirements(&[requirement])
    ///     .context(&context)
    ///     .call()
    ///     .await?;
    ///
    /// println!("{version}");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [invalid]: ResolveToolchainError::InvalidRange
    /// [unavailable]: ResolveToolchainError::MiseUnavailable
    /// [unmet]: ResolveToolchainError::UnmetRequirement
    /// [unpinned]: ResolveToolchainError::UnpinnedToolchain
    /// [unreadable]: ResolveToolchainError::UnreadableVersion
    /// [unrecognized]: ResolveToolchainError::UnrecognizedReport
    #[builder]
    pub async fn select_toolchain(
        toolchain: &MiseTool,
        requirements: &[ToolchainRequirement],
        context: &Context,
    ) -> Result<ToolVersion, ResolveToolchainError> {
        let ranges: Vec<VersionRange> = requirements
            .iter()
            .filter(|requirement| requirement.toolchain() == toolchain)
            .map(|requirement| requirement.versions().clone())
            .collect();

        select(toolchain, &ranges, context.root())
            .await
            .map_err(|unselected| unselected.toolchain_error(toolchain))
    }
}

/// Returns whether mise installed a tool at a version
///
/// # Errors
///
/// Returns the error of a mise that did not run.
async fn installed(
    tool: &MiseTool,
    version: &ToolVersion,
    root: &ProjectRoot,
) -> Result<bool, RunCommandError> {
    let execution = Invocation::new(MISE)
        .arg(WHERE)
        .arg(format!("{tool}@{version}"))
        .in_directory(root.get())
        .run()
        .await?;

    Ok(execution.status().success())
}

/// Returns the exact version at which the declarations of an action give a
/// tool
///
/// # Errors
///
/// Returns [`UndeclaredTool`][undeclared] when the declarations give no exact
/// version of the tool, and [`AmbiguousVersion`][ambiguous] when they give
/// more than one.
///
/// [ambiguous]: ResolveToolError::AmbiguousVersion
/// [undeclared]: ResolveToolError::UndeclaredTool
fn declared_version(
    tools: &[ToolRequirement],
    tool: &MiseTool,
) -> Result<ToolVersion, ResolveToolError> {
    let versions: BTreeSet<&ToolVersion> = tools
        .iter()
        .filter(|requirement| requirement.tool() == tool)
        .filter_map(|requirement| match requirement.version() {
            RequiredVersion::Exact(version) => Some(version),
            _ => None,
        })
        .collect();

    let mut declared = versions.iter();
    match (declared.next(), declared.next()) {
        (Some(version), None) => Ok((*version).clone()),
        (None, _) => Err(ResolveToolError::UndeclaredTool { tool: tool.clone() }),
        (Some(_), Some(_)) => Err(ResolveToolError::AmbiguousVersion {
            tool: tool.clone(),
            versions: versions.into_iter().cloned().collect(),
        }),
    }
}

/// Asks mise for the location of a program of a tool at a version
///
/// The question names the version, so the version that the project pins does
/// not change the answer.
///
/// # Errors
///
/// Returns the error of a mise that did not run.
async fn which(
    tool: &MiseTool,
    version: &ToolVersion,
    executable: &Executable,
    root: &ProjectRoot,
) -> Result<Execution, RunCommandError> {
    Invocation::new(MISE)
        .arg(WHICH)
        .arg(executable.get())
        .arg(TOOL)
        .arg(format!("{tool}@{version}"))
        .in_directory(root.get())
        .run()
        .await
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
/// travels into the message of the error. So whoever reads the failure reads
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
/// action. So both answer `None` here, and the caller turns them into one
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
/// one, side by side under the same name. Mise names the script, which Windows
/// refuses to start. The lookup therefore adds the extensions of the platform
/// to the name and answers with the first file that exists.
///
/// A file that already carries an extension is the answer, and so is one that
/// no launcher sits beside. So a tool which mise resolved and this platform
/// cannot start fails where a run starts it. The message then names the
/// location that mise reported.
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
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::Path;
    use std::process::Command;

    use kawauso_process::invocation::WorkingDirectory;
    use rakko::{Documented, Name, Outcome, ToolchainRequirement, VersionRange, action_name};
    use tempfile::TempDir;

    use super::*;
    use crate::pin::MiseFile;

    /// The tool that this repository pins and that the tests resolve
    ///
    /// The tests need a tool that mise installs for this project. The tool
    /// reads an argument and writes it back, so a run can show what reached the
    /// tool.
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

        MiseFile::new(&pins).pinned_version(PINNED).to_owned()
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
        /// Mise records configuration trust in the user state.
        /// The temporary directory never returns, so the record would pile up.
        /// A failure stays quiet, because the test already reported what it was about.
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
        Tool::resolve_requirement()
            .requirement(&exact())
            .context(&context())
            .call()
            .await
            .expect("expected mise to report the location of a tool that this project pins")
    }

    /// Returns the requirement of the tool of a package manager for Node
    ///
    /// The version comes from the mise file of the crate that wraps the tool.
    /// So an update of the pin reaches the test without a change to it.
    fn scripted() -> ToolRequirement {
        let pins = std::fs::read_to_string(
            root()
                .get()
                .join("crates/aonyx-rakko-prettier/mise.rakko.toml"),
        )
        .expect("the test reads the mise file of the crate that wraps the tool");
        let tool = format!("npm:{SCRIPTED}");
        let version = ToolVersion::new(MiseFile::new(&pins).pinned_version(&tool));

        ToolRequirement::builder()
            .tool(tool)
            .executable(SCRIPTED)
            .version(RequiredVersion::Exact(version))
            .build()
    }

    /// Returns the requirement of the tool of a package with two programs
    ///
    /// The package `renovate` provides `renovate` and
    /// `renovate-config-validator`. The version comes from the mise file of
    /// the action that runs the tool. So an update of the pin reaches the test
    /// without a change to it.
    fn renovate() -> ToolRequirement {
        let pins = std::fs::read_to_string(
            root()
                .get()
                .join("actions/aonyx-rakko-check-renovate-config/mise.rakko.toml"),
        )
        .expect("the test reads the mise file of the action that runs the tool");
        let version = ToolVersion::new(MiseFile::new(&pins).pinned_version(RENOVATE));

        ToolRequirement::builder()
            .tool(RENOVATE)
            .executable("renovate")
            .version(RequiredVersion::Exact(version))
            .build()
    }

    /// The tool of the package with two programs
    const RENOVATE: &str = "npm:renovate";

    /// An action that declares the requirements that it holds
    #[derive(Documented)]
    struct Declaring(Vec<ToolRequirement>);

    impl Action for Declaring {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("declaring")
        }

        fn tools(&self) -> Vec<ToolRequirement> {
            self.0.clone()
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// Resolves a program of a tool for an action that declares the
    /// requirements, in the root of this repository
    ///
    /// # Errors
    ///
    /// Returns the error of the lookup, which the test asserts on.
    #[bon::builder]
    async fn resolved(
        tools: &[ToolRequirement],
        tool: &str,
        executable: &str,
    ) -> Result<Tool, ResolveToolError> {
        Tool::resolve()
            .action(&Declaring(tools.to_vec()))
            .tool(tool)
            .executable(executable)
            .context(&context())
            .call()
            .await
    }

    /// An action that requires the toolchain in the ranges that it holds,
    /// and that resolves the toolchain when it runs
    #[derive(Documented)]
    struct Requiring {
        /// The requirements that the action declares
        requirements: Vec<ToolchainRequirement>,

        /// The tools that the action declares
        tools: Vec<ToolRequirement>,

        /// Whether a run of the action asked for the toolchain
        resolved: std::sync::atomic::AtomicBool,
    }

    impl Requiring {
        /// Creates an action that requires the toolchain in each range
        fn new(ranges: &[&str]) -> Self {
            Self {
                requirements: ranges
                    .iter()
                    .map(|range| {
                        ToolchainRequirement::builder()
                            .toolchain(PINNED)
                            .versions(VersionRange::new(*range))
                            .build()
                    })
                    .collect(),
                tools: Vec::new(),
                resolved: std::sync::atomic::AtomicBool::new(false),
            }
        }

        /// Returns whether a run of the action asked for the toolchain
        fn resolved(&self) -> bool {
            self.resolved.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    impl Action for Requiring {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("requiring")
        }

        fn tools(&self) -> Vec<ToolRequirement> {
            self.tools.clone()
        }

        fn toolchains(&self) -> Vec<ToolchainRequirement> {
            self.requirements.clone()
        }

        async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
            self.resolved
                .store(true, std::sync::atomic::Ordering::SeqCst);

            match Tool::resolve_toolchain()
                .action(self)
                .toolchain(PINNED)
                .executable(PINNED)
                .context(context)
                .call()
                .await
            {
                Ok(_) => Outcome::Passed { summary: None },
                Err(error) => Outcome::Errored {
                    source: Box::new(error),
                },
            }
        }
    }

    /// Resolves the toolchain for an action that requires it in each range,
    /// with the range of the run, in the context
    ///
    /// # Errors
    ///
    /// Returns the error of the lookup, which the test asserts on.
    #[bon::builder]
    async fn toolchain_of(
        ranges: &[&str],
        accepting: Option<&str>,
        #[builder(default = PINNED)] executable: &str,
        context: &Context,
    ) -> Result<Tool, ResolveToolchainError> {
        Tool::resolve_toolchain()
            .action(&Requiring::new(ranges))
            .toolchain(PINNED)
            .executable(executable)
            .maybe_accepting(accepting.map(VersionRange::new))
            .context(context)
            .call()
            .await
    }

    /// Drives the run of an erased action to its outcome
    async fn erased_run(action: &dyn rakko::ErasedAction, context: &Context) -> Outcome {
        action.run(context, &rakko::ArgsValues::empty()).await
    }

    /// Returns a platform with an operating system that is not the one of the
    /// run of the test
    fn another_platform() -> rakko::Platform {
        let system = if cfg!(target_os = "windows") {
            rakko::OperatingSystem::Linux
        } else {
            rakko::OperatingSystem::Windows
        };

        rakko::Platform::new(system)
    }

    /// A program in a directory that cannot be an entry of `PATH`
    ///
    /// The directory holds the separator of the entries of a path on Unix.
    /// The test creates no such directory, because a tool needs only the
    /// location of its program.
    #[cfg(unix)]
    const UNPLACEABLE: &str = "/opt/tool:chain/bin/node";

    /// A program in a directory that cannot be an entry of `PATH`
    ///
    /// Windows refuses a quotation mark in an entry of a path. The test
    /// creates no such directory, because a tool needs only the location of
    /// its program.
    #[cfg(windows)]
    const UNPLACEABLE: &str = "C:\\opt\\tool\"chain\\node.exe";

    /// Returns a tool at a program, without a lookup through mise
    fn tool_at(program: &str) -> Tool {
        Tool {
            program: Program::from(program),
            root: root(),
            version: ToolVersion::new("1.0.0"),
            toolchains: Vec::new(),
        }
    }

    /// Returns the value that a command gives to the path, if it gives one
    fn path_of(invocation: &Invocation) -> Option<std::ffi::OsString> {
        invocation
            .environment()
            .iter()
            .find(|variable| variable.name().get() == PATH)
            .map(|variable| variable.value().get().to_owned())
    }

    #[test]
    fn invocation_of_a_tool_on_a_toolchain_keeps_the_inherited_path() {
        let toolchain = tool_at("/opt/toolchain/bin/node");
        let tool = tool_at("/opt/tool/bin/prettier")
            .with_toolchain(&toolchain)
            .expect("the test adds a toolchain whose directory can be an entry of PATH");
        let inherited: Vec<PathBuf> =
            std::env::split_paths(&std::env::var_os(PATH).unwrap_or_default()).collect();

        let path = path_of(&tool.invocation()).expect("expected the command to set the path");

        assert_eq!(
            std::env::split_paths(&path).skip(1).collect::<Vec<_>>(),
            inherited
        );
    }

    #[test]
    fn invocation_of_a_tool_on_a_toolchain_searches_the_toolchain_first() {
        let toolchain = tool_at("/opt/toolchain/bin/node");
        let tool = tool_at("/opt/tool/bin/prettier")
            .with_toolchain(&toolchain)
            .expect("the test adds a toolchain whose directory can be an entry of PATH");

        let path = path_of(&tool.invocation()).expect("expected the command to set the path");

        assert_eq!(
            std::env::split_paths(&path).next(),
            Some(PathBuf::from("/opt/toolchain/bin"))
        );
    }

    #[test]
    fn invocation_of_a_tool_on_two_toolchains_searches_them_in_order() {
        let first = tool_at("/opt/first/bin/node");
        let second = tool_at("/opt/second/bin/python");
        let tool = tool_at("/opt/tool/bin/prettier")
            .with_toolchain(&first)
            .and_then(|tool| tool.with_toolchain(&second))
            .expect("the test adds toolchains whose directories can be entries of PATH");

        let path = path_of(&tool.invocation()).expect("expected the command to set the path");

        assert_eq!(
            std::env::split_paths(&path).take(2).collect::<Vec<_>>(),
            [
                PathBuf::from("/opt/first/bin"),
                PathBuf::from("/opt/second/bin")
            ]
        );
    }

    #[test]
    fn invocation_of_a_tool_without_a_toolchain_leaves_the_path_alone() {
        let tool = tool_at("/opt/tool/bin/prettier");

        let path = path_of(&tool.invocation());

        assert_eq!(path, None);
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
    async fn resolve_ignores_the_pin_of_the_project() {
        let project = Project::pinning(&["1.7.1"]);

        let tool = Tool::resolve()
            .action(&Declaring(vec![exact()]))
            .tool(PINNED)
            .executable(PINNED)
            .context(&project.context())
            .call()
            .await
            .expect("expected mise to report the declared version");

        assert!(runs_version(&tool, &pinned_by_the_repository()));
    }

    #[tokio::test]
    async fn resolve_of_a_tool_reports_each_program_that_it_provides() {
        let tools = [renovate()];

        let validator = resolved()
            .tools(&tools)
            .tool(RENOVATE)
            .executable("renovate-config-validator")
            .call()
            .await
            .expect("expected mise to report the validator of the declared tool");
        let renovate = resolved()
            .tools(&tools)
            .tool(RENOVATE)
            .executable("renovate")
            .call()
            .await
            .expect("expected mise to report the main program of the declared tool");

        assert_ne!(validator.program(), renovate.program());
    }

    #[tokio::test]
    async fn resolve_requirement_ignores_the_pin_of_the_project() {
        let project = Project::pinning(&["0.0.1"]);

        let tool = Tool::resolve_requirement()
            .requirement(&exact())
            .context(&project.context())
            .call()
            .await;

        assert!(tool.is_ok(), "expected the declared version, got {tool:?}");
    }

    #[tokio::test]
    async fn resolve_requirement_runs_the_tool_in_the_root_of_the_context() {
        let tool = Tool::resolve_requirement()
            .requirement(&exact())
            .context(&context())
            .call()
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

        let result = Tool::resolve_requirement()
            .requirement(&toolchain(">=1"))
            .context(&project.context())
            .call()
            .await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::UnreadableVersion { .. })
            ),
            "expected an unreadable version, got {result:?}"
        );
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

        let result = Tool::resolve_requirement()
            .requirement(&beta)
            .context(&context())
            .call()
            .await;

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

        let result = Tool::resolve_requirement()
            .requirement(&nightly)
            .context(&context())
            .call()
            .await;

        assert!(
            result
                .as_ref()
                .is_ok_and(|tool| tool.version().get().starts_with("nightly-")),
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

        let result = Tool::resolve_requirement()
            .requirement(&nightly)
            .context(&context())
            .call()
            .await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::VersionOutOfRange { .. })
            ),
            "expected a nightly outside the range, got {result:?}"
        );
    }

    /// Mise names a script for a package of Node on every platform
    ///
    /// Windows starts a program by the extension of its file. So this run
    /// reaches the tool only when resolution answered with the launcher beside
    /// the script.
    #[tokio::test]
    async fn resolve_requirement_with_a_tool_of_a_package_manager_reports_a_program_that_runs() {
        let tool = Tool::resolve_requirement()
            .requirement(&scripted())
            .context(&context())
            .call()
            .await
            .expect("expected mise to report the location of a tool that this project installs");

        let execution = tool.invocation().arg("--version").run().await;

        assert!(
            execution.is_ok(),
            "expected the tool to run, got {execution:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_toolchain_in_its_range_reports_the_pinned_version() {
        let pinned = pinned_by_the_repository();

        let result = Tool::resolve_requirement()
            .requirement(&toolchain(">=1"))
            .context(&context())
            .call()
            .await;

        assert!(
            result
                .as_ref()
                .is_ok_and(|tool| runs_version(tool, &pinned)),
            "expected the program of the pinned version, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_a_toolchain_outside_its_range_stops() {
        let result = Tool::resolve_requirement()
            .requirement(&toolchain("<1"))
            .context(&context())
            .call()
            .await;

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

        let result = Tool::resolve_requirement()
            .requirement(&toolchain(">=1"))
            .context(&above.context_below())
            .call()
            .await;

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

        let result = Tool::resolve_requirement()
            .requirement(&toolchain(">=0"))
            .context(&project.context())
            .call()
            .await;

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

        let result = Tool::resolve_requirement()
            .requirement(&toolchain("<99"))
            .context(&project.context())
            .call()
            .await;

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

        let result = Tool::resolve_requirement()
            .requirement(&unknown)
            .context(&context())
            .call()
            .await;

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

        let Err(error) = Tool::resolve_requirement()
            .requirement(&missing)
            .context(&context())
            .call()
            .await
        else {
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

        let Err(error) = Tool::resolve_requirement()
            .requirement(&missing)
            .context(&context())
            .call()
            .await
        else {
            panic!("expected the lookup to report an error");
        };

        assert!(
            error.to_string().contains("`jq` 0.0.1"),
            "expected the tool and the version, got {error}"
        );
    }

    #[tokio::test]
    async fn resolve_requirement_with_an_installed_version_reports_its_program() {
        let tool = Tool::resolve_requirement()
            .requirement(&exact())
            .context(&context())
            .call()
            .await
            .expect("expected mise to report the version that this project pins");

        assert!(tool.program().get().is_file());
    }

    #[tokio::test]
    async fn resolve_requirement_with_an_installed_version_reports_the_version() {
        let tool = Tool::resolve_requirement()
            .requirement(&exact())
            .context(&context())
            .call()
            .await
            .expect("expected mise to report the version that this project pins");

        assert_eq!(tool.version().get(), pinned_by_the_repository());
    }

    #[tokio::test]
    async fn resolve_requirement_with_an_invalid_range_reports_it() {
        let result = Tool::resolve_requirement()
            .requirement(&toolchain("not a range"))
            .context(&context())
            .call()
            .await;

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

        let result = Tool::resolve_requirement()
            .requirement(&unpinned)
            .context(&context_of(outside.path()))
            .call()
            .await;

        assert!(
            matches!(
                result,
                Err(ResolveRequirementError::UnresolvedToolchain { .. })
            ),
            "expected an unpinned toolchain, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_toolchain_with_a_program_that_the_toolchain_lacks_reports_it_missing() {
        let result = toolchain_of()
            .ranges(&[])
            .executable("rakko-provides-no-such-program")
            .context(&context())
            .call()
            .await;

        assert!(
            matches!(result, Err(ResolveToolchainError::MissingExecutable { .. })),
            "expected a missing program, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_toolchain_with_a_range_of_dates_beside_a_pin_of_releases_reports_it_unpinned()
    {
        let result = toolchain_of()
            .ranges(&[">=nightly-2025-02-20", "<1"])
            .context(&context())
            .call()
            .await;

        assert!(
            matches!(result, Err(ResolveToolchainError::UnpinnedToolchain { .. })),
            "expected a pin of another channel to count for nothing, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_toolchain_with_a_range_of_the_run_reports_the_first_pin_that_it_accepts() {
        let pinned = pinned_by_the_repository();
        let project = Project::pinning(&["99.0.0", &pinned]);

        let result = toolchain_of()
            .ranges(&[])
            .accepting("<99")
            .context(&project.context())
            .call()
            .await;

        assert!(
            result
                .as_ref()
                .is_ok_and(|tool| runs_version(tool, &pinned)),
            "expected the program of the pin in the range of the run, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_toolchain_with_a_requirement_and_a_range_of_the_run_reports_the_pin_of_both() {
        let pinned = pinned_by_the_repository();
        let project = Project::pinning(&["0.0.1", "99.0.0", &pinned]);

        let result = toolchain_of()
            .ranges(&[">=1"])
            .accepting("<99")
            .context(&project.context())
            .call()
            .await;

        assert!(
            result
                .as_ref()
                .is_ok_and(|tool| runs_version(tool, &pinned)),
            "expected the program of the pin that both ranges accept, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_toolchain_with_a_requirement_reports_the_first_pin_that_it_accepts() {
        let pinned = pinned_by_the_repository();
        let project = Project::pinning(&["99.0.0", &pinned]);

        let result = toolchain_of()
            .ranges(&["<99"])
            .context(&project.context())
            .call()
            .await;

        assert!(
            result
                .as_ref()
                .is_ok_and(|tool| runs_version(tool, &pinned)),
            "expected the program of the pin in the range, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_toolchain_with_a_toolchain_pinned_above_the_project_reports_it_unpinned() {
        let above = Project::pinning(&[&pinned_by_the_repository()]);

        let result = toolchain_of()
            .ranges(&[])
            .context(&above.context_below())
            .call()
            .await;

        assert!(
            matches!(result, Err(ResolveToolchainError::UnpinnedToolchain { .. })),
            "expected a pin outside the project to count for nothing, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_toolchain_with_a_version_that_is_not_installed_reports_it_uninstalled() {
        let project = Project::pinning(&["1.6"]);

        let result = toolchain_of()
            .ranges(&[])
            .context(&project.context())
            .call()
            .await;

        assert!(
            matches!(
                result,
                Err(ResolveToolchainError::UninstalledToolchain { .. })
            ),
            "expected a pin that mise did not install, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_toolchain_with_an_unmet_requirement_gives_the_pin_and_the_requirement() {
        let pinned = pinned_by_the_repository();

        let result = toolchain_of()
            .ranges(&[">=0", "<1"])
            .context(&context())
            .call()
            .await;

        let Err(ResolveToolchainError::UnmetRequirement { version, range, .. }) = result else {
            panic!("expected an unmet requirement, got {result:?}");
        };
        assert_eq!((version.get(), range.get()), (pinned.as_str(), "<1"));
    }

    #[tokio::test]
    async fn resolve_toolchain_without_a_requirement_reports_the_first_pin() {
        let pinned = pinned_by_the_repository();
        let project = Project::pinning(&[&pinned, "99.0.0"]);

        let result = toolchain_of()
            .ranges(&[])
            .context(&project.context())
            .call()
            .await;

        assert!(
            result
                .as_ref()
                .is_ok_and(|tool| runs_version(tool, &pinned)),
            "expected the program of the first pin, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_with_a_declared_tool_reports_the_program_of_its_version() {
        let tool = resolved()
            .tools(&[exact()])
            .tool(PINNED)
            .executable(PINNED)
            .call()
            .await
            .expect("expected mise to report the version that this project pins");

        assert!(runs_version(&tool, &pinned_by_the_repository()));
    }

    #[tokio::test]
    async fn resolve_with_a_program_that_the_tool_lacks_reports_it_missing() {
        let result = resolved()
            .tools(&[exact()])
            .tool(PINNED)
            .executable("jq-but-not-jq")
            .call()
            .await;

        assert!(
            matches!(
                result,
                Err(ResolveToolError::MissingExecutable { ref executable, .. })
                    if executable.get() == "jq-but-not-jq"
            ),
            "expected a missing program, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_with_a_tool_declared_at_one_version_twice_reports_it() {
        let tools = [exact(), exact()];

        let tool = resolved()
            .tools(&tools)
            .tool(PINNED)
            .executable(PINNED)
            .call()
            .await
            .expect("expected mise to report the one declared version");

        assert_eq!(tool.version().get(), pinned_by_the_repository());
    }

    #[tokio::test]
    async fn resolve_with_a_tool_declared_at_two_versions_reports_it_ambiguous() {
        let tools = [
            exact(),
            requirement(RequiredVersion::Exact(ToolVersion::new("0.0.1"))),
        ];

        let result = resolved()
            .tools(&tools)
            .tool(PINNED)
            .executable(PINNED)
            .call()
            .await;

        assert!(
            matches!(result, Err(ResolveToolError::AmbiguousVersion { ref versions, .. }) if versions.len() == 2),
            "expected an ambiguous version, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_with_a_tool_declared_only_as_a_toolchain_reports_it_undeclared() {
        let result = resolved()
            .tools(&[toolchain(">=1.0")])
            .tool(PINNED)
            .executable(PINNED)
            .call()
            .await;

        assert!(
            matches!(result, Err(ResolveToolError::UndeclaredTool { .. })),
            "expected an undeclared tool, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_with_a_version_that_is_not_installed_reports_it_uninstalled() {
        let missing = requirement(RequiredVersion::Exact(ToolVersion::new("0.0.1")));

        let result = resolved()
            .tools(&[missing])
            .tool(PINNED)
            .executable(PINNED)
            .call()
            .await;

        assert!(
            matches!(result, Err(ResolveToolError::UninstalledVersion { ref version, .. }) if version.get() == "0.0.1"),
            "expected an uninstalled version, got {result:?}"
        );
    }

    #[tokio::test]
    async fn resolve_without_the_tool_in_the_declarations_reports_it_undeclared() {
        let result = resolved()
            .tools(&[renovate()])
            .tool(PINNED)
            .executable(PINNED)
            .call()
            .await;

        assert!(
            matches!(result, Err(ResolveToolError::UndeclaredTool { ref tool }) if tool.get() == PINNED),
            "expected an undeclared tool, got {result:?}"
        );
    }

    #[tokio::test]
    async fn run_of_an_action_whose_tool_has_no_build_here_checks_no_toolchain() {
        let mut action = Requiring::new(&["<1"]);
        action.tools = vec![
            ToolRequirement::builder()
                .tool("github:aonyx-ai/stiles")
                .executable("stiles")
                .exact_version_on(ToolVersion::new("0.6.0"), [another_platform()])
                .build(),
        ];
        let above = Project::pinning(&[&pinned_by_the_repository()]);

        let outcome = erased_run(&action, &above.context_below()).await;

        assert_eq!(
            (
                matches!(outcome, Outcome::Skipped { .. }),
                action.resolved()
            ),
            (true, false),
            "got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn run_of_an_action_with_an_unmet_requirement_reports_the_error() {
        let action = Requiring::new(&["<1"]);

        let outcome = action.run(&context(), &()).await;

        let Outcome::Errored { source } = outcome else {
            panic!("expected the run to stop, got {outcome:?}");
        };
        assert!(
            matches!(
                source.downcast_ref::<ResolveToolchainError>(),
                Some(ResolveToolchainError::UnmetRequirement { .. })
            ),
            "got {source:?}"
        );
    }

    #[tokio::test]
    async fn run_of_an_erased_action_with_an_unmet_requirement_reports_the_error() {
        let action = Requiring::new(&["<1"]);

        let outcome = erased_run(&action, &context()).await;

        let Outcome::Errored { source } = outcome else {
            panic!("expected the run to stop, got {outcome:?}");
        };
        assert!(
            matches!(
                source.downcast_ref::<ResolveToolchainError>(),
                Some(ResolveToolchainError::UnmetRequirement { .. })
            ),
            "got {source:?}"
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

    #[tokio::test]
    async fn select_toolchain_passes_over_the_requirements_of_another_toolchain() {
        let pinned = pinned_by_the_repository();
        let requirements = [ToolchainRequirement::builder()
            .toolchain("node")
            .versions(VersionRange::new("<1"))
            .build()];

        let result = Tool::select_toolchain()
            .toolchain(&MiseTool::new(PINNED))
            .requirements(&requirements)
            .context(&context())
            .call()
            .await;

        assert_eq!(
            result.ok().map(|version| version.get().to_owned()),
            Some(pinned)
        );
    }

    #[tokio::test]
    async fn select_toolchain_with_a_pin_that_is_not_installed_reports_the_pin() {
        let project = Project::pinning(&["1.6"]);

        let result = Tool::select_toolchain()
            .toolchain(&MiseTool::new(PINNED))
            .requirements(&[])
            .context(&project.context())
            .call()
            .await;

        assert_eq!(
            result.ok().map(|version| version.get().to_owned()),
            Some("1.6".to_owned())
        );
    }

    /// Shows that the tool type satisfies Send and Sync
    ///
    /// A scheduler runs actions in parallel and can move a run to a different
    /// thread. So a tool that an action holds across a wait travels with it.
    /// This test holds the tool to the auto traits that make this possible. It
    /// does so because a field of a later version could take them away without
    /// a word from the compiler.
    #[test]
    fn tool_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<Tool>();
    }

    #[test]
    fn tool_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Tool>();
    }

    #[test]
    fn with_toolchain_in_a_directory_that_cannot_stand_on_a_path_reports_it() {
        let toolchain = tool_at(UNPLACEABLE);

        let tool = tool_at("/opt/tool/bin/prettier").with_toolchain(&toolchain);

        assert!(
            matches!(
                &tool,
                Err(ResolveRequirementError::UnplaceableToolchain { program })
                    if program.get() == Path::new(UNPLACEABLE)
            ),
            "expected the toolchain to be refused, got {tool:?}"
        );
    }

    #[test]
    fn with_toolchain_of_a_program_without_a_directory_reports_it() {
        let toolchain = tool_at("");

        let tool = tool_at("/opt/tool/bin/prettier").with_toolchain(&toolchain);

        assert!(
            matches!(
                tool,
                Err(ResolveRequirementError::UnplaceableToolchain { .. })
            ),
            "expected the toolchain to be refused, got {tool:?}"
        );
    }
}
