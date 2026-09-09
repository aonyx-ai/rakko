//! The spelling of a path that an action reports
//!
//! An action reports a location, and that location is a path relative to the
//! root of the project. Whatever produces it compares a path of its own with
//! a path that a program wrote, and the two compare as one path only when
//! both are spelled the way the platform spells a path.
//!
//! Every platform but Windows has one spelling. Windows has two: the one that
//! the file system resolves a path to, which opens with `\\?\`, and the one
//! that a program writes. The standard library answers with the first, so
//! this module answers with the second, and a directory of the project and a
//! file that a tool reported below it then meet.

use std::io::Result as IoResult;
use std::path::{Path, PathBuf};

/// Returns the directory or file that the file system resolves a path to
///
/// The file system resolves the path, so a relative path and a path through a
/// symbolic link both name the file that they point at. The answer carries
/// the spelling that a program of the platform writes, which is what makes it
/// comparable with the paths that an external tool reports.
///
/// # Errors
///
/// Returns the error of the file system when it does not answer for the path,
/// which is what a path that names nothing produces.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::path;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let resolved = path::canonical(".".as_ref())?;
///
/// println!("{}", resolved.display());
/// # Ok(())
/// # }
/// ```
// action[impl path.canonical]
pub fn canonical(path: &Path) -> IoResult<PathBuf> {
    dunce::canonicalize(path)
}

/// Returns the path in the spelling that a program of the platform writes
///
/// The file system of Windows resolves a path to the verbatim spelling, which
/// opens with `\\?\`. A program writes the plain spelling, and the two do not
/// compare as one path, so this drops the prefix. A path that the plain
/// spelling cannot describe, such as one that is too long or that names a
/// device of the platform, keeps the verbatim spelling, which is the spelling
/// that reaches the file behind it.
///
/// A path that the caller did not resolve arrives back unchanged, because
/// only the resolution of the file system produces the verbatim spelling.
/// Every platform but Windows has one spelling, so there the path arrives
/// back unchanged as well.
///
/// # Examples
///
/// ```
/// use std::path::{Path, PathBuf};
///
/// use rakko_action::path;
///
/// let path = path::plain(Path::new("notes.md"));
///
/// assert_eq!(path, PathBuf::from("notes.md"));
/// ```
// action[impl path.plain]
pub fn plain(path: &Path) -> PathBuf {
    dunce::simplified(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]
    // The helpers of this module exist only for tests.
    #![allow(clippy::expect_used)]

    use super::*;

    // action[verify path.canonical]
    #[test]
    fn canonical_of_a_directory_that_exists_names_it() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let resolved = canonical(directory.path()).expect("the directory exists");

        assert!(resolved.is_dir());
    }

    // action[verify path.canonical]
    #[test]
    fn canonical_of_a_path_that_names_nothing_reports_the_error() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let resolved = canonical(&directory.path().join("absent"));

        assert!(resolved.is_err());
    }

    // The paths that a tool reports carry the plain spelling, so a directory
    // that kept the verbatim one would hold none of the files below it.
    // action[verify path.plain]
    #[test]
    fn canonical_names_a_directory_in_the_spelling_that_a_program_writes() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let resolved = canonical(directory.path()).expect("the directory exists");

        assert!(!resolved.to_string_lossy().starts_with(r"\\?\"));
    }

    // action[verify path.plain]
    #[test]
    fn plain_of_a_path_that_nothing_resolved_leaves_it_alone() {
        let path = PathBuf::from("sub").join("notes.md");

        let plain = plain(&path);

        assert_eq!(plain, path);
    }
}
