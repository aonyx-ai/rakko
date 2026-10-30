use std::io::Result as IoResult;
use std::path::Path;

use crate::path;

typed_fields::path! {
    /// The directory where a run of an action happens
    ///
    /// An action reads its files, starts its tools, and makes its changes in
    /// this directory. A [`Context`] carries the root, and the [`Layout`]
    /// derives its defaults from it.
    ///
    /// The root can be a copy of the project, so an action must not assume
    /// that the root is the tree that the user edits. A copy holds the files
    /// of the project, but it can lack the files that the project ignores,
    /// such as build outputs and installed packages. An action must not assume
    /// that such a file exists under the root.
    ///
    /// Every path that an action reads, writes, or reports is relative to the
    /// root. A relative path names the same file in a copy and in the project,
    /// so a finding points at the file that the user edits, also when the run
    /// happens in a copy.
    ///
    /// [`Context`]: crate::Context
    /// [`Layout`]: crate::Layout
    ProjectRoot
}

impl ProjectRoot {
    /// Returns the root of a project, as the file system names the directory
    ///
    /// The file system resolves the path, so a relative path and a path
    /// through a symbolic link both reach an action as the directory that
    /// they name. A caller that has a path instead of a root uses this rather
    /// than the plain constructor: an action reports every location relative
    /// to the root, and a root that spells a directory differently from the
    /// programs that the action starts leaves every path they report outside
    /// the project.
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
