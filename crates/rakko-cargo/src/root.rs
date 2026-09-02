use std::path::{Component, Path, PathBuf};

use getset::Getters;
use rakko_action::{FilePath, ProjectRoot};

/// The name of the file that describes a package or a workspace to cargo
pub const MANIFEST: &str = "Cargo.toml";

/// One workspace root of a project
///
/// Cargo works on one workspace at a time, and a project can hold more than
/// one: the harness of a project is a package of its own, outside the
/// workspace of the crates that it maintains. A root names the directory of
/// one workspace, so that an action runs its job there and reads the paths
/// that cargo reports relative to it.
///
/// The directory is absolute, because a run starts cargo in it. A caller
/// that reports a problem asks for a path [relative][relative] to the
/// project root, which is the name that a reader, a machine, and a code host
/// all recognize.
///
/// [relative]: CargoRoot::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct CargoRoot {
    /// The directory that holds the manifest of the workspace
    #[getset(get = "pub")]
    directory: PathBuf,
}

impl CargoRoot {
    /// Creates a root from the directory that holds its manifest
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    /// Returns the path of the manifest of the workspace
    pub fn manifest(&self) -> PathBuf {
        self.directory.join(MANIFEST)
    }

    /// Returns the directory of the root, relative to the project root
    ///
    /// Returns `None` when the project root does not contain the directory.
    /// The discovery finds a root below the project root, so a directory
    /// that does not fit points at a project that moved while the run was
    /// on, and the caller decides what to do about that.
    ///
    /// The directory of the project root itself has no name, and the empty
    /// path is that name: joining a file to it yields the file.
    // cargo[impl path.relative]
    // cargo[impl path.foreign]
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
    // cargo[impl path.relative]
    // cargo[impl path.foreign]
    // cargo[impl path.parent]
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
// cargo[impl path.parent]
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
/// link, and cargo answers with the directory that it resolved, which is why
/// the canonical root is tried as well.
fn strip(path: &Path, root: &ProjectRoot) -> Option<PathBuf> {
    if let Ok(stripped) = path.strip_prefix(root.get()) {
        return Some(stripped.to_path_buf());
    }

    let canonical = root.get().canonicalize().ok()?;

    path.strip_prefix(canonical).ok().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns the project root that the tests place a root in
    fn project() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    /// Returns a root in a subdirectory of the project
    fn nested() -> CargoRoot {
        CargoRoot::new(PathBuf::from("/home/otter/project/tools/harness"))
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

    // cargo[verify path.relative]
    #[test]
    fn relative_directory_of_the_project_root_is_empty() {
        let root = CargoRoot::new(PathBuf::from("/home/otter/project"));

        let directory = root.relative_directory(&project());

        assert_eq!(directory, Some(PathBuf::new()));
    }

    // cargo[verify path.foreign]
    #[test]
    fn relative_directory_outside_the_project_names_nothing() {
        let root = CargoRoot::new(PathBuf::from("/home/otter/elsewhere"));

        let directory = root.relative_directory(&project());

        assert_eq!(directory, None);
    }

    // cargo[verify path.relative]
    #[test]
    fn relative_directory_under_the_project_drops_the_project_root() {
        let root = nested();

        let directory = root.relative_directory(&project());

        assert_eq!(directory, Some(PathBuf::from("tools/harness")));
    }

    // cargo[verify path.parent]
    #[test]
    fn relative_path_of_a_path_that_climbs_out_of_the_project_names_nothing() {
        let root = nested();

        let path = root.relative_path(Path::new("../../../outside.rs"), &project());

        assert_eq!(path, None);
    }

    // cargo[verify path.parent]
    #[test]
    fn relative_path_of_a_path_that_climbs_within_the_project_resolves_the_climb() {
        let root = nested();

        let path = root.relative_path(Path::new("../other/src/lib.rs"), &project());

        assert_eq!(path, FilePath::try_from("tools/other/src/lib.rs").ok());
    }

    // cargo[verify path.relative]
    #[test]
    fn relative_path_of_a_relative_path_joins_the_root() {
        let root = nested();

        let path = root.relative_path(Path::new("src/main.rs"), &project());

        assert_eq!(path, FilePath::try_from("tools/harness/src/main.rs").ok());
    }

    // cargo[verify path.relative]
    #[test]
    fn relative_path_of_an_absolute_path_drops_the_project_root() {
        let root = nested();

        let path = root.relative_path(
            Path::new("/home/otter/project/tools/harness/src/main.rs"),
            &project(),
        );

        assert_eq!(path, FilePath::try_from("tools/harness/src/main.rs").ok());
    }

    // cargo[verify path.foreign]
    #[test]
    fn relative_path_of_an_absolute_path_outside_the_project_names_nothing() {
        let root = nested();

        let path = root.relative_path(Path::new("/home/otter/.cargo/registry/a.rs"), &project());

        assert_eq!(path, None);
    }
}
