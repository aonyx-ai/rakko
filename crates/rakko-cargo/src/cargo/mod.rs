//! The cargo that a project runs
//!
//! This module holds the program that mise installed for a project, the look
//! that tells whether cargo has anything to do there, the discovery of the
//! workspace roots, and the command that runs one job at a root. An action
//! writes the arguments of its job, and everything between the action and
//! the process lives here.

/// The error that stops the discovery of the roots
mod error;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rakko_action::ProjectRoot;
use rakko_tool::{Execution, Invocation, ResolveToolError, RunCommandError, Tool, ToolName};
use serde::Deserialize;

pub use self::error::DiscoverRootsError;
use crate::root::{CargoRoot, MANIFEST};
use crate::toolchain::Toolchain;
use crate::version::{ReadRustVersionError, RustVersion};

/// The name that mise knows the tool by
const CARGO: &str = "cargo";

/// The directory entry that the look does not read
const GIT_DIRECTORY: &str = ".git";

/// The directory that cargo builds in, which the look does not read
const TARGET_DIRECTORY: &str = "target";

/// The arguments that ask cargo to describe the workspace of a manifest
///
/// Without the dependencies, cargo reads the manifests of the workspace and
/// nothing else, so the description needs no network and no lock file.
const METADATA: [&str; 4] = ["metadata", "--no-deps", "--format-version", "1"];

/// The flag that names the manifest that cargo describes
const MANIFEST_PATH: &str = "--manifest-path";

/// The details of a run of cargo that ended without success and wrote nothing
const NO_DIAGNOSIS: &str = "cargo wrote nothing about it";

/// The cargo that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and a cargo that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_cargo::Cargo;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cargo = Cargo::resolve(ProjectRoot::new("/home/otter/project".into())).await?;
///
/// for root in cargo.roots().await? {
///     let execution = cargo.invocation(&root).arg("check").run().await?;
///
///     println!("{}: {}", root.directory().display(), execution.status());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Cargo {
    /// The root of the project that the tool works on
    root: ProjectRoot,

    /// The program that mise installed for the project
    tool: Tool,
}

impl Cargo {
    /// Returns whether the project holds a manifest that cargo would read
    ///
    /// The look walks the project from its root and stops at the first file
    /// named `Cargo.toml`. It reads hidden directories, because a project
    /// can keep a package in one. It does not read the `.git` entry, which
    /// holds no file of the project, and it does not read a directory named
    /// `target`, where cargo builds. It follows no symbolic link, so a cycle
    /// of links cannot trap it.
    ///
    /// A directory that the look cannot read counts as holding a manifest. A
    /// look that cannot prove absence must not hide a real check behind a
    /// skip, and the discovery of the roots reports its own failure when a
    /// run reaches it.
    // cargo[impl look.git]
    // cargo[impl look.links]
    // cargo[impl look.manifest]
    // cargo[impl look.target]
    // cargo[impl look.unreadable]
    pub async fn applies(root: &ProjectRoot) -> bool {
        match manifests(root.get(), Search::First).await {
            Ok(found) => !found.is_empty(),
            Err(_) => true,
        }
    }

    /// Returns the command that runs cargo at a root
    ///
    /// The command starts the program that [`resolve`][resolve] found, in
    /// the directory of the root, and it carries no argument yet. The caller
    /// adds the arguments of the job that it wants, and then runs the
    /// command. Cargo works on the workspace whose directory it runs in, so
    /// the command line reads the same for every root.
    ///
    /// The command selects the default toolchain of the project, which mise
    /// names in the environment. [`invocation_with_toolchain`][with] selects
    /// another one.
    ///
    /// [resolve]: Cargo::resolve
    /// [with]: Cargo::invocation_with_toolchain
    // cargo[impl run.directory]
    // cargo[impl tool.resolve]
    pub fn invocation(&self, root: &CargoRoot) -> Invocation {
        self.tool
            .invocation()
            .in_directory(root.directory().as_path())
    }

