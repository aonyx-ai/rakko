use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use crate::ProjectRoot;
use crate::path::canonical;

/// The error type for file path parsing
mod error;
pub use self::error::ParseFilePathError;

/// The path of a file that a problem is in
///
/// The path is relative to the project root. A reader, a machine, and a code
/// host therefore see the same path for the same file, and none of them has to
/// know where the project lives on disk.
///
/// Construct a file path through [`FromStr`], [`TryFrom<&str>`],
/// [`TryFrom<String>`], [`TryFrom<&Path>`], or [`TryFrom<PathBuf>`]. Each of
/// these conversions refuses an absolute path and returns a
/// [`ParseFilePathError`]. A path that a tool reported can start with the
/// project root or with `./`, and [`FilePath::within`] makes a file path from
/// it.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FilePath(PathBuf);

impl FilePath {
    /// Returns the path of the file
    pub fn get(&self) -> &Path {
        &self.0
    }

    /// Returns the path of a file that a tool reported, relative to the project
    /// root
    ///
    /// A tool that starts in the root writes a path relative to it, and some
    /// tools write `./` in front of it. The answer drops every component that
    /// names the current directory, because a reader and a code host expect
    /// the path without it.
    ///
    /// A tool can also write an absolute path, and the answer then drops the
    /// root. The root can name the directory of the project through a
    /// symbolic link, and a tool can write the directory that the link
    /// resolves to. The answer therefore also drops the root as the file
    /// system resolves it, which asks the file system and can take time.
    ///
    /// Returns `None` when the root does not contain an absolute path. A tool
    /// that starts in the root reports files below it, so a path that does not
    /// fit points at a report that the caller misread, or at a file outside the
    /// project. The caller decides what to do about that.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    ///
    /// use rakko_action::{FilePath, ProjectRoot};
    ///
    /// let root = ProjectRoot::new(PathBuf::from("project"));
    ///
    /// let path = FilePath::within(Path::new("./src/main.rs"), &root);
    ///
    /// assert_eq!(path, "src/main.rs".parse().ok());
    /// ```
    // action[impl reported.relative]
    // action[impl reported.absolute]
    // action[impl reported.foreign]
    pub fn within(path: &Path, root: &ProjectRoot) -> Option<Self> {
        let relative = if path.is_relative() {
            path.to_path_buf()
        } else if let Ok(stripped) = path.strip_prefix(root.get()) {
            stripped.to_path_buf()
        } else {
            let resolved = canonical(root.get()).ok()?;

            path.strip_prefix(resolved).ok()?.to_path_buf()
        };

        let plain: PathBuf = relative
            .components()
            .filter(|component| *component != Component::CurDir)
            .collect();

        Self::try_from(plain).ok()
    }

    /// Validates that `path` is relative to the project root
    ///
    /// # Errors
    ///
    /// Returns a [`ParseFilePathError`] when `path` is absolute.
    // action[impl location.relative]
    fn validate(path: &Path) -> Result<(), ParseFilePathError> {
        if path.is_absolute() {
            return Err(ParseFilePathError::AbsolutePath {
                path: path.to_path_buf(),
            });
        }

        Ok(())
    }
}

impl FromStr for FilePath {
    type Err = ParseFilePathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(PathBuf::from(s))
    }
}

impl TryFrom<&str> for FilePath {
    type Error = ParseFilePathError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(s))
    }
}

impl TryFrom<String> for FilePath {
    type Error = ParseFilePathError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(s))
    }
}

impl TryFrom<&Path> for FilePath {
    type Error = ParseFilePathError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Self::try_from(path.to_path_buf())
    }
}

impl TryFrom<PathBuf> for FilePath {
    type Error = ParseFilePathError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::validate(&path)?;

