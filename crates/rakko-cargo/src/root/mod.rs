//! One workspace root of a project
//!
//! This module holds the root that the discovery reports and what a root knows
//! about the packages of its workspace. A caller reads that instead of asking
//! cargo a second time.

mod documentation;
mod package;

use std::path::{Component, Path, PathBuf};

use bon::bon;
use getset::{CopyGetters, Getters};
use rakko_action::path::canonical;
use rakko_action::{FilePath, ProjectRoot};

pub use self::documentation::Documentation;
pub use self::package::{CargoPackage, DocumentedNames, PackageName};
use crate::version::RustVersion;

/// The name of the file that describes a package or a workspace to cargo
///
/// # Examples
///
/// ```
/// use rakko_cargo::root::MANIFEST;
///
/// assert_eq!(MANIFEST, "Cargo.toml");
/// ```
pub const MANIFEST: &str = "Cargo.toml";

/// One workspace root of a project
///
/// Cargo works on one workspace at a time, and a project can hold more than
/// one. The harness of a project is a package of its own, outside the
/// workspace of the crates that it maintains. A root names the directory of
/// one workspace. This lets an action run its job there and read the paths
/// that cargo reports relative to it.
///
/// The directory is absolute, because a run starts cargo in it. A caller that
/// reports a problem asks for a path [relative][relative] to the project root.
/// This is the name that a reader, a machine, and a code host all recognize.
///
/// A root also carries what cargo said about the packages of its workspace. It
/// carries their names and which of them share a crate name. It also carries
/// whether cargo can test the examples in their documentation and the oldest
/// toolchain that they promise to compile on. The discovery of the roots asks
/// cargo to describe every workspace anyway, and one description answers all
/// of them. So a caller reads them here instead of asking cargo again.
///
/// The type has no order, because a version has none. A caller that wants
/// the roots in a fixed order sorts them by their directory, which is how
/// the discovery reports them.
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
///     println!("{}", root.directory().display());
/// }
/// # Ok(())
/// # }
/// ```
///
/// [relative]: CargoRoot::relative_path
#[derive(Clone, Eq, PartialEq, Hash, Debug, CopyGetters, Getters)]
pub struct CargoRoot {
    /// The directory that holds the manifest of the workspace
    #[getset(get = "pub")]
    directory: PathBuf,

    /// Whether cargo can test the examples in the documentation of the
    /// workspace
    #[getset(get_copy = "pub")]
    documentation: Documentation,

    /// Every package of the workspace, in the order of their names
    #[getset(get = "pub")]
    packages: Vec<CargoPackage>,

    /// The oldest Rust toolchain that the packages of the workspace declare
    /// they compile on, or `None` when no package declares one
    #[getset(get = "pub")]
    rust_version: Option<RustVersion>,
}

#[bon]
impl CargoRoot {
    /// Creates a root from the directory that holds its manifest and what
    /// cargo said about the packages of the workspace
    ///
    /// A root whose packages declare no `rust-version` carries none. A root
    /// built without its packages names none, so no package of it shares a
    /// crate name. A caller treats the workspace as one whole.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_cargo::{CargoRoot, Documentation};
    /// use std::path::PathBuf;
    ///
    /// let root = CargoRoot::builder()
    ///     .directory(PathBuf::from("/home/otter/project"))
    ///     .documentation(Documentation::Testable)
    ///     .build();
    ///
    /// assert_eq!(root.directory(), &PathBuf::from("/home/otter/project"));
    /// ```
    #[builder]
    pub fn new(
        directory: PathBuf,
        documentation: Documentation,
        #[builder(default)] packages: Vec<CargoPackage>,
        rust_version: Option<RustVersion>,
    ) -> Self {
        Self {
            directory,
            documentation,
            packages,
            rust_version,
        }
    }

    /// Returns the path of the manifest of the workspace
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_cargo::{CargoRoot, Documentation};
    /// use std::path::PathBuf;
    ///
    /// let root = CargoRoot::builder()
    ///     .directory(PathBuf::from("/home/otter/project"))
    ///     .documentation(Documentation::Testable)
    ///     .build();
    ///
    /// assert_eq!(root.manifest(), PathBuf::from("/home/otter/project/Cargo.toml"));
    /// ```
    pub fn manifest(&self) -> PathBuf {
        self.directory.join(MANIFEST)
    }

    /// Returns the directory of the root, relative to the project root
    ///
    /// Returns `None` when the project root does not contain the directory.
    /// The discovery finds a root below the project root. So a directory that
    /// does not fit points at a project that moved while the run was on. The
    /// caller decides what to do about that.
    ///
    /// The directory of the project root itself has no name, and the empty
    /// path is that name: joining a file to it yields the file.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_action::ProjectRoot;
    /// use rakko_cargo::Cargo;
    /// use std::path::PathBuf;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let project = ProjectRoot::new("/home/otter/project".into());
    /// let cargo = Cargo::resolve(project.clone()).await?;
    ///
    /// for root in cargo.roots().await? {
    ///     if let Some(relative) = root.relative_directory(&project) {
    ///         println!("{}", relative.display());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn relative_directory(&self, root: &ProjectRoot) -> Option<PathBuf> {
        strip(&self.directory, root)
    }

    /// Returns the path of a file that cargo wrote, relative to the project
    /// root
    ///
    /// Cargo writes a path relative to the root that it works on, and a tool
    /// that cargo runs can write an absolute path instead. This method reads
    /// both: a relative path is joined to the directory of this root first,
    /// and an absolute path is taken as it is.
    ///
    /// Returns `None` when the project root does not contain the file, which
    /// happens for a diagnostic in a dependency that lives elsewhere on the
    /// machine. A path that climbs through its parents is resolved first, so
    /// a path that leaves the project that way gets no name either.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_action::ProjectRoot;
    /// use rakko_cargo::Cargo;
    /// use std::path::Path;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let project = ProjectRoot::new("/home/otter/project".into());
    /// let cargo = Cargo::resolve(project.clone()).await?;
    ///
    /// for root in cargo.roots().await? {
    ///     if let Some(path) = root.relative_path(Path::new("src/main.rs"), &project) {
    ///         println!("{}", path.get().display());
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn relative_path(&self, path: &Path, root: &ProjectRoot) -> Option<FilePath> {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.directory.join(path)
        };

        FilePath::try_from(strip(&normalize(&absolute), root)?).ok()
    }
}