    /// Returns the command that runs cargo at a root, on a toolchain
    ///
    /// The command is the one that [`invocation`][invocation] returns, with
    /// the toolchain in front of every other argument, in the form that the
    /// proxy of rustup reads. The argument selects which cargo answers, and
    /// it changes nothing about what that cargo does.
    ///
    /// [invocation]: Cargo::invocation
    // cargo[impl run.toolchain]
    pub fn invocation_with_toolchain(&self, root: &CargoRoot, toolchain: &Toolchain) -> Invocation {
        self.invocation(root).arg(toolchain.argument())
    }

    /// Returns the cargo that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names,
    /// so the version that the project pinned answers, whatever the shell
    /// that started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no cargo for the
    /// project.
    // cargo[impl tool.missing]
    // cargo[impl tool.resolve]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(CARGO), root.clone()).await?;

        Ok(Self { root, tool })
    }

    /// Returns every workspace root of the project, in the order of their
    /// paths
    ///
    /// The discovery walks the project for manifests with the rules of the
    /// look, and it asks cargo which workspace each manifest belongs to.
    /// Cargo names the root of that workspace and its members, so the
    /// members need no question of their own, and a root counts once
    /// however many members it has.
    ///
    /// The discovery starts a process per workspace, so a caller that needs
    /// the roots more than once keeps the answer for the length of the run.
    ///
    /// # Errors
    ///
    /// Returns [`UnreadableDirectory`][directory] when a directory of the
    /// project cannot be read, [`CargoUnavailable`][unavailable] when cargo
    /// does not run, [`UnreadableManifest`][manifest] when cargo refuses a
    /// manifest, [`UnrecognizedMetadata`][metadata] when cargo describes a
    /// workspace in a shape that the crate cannot read, and
    /// [`ForeignWorkspace`][foreign] when a manifest belongs to a workspace
    /// whose root lies outside the project.
    ///
    /// [directory]: DiscoverRootsError::UnreadableDirectory
    /// [foreign]: DiscoverRootsError::ForeignWorkspace
    /// [manifest]: DiscoverRootsError::UnreadableManifest
    /// [metadata]: DiscoverRootsError::UnrecognizedMetadata
    /// [unavailable]: DiscoverRootsError::CargoUnavailable
    // cargo[impl root.contained]
    // cargo[impl root.discover]
    // cargo[impl root.member]
    // cargo[impl root.walk]
    pub async fn roots(&self) -> Result<Vec<CargoRoot>, DiscoverRootsError> {
        let mut manifests = manifests(self.root.get(), Search::All).await?;
        manifests.sort_by_key(|manifest| (manifest.components().count(), manifest.clone()));

        let project = canonical(self.root.get()).await;
        let mut claimed: HashSet<PathBuf> = HashSet::new();
        let mut roots: Vec<CargoRoot> = Vec::new();

        for manifest in manifests {
            if claimed.contains(&canonical(&manifest).await) {
                continue;
            }

            let metadata = self.metadata(&manifest).await?;
            let directory = canonical(&metadata.workspace_root).await;

            if !directory.starts_with(&project) {
                return Err(DiscoverRootsError::ForeignWorkspace {
                    manifest,
                    workspace: directory,
                });
            }

            claimed.insert(directory.join(MANIFEST));
            for package in metadata.packages {
                claimed.insert(canonical(&package.manifest_path).await);
            }

            let root = CargoRoot::new(directory);
            if !roots.contains(&root) {
                roots.push(root);
            }
        }

        roots.sort();

        Ok(roots)
    }

    /// Returns the Rust version that the packages of a root declare
    ///
    /// A package declares the oldest toolchain that it compiles on as the
    /// `rust-version` of its manifest, and cargo resolves the inheritance
    /// from the workspace before it reports the declaration. A workspace
    /// compiles as one unit, so the highest declaration of its packages
    /// answers, and a workspace whose packages declare nothing answers
    /// `None`.
    ///
    /// The lookup starts a process, so a caller that needs the version more
    /// than once keeps the answer for the length of the run.
    ///
    /// # Errors
    ///
    /// Returns [`CargoUnavailable`][unavailable] when cargo does not run,
    /// [`UnreadableManifest`][manifest] when cargo refuses the manifest of
    /// the root, and [`UnrecognizedMetadata`][metadata] when cargo describes
    /// the workspace in a shape that the crate cannot read.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// cargo, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_action::ProjectRoot;
    /// use rakko_cargo::Cargo;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let cargo = Cargo::resolve(ProjectRoot::new("/home/otter/project".into())).await?;
    ///
    /// for root in cargo.roots().await? {
    ///     if let Some(version) = cargo.rust_version(&root).await? {
    ///         println!("{}", version);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [manifest]: ReadRustVersionError::UnreadableManifest
    /// [metadata]: ReadRustVersionError::UnrecognizedMetadata
    /// [unavailable]: ReadRustVersionError::CargoUnavailable
    // cargo[impl version.declared]
    // cargo[impl version.unreadable]
    pub async fn rust_version(
        &self,
        root: &CargoRoot,
    ) -> Result<Option<RustVersion>, ReadRustVersionError> {
        let manifest = root.manifest();
        let metadata = self
            .describe(&manifest)
            .await
            .map_err(|failure| match failure {
                MetadataFailure::Unavailable { source } => {
                    ReadRustVersionError::CargoUnavailable { source }
                }
                MetadataFailure::Unreadable { details } => {
                    ReadRustVersionError::UnreadableManifest { manifest, details }
                }
                MetadataFailure::Unrecognized { source } => {
                    ReadRustVersionError::UnrecognizedMetadata { manifest, source }
                }
            })?;

        Ok(RustVersion::highest(
            metadata
                .packages
                .into_iter()
                .filter_map(|package| package.rust_version)
                .map(RustVersion::new),
        ))
    }

    /// Asks cargo to describe the workspace of a manifest
    ///
    /// The question runs in the directory of the manifest, because cargo
    /// reads the configuration in `.cargo` from the directory it runs in
    /// and not from the manifest it is given. The job that follows runs in
    /// the directory of the root, so the answer and the job see the same
    /// configuration.
    ///
    /// # Errors
    ///
    /// Returns what went wrong, without the manifest: every caller knows
    /// which manifest it asked about, and each of them reports the failure
    /// in the error of its own question.
    async fn describe(&self, manifest: &Path) -> Result<Metadata, MetadataFailure> {
        let directory = manifest.parent().unwrap_or(self.root.get());
        let execution = self
            .tool
            .invocation()
            .in_directory(directory)
            .args(METADATA)
            .arg(MANIFEST_PATH)
            .arg(manifest)
            .run()
            .await
            .map_err(|source| MetadataFailure::Unavailable { source })?;

        if !execution.status().success() {
            return Err(MetadataFailure::Unreadable {
                details: details(&execution),
            });
        }

        serde_json::from_slice(execution.stdout().get())
            .map_err(|source| MetadataFailure::Unrecognized { source })
    }

    /// Asks cargo which workspace a manifest belongs to
    ///
    /// # Errors
    ///
    /// Returns [`CargoUnavailable`][unavailable] when cargo does not run,
    /// [`UnreadableManifest`][manifest] when cargo ends without success, and
    /// [`UnrecognizedMetadata`][metadata] when its answer has a shape that
    /// the crate cannot read.
    ///
    /// [manifest]: DiscoverRootsError::UnreadableManifest
    /// [metadata]: DiscoverRootsError::UnrecognizedMetadata
    /// [unavailable]: DiscoverRootsError::CargoUnavailable
    // cargo[impl root.manifest]
    async fn metadata(&self, manifest: &Path) -> Result<Metadata, DiscoverRootsError> {
        self.describe(manifest)
            .await
            .map_err(|failure| match failure {
                MetadataFailure::Unavailable { source } => {
                    DiscoverRootsError::CargoUnavailable { source }
                }
                MetadataFailure::Unreadable { details } => DiscoverRootsError::UnreadableManifest {
                    manifest: manifest.to_path_buf(),
                    details,
                },
                MetadataFailure::Unrecognized { source } => {
                    DiscoverRootsError::UnrecognizedMetadata {
                        manifest: manifest.to_path_buf(),
                        source,
                    }
                }
            })
    }
}

