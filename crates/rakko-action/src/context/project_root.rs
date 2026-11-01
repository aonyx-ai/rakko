use std::io::Result as IoResult;
use std::path::Path;

use crate::path;

typed_fields::path! {
    /// The directory where the run of an action happens
    ///
    /// A [`Context`] carries the root, and the [`Layout`] derives its defaults
    /// from it. Every path of the project that an action reads, writes, or
    /// reports is relative to this directory. The working directory of the
    /// process can be another directory. A tool that an action starts with no
    /// directory runs there, and not in the root.
    ///
    /// The root can be the directory that the user edits, or it can be a copy
    /// of the project. So an action must not assume that the root is the
    /// directory that the user edits. A copy holds the files of the project,
    /// but it can lack the files that the project ignores, such as build
    /// outputs and installed packages.
    ///
    /// A path below the root points at the same file in the project, also
    /// when the root is a copy. So a finding whose [`FilePath`] names a file
    /// below the root points the user at the correct file.
    ///
    /// [`Context`]: crate::Context
    /// [`FilePath`]: crate::FilePath
    /// [`Layout`]: crate::Layout
    ProjectRoot
}

impl ProjectRoot {
    /// Returns the root at a path, as the file system names the directory
    ///
    /// The file system resolves the path, so a relative path and a path
    /// through a symbolic link both reach an action as the directory that
    /// they name. A caller that has a path instead of a root uses this rather
    /// than the plain constructor: an action reports every location relative
    /// to the root, and a root that spells a directory differently from the
    /// programs that the action starts leaves every path they report outside
    /// the root.
    ///
    /// # Errors
    ///
    /// Returns the error of the file system when it does not answer for the
    /// path, which is what a path that names no directory produces.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_action::ProjectRoot;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let root = ProjectRoot::canonical(".")?;
    ///
    /// println!("{}", root.get().display());
    /// # Ok(())
    /// # }
    /// ```
    pub fn canonical(path: impl AsRef<Path>) -> IoResult<Self> {
        Ok(Self::new(path::canonical(path.as_ref())?))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]
    // The helpers of this module exist only for tests.
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn canonical_of_a_directory_that_exists_names_it() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let root = ProjectRoot::canonical(directory.path()).expect("the directory exists");

        assert!(root.get().is_dir());
    }

    #[test]
    fn canonical_of_a_directory_that_does_not_exist_reports_the_error() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let root = ProjectRoot::canonical(directory.path().join("absent"));

        assert!(root.is_err());
    }
}
