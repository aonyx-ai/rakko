use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// The error type for file path parsing
mod error;
pub use error::ParseFilePathError;

/// The path of a file that a problem is in
///
/// The path is relative to the project root. A reader, a machine, and a code
/// host therefore see the same path for the same file, and none of them has to
/// know where the project lives on disk.
///
/// Construct a file path through [`FromStr`], [`TryFrom<&str>`],
/// [`TryFrom<String>`], [`TryFrom<&Path>`], or [`TryFrom<PathBuf>`]. Every
/// constructor refuses an absolute path and returns a [`ParseFilePathError`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FilePath(PathBuf);

impl FilePath {
    /// Returns the path of the file
    pub fn get(&self) -> &Path {
        &self.0
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

    use super::*;

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
        let error = "/etc/hosts".parse::<FilePath>().unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from("/etc/hosts"),
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
        let error = FilePath::try_from(PathBuf::from("/etc/hosts")).unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from("/etc/hosts"),
            },
        );
    }

    // action[verify location.relative]
    #[test]
    fn try_from_path_with_absolute_path_returns_error() {
        let error = FilePath::try_from(Path::new("/etc/hosts")).unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from("/etc/hosts"),
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
        let error = FilePath::try_from("/etc/hosts").unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from("/etc/hosts"),
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
        let error = FilePath::try_from("/etc/hosts".to_string()).unwrap_err();

        assert_eq!(
            error,
            ParseFilePathError::AbsolutePath {
                path: PathBuf::from("/etc/hosts"),
            },
        );
    }
}