/// What went wrong when cargo described a manifest
///
/// The question has two callers, and each of them reports the failure in the
/// error of its own question. This enum carries what happened, and the
/// caller adds the manifest that it asked about.
#[derive(Debug)]
enum MetadataFailure {
    /// Cargo did not run
    Unavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Cargo refused the manifest
    Unreadable {
        /// What cargo wrote about the manifest
        details: String,
    },

    /// Cargo answered in a shape that the crate does not recognize
    Unrecognized {
        /// The cause of the failure
        source: serde_json::Error,
    },
}

/// What cargo reports about the workspace of a manifest
///
/// Cargo writes far more than these fields, and the reading ignores the
/// rest, so a field that a new version adds does not break it.
#[derive(Deserialize)]
struct Metadata {
    /// The directory of the workspace that the manifest belongs to
    workspace_root: PathBuf,

    /// The packages of the workspace
    packages: Vec<Package>,
}

/// One package of a workspace, as cargo describes it
#[derive(Deserialize)]
struct Package {
    /// The manifest of the package
    manifest_path: PathBuf,

    /// The oldest Rust toolchain that the package declares it compiles on,
    /// with the inheritance from the workspace resolved, when the package
    /// declares one
    rust_version: Option<String>,
}

/// How far a walk for manifests goes
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Search {
    /// Stop at the first manifest, because the caller asks whether one exists
    First,

    /// Collect every manifest, because the caller asks for all of them
    All,
}

