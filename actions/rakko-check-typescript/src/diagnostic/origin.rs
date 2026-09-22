use std::path::{Component, Path, PathBuf};

use bon::Builder;
use getset::{CopyGetters, Getters};
use rakko_action::{FilePath, ProjectRoot};

/// The place in the project that a diagnostic points at
///
/// Tsc writes the path, the line, and the column in front of a diagnostic that
/// it found in a file. The path stands as tsc wrote it: a run starts tsc in
/// the project root and names the root as the project, so tsc reports a path
/// relative to it. A caller that reports the diagnostic asks for the
/// [relative][relative] path.
///
/// [relative]: Origin::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub struct Origin {
    /// The path of the file, as tsc wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The line that the diagnostic points at, starting at 1
    #[getset(get_copy = "pub")]
    line: u32,

    /// The column that the diagnostic points at, starting at 1
    #[getset(get_copy = "pub")]
    column: u32,
}

impl Origin {
    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. A run names the
    /// root as the project, so a path that does not fit points at a report
    /// that the caller misread, and the caller decides what to do about that.
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::try_from(strip(&self.path, root)?).ok()
    }
}

/// Returns the path without the prefix that names where tsc started
///
/// A run names the project root as the project, so tsc reports a path that is
/// already relative to it. A path that names the current directory in front of
/// it loses that part, because a reader and a code host expect the plain path.
///
/// A path that arrives absolute loses the project root instead. The root of a
/// context can name the same directory through a symbolic link, which is why
/// the canonical root is tried as well.
fn strip(path: &Path, root: &ProjectRoot) -> Option<PathBuf> {
    if path.is_relative() {
        let plain: PathBuf = path
            .components()
            .filter(|component| *component != Component::CurDir)
            .collect();

        return Some(plain);
    }

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

    use rakko_test_utils::path;

    use super::*;

    /// Returns the place of a diagnostic in the given file
    fn origin(path: PathBuf) -> Origin {
        Origin::builder().path(path).line(2).column(3).build()
    }

    /// The root that the diagnostics of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn relative_path_of_a_path_below_the_root_keeps_it() {
        let relative = origin(path("src/index.ts")).relative_path(&root());

        assert_eq!(relative, Some("src/index.ts".parse().unwrap()));
    }

    #[test]
    fn relative_path_of_a_path_outside_the_root_is_none() {
        let relative = origin(path("/elsewhere/index.ts")).relative_path(&root());

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_of_a_path_that_names_the_current_directory_drops_it() {
        let relative = origin(path("./src/index.ts")).relative_path(&root());

        assert_eq!(relative, Some("src/index.ts".parse().unwrap()));
    }

    #[test]
    fn relative_path_of_an_absolute_path_below_the_root_strips_it() {
        let relative = origin(path("/home/otter/project/src/index.ts")).relative_path(&root());

        assert_eq!(relative, Some("src/index.ts".parse().unwrap()));
    }
}
