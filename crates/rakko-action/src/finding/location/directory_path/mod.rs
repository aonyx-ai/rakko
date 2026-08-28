use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// The error type for directory path parsing
mod error;
pub use self::error::ParseDirectoryPathError;

/// The path of a directory that a problem is in
///
/// The path is relative to the project root. A reader, a machine, and a code
/// host therefore see the same path for the same directory, and none of them
/// has to know where the project lives on disk.
///
/// A tool that reports per directory rather than per file gives this path. A
/// coverage report is one example: it names a directory and says nothing
/// about the files in it.
///
/// Construct a directory path through [`FromStr`], [`TryFrom<&str>`],
/// [`TryFrom<String>`], [`TryFrom<&Path>`], or [`TryFrom<PathBuf>`]. Every
/// constructor refuses an absolute path and returns a
/// [`ParseDirectoryPathError`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DirectoryPath(PathBuf);

impl DirectoryPath {
    /// Returns the path of the directory
    pub fn get(&self) -> &Path {
        &self.0
    }

    /// Validates that `path` is relative to the project root
    ///
    /// # Errors
    ///
    /// Returns a [`ParseDirectoryPathError`] when `path` is absolute.
    // action[impl location.relative]
    fn validate(path: &Path) -> Result<(), ParseDirectoryPathError> {
        if path.is_absolute() {
            return Err(ParseDirectoryPathError::AbsolutePath {
                path: path.to_path_buf(),
            });
        }

        Ok(())
    }
}

impl FromStr for DirectoryPath {
    type Err = ParseDirectoryPathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(PathBuf::from(s))
    }
}

impl TryFrom<&str> for DirectoryPath {
    type Error = ParseDirectoryPathError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(s))
    }
}

impl TryFrom<String> for DirectoryPath {
    type Error = ParseDirectoryPathError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(s))
    }
}

impl TryFrom<&Path> for DirectoryPath {
    type Error = ParseDirectoryPathError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Self::try_from(path.to_path_buf())
    }
}

impl TryFrom<PathBuf> for DirectoryPath {
    type Error = ParseDirectoryPathError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::validate(&path)?;

        Ok(Self(path))
    }
}

impl fmt::Display for DirectoryPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn display_shows_the_path_that_the_directory_path_was_made_from() {
        let path: DirectoryPath = "crates/rakko".parse().unwrap();

        assert_eq!(path.to_string(), "crates/rakko");
    }

    #[test]
    fn from_str_accepts_relative_path() {
        let path: DirectoryPath = "crates/rakko".parse().unwrap();

        assert_eq!(path.get(), Path::new("crates/rakko"));
    }

    // action[verify location.relative]
    #[test]
    fn from_str_with_absolute_path_returns_error() {
        let error = "/etc".parse::<DirectoryPath>().unwrap_err();

        assert_eq!(
            error,
            ParseDirectoryPathError::AbsolutePath {
                path: PathBuf::from("/etc"),
            },
        );
    }

    #[test]
    fn get_returns_the_path_that_the_directory_path_was_made_from() {
        let path = DirectoryPath::try_from("docs").unwrap();

        assert_eq!(path.get(), Path::new("docs"));
    }

    #[test]
    fn try_from_path_accepts_relative_path() {
        let path = DirectoryPath::try_from(Path::new("crates/rakko")).unwrap();

        assert_eq!(path.get(), Path::new("crates/rakko"));
    }

    // action[verify location.relative]
    #[test]
    fn try_from_path_buf_with_absolute_path_returns_error() {
        let error = DirectoryPath::try_from(PathBuf::from("/etc")).unwrap_err();

        assert_eq!(
            error,
            ParseDirectoryPathError::AbsolutePath {
                path: PathBuf::from("/etc"),
            },
        );
    }

    #[test]
    fn try_from_path_buf_accepts_relative_path() {
        let path = DirectoryPath::try_from(PathBuf::from("crates/rakko")).unwrap();

        assert_eq!(path.get(), Path::new("crates/rakko"));
    }

    // action[verify location.relative]
    #[test]
    fn try_from_path_with_absolute_path_returns_error() {
        let error = DirectoryPath::try_from(Path::new("/etc")).unwrap_err();

        assert_eq!(
            error,
            ParseDirectoryPathError::AbsolutePath {
                path: PathBuf::from("/etc"),
            },
        );
    }

    #[test]
    fn try_from_str_accepts_relative_path() {
        let path = DirectoryPath::try_from("crates/rakko").unwrap();

        assert_eq!(path.get(), Path::new("crates/rakko"));
    }

    // action[verify location.relative]
    #[test]
    fn try_from_str_with_absolute_path_returns_error() {
        let error = DirectoryPath::try_from("/etc").unwrap_err();

        assert_eq!(
            error,
            ParseDirectoryPathError::AbsolutePath {
                path: PathBuf::from("/etc"),
            },
        );
    }

    #[test]
    fn try_from_string_accepts_relative_path() {
        let path = DirectoryPath::try_from("crates/rakko".to_string()).unwrap();

        assert_eq!(path.get(), Path::new("crates/rakko"));
    }

    // action[verify location.relative]
    #[test]
    fn try_from_string_with_absolute_path_returns_error() {
        let error = DirectoryPath::try_from("/etc".to_string()).unwrap_err();

        assert_eq!(
            error,
            ParseDirectoryPathError::AbsolutePath {
                path: PathBuf::from("/etc"),
            },
        );
    }
}