/// Returns the path with every symbolic link resolved
///
/// Cargo resolves the links of a path that it reports, and a temporary
/// directory on macOS sits behind one, so a path of the walk and a path of
/// cargo can name one file in two ways. Both go through this function before
/// they meet. A path that cannot be resolved stays as it is.
async fn canonical(path: &Path) -> PathBuf {
    tokio::fs::canonicalize(path)
        .await
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Returns what cargo said about a manifest that it refused
///
/// Cargo writes its diagnosis to the standard error stream, and the text
/// travels into the error, so whoever reads the failure reads the answer of
/// cargo instead of a sentence that Rakko wrote about it.
fn details(execution: &Execution) -> String {
    let diagnosis = execution.stderr().to_string_lossy();
    let text = diagnosis.trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
    }
}

/// Returns the manifests below a directory
///
/// The walk does not read the `.git` entry, a directory named `target`, or a
/// symbolic link.
///
/// # Errors
///
/// Returns [`UnreadableDirectory`][directory] when a directory of the walk
/// cannot be read.
///
/// [directory]: DiscoverRootsError::UnreadableDirectory
// cargo[impl look.git]
// cargo[impl look.links]
// cargo[impl look.manifest]
// cargo[impl look.target]
// cargo[impl root.directory]
// cargo[impl root.walk]
async fn manifests(root: &Path, search: Search) -> Result<Vec<PathBuf>, DiscoverRootsError> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let mut entries = tokio::fs::read_dir(&directory)
            .await
            .map_err(|source| unreadable(&directory, source))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|source| unreadable(&directory, source))?
        {
            let name = entry.file_name();

            if name == GIT_DIRECTORY || name == TARGET_DIRECTORY {
                continue;
            }

            let kind = entry
                .file_type()
                .await
                .map_err(|source| unreadable(&directory, source))?;

            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() && name == MANIFEST {
                found.push(entry.path());

                if search == Search::First {
                    return Ok(found);
                }
            }
        }
    }

    Ok(found)
}

/// Returns the error for a directory that the walk could not read
fn unreadable(directory: &Path, source: std::io::Error) -> DiscoverRootsError {
    DiscoverRootsError::UnreadableDirectory {
        directory: directory.to_path_buf(),
        source,
    }
}