/// Returns the path with every `.` dropped and every `..` resolved against
/// the component before it
///
/// A prefix check on a path that climbs through its parents answers for the
/// wrong directory, so the climb is resolved first. The resolution is
/// lexical: a component that is a symbolic link resolves as a directory,
/// which is how cargo wrote the path.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }

    normalized
}

/// Returns the path without the project root that prefixes it
///
/// The root of a context can name the same directory through a symbolic
/// link.
/// Cargo answers with the directory that it resolved, which is why the
/// canonical root is tried as well.
fn strip(path: &Path, root: &ProjectRoot) -> Option<PathBuf> {
    if let Ok(stripped) = path.strip_prefix(root.get()) {
        return Some(stripped.to_path_buf());
    }

    let resolved = canonical(root.get()).ok()?;

    path.strip_prefix(resolved).ok().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Returns the project root that the tests place a root in
    fn project() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    /// Returns a root in a subdirectory of the project
    fn nested() -> CargoRoot {
        CargoRoot::builder()
            .directory(PathBuf::from("/home/otter/project/tools/harness"))
            .documentation(Documentation::Testable)
            .build()
    }

    #[test]
    fn cargo_root_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CargoRoot>();
    }

    #[test]
    fn manifest_names_the_file_in_the_directory() {
        let root = nested();

        let manifest = root.manifest();

        assert_eq!(
            manifest,
            PathBuf::from("/home/otter/project/tools/harness/Cargo.toml")
        );
    }

    #[test]
    fn relative_directory_of_the_project_root_is_empty() {
        let root = CargoRoot::builder()
            .directory(PathBuf::from("/home/otter/project"))
            .documentation(Documentation::Testable)
            .build();

        let directory = root.relative_directory(&project());

        assert_eq!(directory, Some(PathBuf::new()));
    }

    #[test]
    fn relative_directory_outside_the_project_names_nothing() {
        let root = CargoRoot::builder()
            .directory(PathBuf::from("/home/otter/elsewhere"))
            .documentation(Documentation::Testable)
            .build();

        let directory = root.relative_directory(&project());

        assert_eq!(directory, None);
    }

    #[test]
    fn relative_directory_under_the_project_drops_the_project_root() {
        let root = nested();

        let directory = root.relative_directory(&project());

        assert_eq!(directory, Some(PathBuf::from("tools/harness")));
    }

    #[test]
    fn relative_path_of_a_path_that_climbs_out_of_the_project_names_nothing() {
        let root = nested();

        let path = root.relative_path(Path::new("../../../outside.rs"), &project());

        assert_eq!(path, None);
    }

    #[test]
    fn relative_path_of_a_path_that_climbs_within_the_project_resolves_the_climb() {
        let root = nested();

        let path = root.relative_path(Path::new("../other/src/lib.rs"), &project());

        assert_eq!(path, FilePath::try_from("tools/other/src/lib.rs").ok());
    }

    #[test]
    fn relative_path_of_a_relative_path_joins_the_root() {
        let root = nested();

        let path = root.relative_path(Path::new("src/main.rs"), &project());

        assert_eq!(path, FilePath::try_from("tools/harness/src/main.rs").ok());
    }

    #[test]
    fn relative_path_of_an_absolute_path_drops_the_project_root() {
        let root = nested();

        let path = root.relative_path(
            Path::new("/home/otter/project/tools/harness/src/main.rs"),
            &project(),
        );

        assert_eq!(path, FilePath::try_from("tools/harness/src/main.rs").ok());
    }

    #[test]
    fn relative_path_of_an_absolute_path_outside_the_project_names_nothing() {
        let root = nested();

        let path = root.relative_path(Path::new("/home/otter/.cargo/registry/a.rs"), &project());

        assert_eq!(path, None);
    }

    /// Shows that the resolved root must use cargo's spelling
    ///
    /// Cargo writes the directory that the file system resolved.
    /// On some platforms the temporary directory differs from it as text: a
    /// symbolic link on macOS or a short name on Windows.
    /// The resolved root must carry the spelling that cargo writes, or no file
    /// gets a name.
    #[test]
    fn relative_path_of_an_absolute_path_under_the_resolved_root_drops_it() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = ProjectRoot::new(directory.path().to_path_buf());
        let resolved = canonical(directory.path()).expect("the directory exists");
        let root = CargoRoot::builder()
            .directory(resolved.clone())
            .documentation(Documentation::Testable)
            .build();

        let path = root.relative_path(&resolved.join("src/main.rs"), &project);

        assert_eq!(path, FilePath::try_from("src/main.rs").ok());
    }
}