        Ok(Self(path))
    }
}

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::fs;

    use rakko_test_utils::path;

    use super::*;

    /// The root that the paths of a test are reported for
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    /// A path that the platform of the test reads as absolute
    ///
    /// A path that starts with a separator is absolute on Unix and relative
    /// on Windows, where a path is absolute only when it names a volume as
    /// well. The tests below need a path that the platform they run on
    /// refuses, so each platform names one of its own.
    const ABSOLUTE: &str = if cfg!(windows) {
        r"C:\Windows\system.ini"
    } else {
        "/etc/hosts"
    };

    #[test]
    fn display_shows_the_path_that_the_file_path_was_made_from() {
        let path: FilePath = "src/main.rs".parse().unwrap();

        assert_eq!(path.to_string(), "src/main.rs");
    }

    #[test]
    fn from_str_accepts_relative_path() {
        let path: FilePath = "src/main.rs".parse().unwrap();

        assert_eq!(path.get(), Path::new("src/main.rs"));
    }

    // action[verify location.relative]
    #[test]
    fn from_str_with_absolute_path_returns_error() {
        let error = ABSOLUTE.parse::<FilePath>().unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from(ABSOLUTE),
            },
        );
    }

    #[test]
    fn get_returns_the_path_that_the_file_path_was_made_from() {
        let path = FilePath::try_from("docs/README.md").unwrap();

        assert_eq!(path.get(), Path::new("docs/README.md"));
    }

    #[test]
    fn try_from_path_accepts_relative_path() {
        let path = FilePath::try_from(Path::new("src/main.rs")).unwrap();

        assert_eq!(path.get(), Path::new("src/main.rs"));
    }

    #[test]
    fn try_from_path_buf_accepts_relative_path() {
        let path = FilePath::try_from(PathBuf::from("src/main.rs")).unwrap();

        assert_eq!(path.get(), Path::new("src/main.rs"));
    }

    // action[verify location.relative]
    #[test]
    fn try_from_path_buf_with_absolute_path_returns_error() {
        let error = FilePath::try_from(PathBuf::from(ABSOLUTE)).unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from(ABSOLUTE),
            },
        );
    }

    // action[verify location.relative]
    #[test]
    fn try_from_path_with_absolute_path_returns_error() {
        let error = FilePath::try_from(Path::new(ABSOLUTE)).unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from(ABSOLUTE),
            },
        );
    }

    #[test]
    fn try_from_str_accepts_relative_path() {
        let path = FilePath::try_from("src/main.rs").unwrap();

        assert_eq!(path.get(), Path::new("src/main.rs"));
    }

    // action[verify location.relative]
    #[test]
    fn try_from_str_with_absolute_path_returns_error() {
        let error = FilePath::try_from(ABSOLUTE).unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from(ABSOLUTE),
            },
        );
    }

    #[test]
    fn try_from_string_accepts_relative_path() {
        let path = FilePath::try_from("src/main.rs".to_string()).unwrap();

        assert_eq!(path.get(), Path::new("src/main.rs"));
    }

    // action[verify location.relative]
    #[test]
    fn try_from_string_with_absolute_path_returns_error() {
        let error = FilePath::try_from(ABSOLUTE.to_string()).unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from(ABSOLUTE),
            },
        );
    }

    // action[verify reported.relative]
    #[test]
    fn within_of_a_path_that_names_the_current_directory_drops_it() {
        let within = FilePath::within(&path("./src/main.rs"), &root());

        assert_eq!(within, Some(FilePath(path("src/main.rs"))));
    }

    // action[verify reported.relative]
    #[test]
    fn within_of_a_relative_path_keeps_it() {
        let within = FilePath::within(&path("src/main.rs"), &root());

        assert_eq!(within, Some(FilePath(path("src/main.rs"))));
    }

    // The root names the temporary directory through `sub/..`, so it never
    // starts the path that the file system resolves. Without that, the
    // temporary directory of Linux is already resolved, and the test passes
    // without the resolution that it is about.
    // action[verify reported.absolute]
    #[test]
    fn within_of_an_absolute_path_below_the_resolved_root_drops_the_root() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        fs::create_dir(directory.path().join("sub")).expect("the test creates a directory");
        let resolved = canonical(directory.path()).expect("the directory exists");
        let root = ProjectRoot::new(directory.path().join("sub").join(".."));

        let within = FilePath::within(&resolved.join("notes.md"), &root);

        assert_eq!(within, Some(FilePath(path("notes.md"))));
    }

    // action[verify reported.absolute]
    #[test]
    fn within_of_an_absolute_path_below_the_root_drops_the_root() {
        let within = FilePath::within(&path("/home/otter/project/src/main.rs"), &root());

        assert_eq!(within, Some(FilePath(path("src/main.rs"))));
    }

    // action[verify reported.foreign]
    #[test]
    fn within_of_an_absolute_path_outside_the_root_is_none() {
        let within = FilePath::within(&path("/home/otter/elsewhere/main.rs"), &root());

        assert_eq!(within, None);
    }
}
