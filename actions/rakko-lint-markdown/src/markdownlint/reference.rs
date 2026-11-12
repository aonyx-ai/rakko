//! The reference that makes markdownlint report a configuration it rejects
//!
//! Markdownlint looks for its configuration in four files of the directory
//! where it starts. It takes the first one that exists and that it can parse.
//! It treats a file that it cannot parse as a file that is not there. It
//! takes the next file name, or it runs on its defaults, and it reports
//! nothing on either stream.
//!
//! Markdownlint reports a configuration that it cannot parse only for the file
//! that its `--config` option gives. That option does not suit the file of the
//! project. Markdownlint first parses the file of the option as JSON with
//! comments or as INI. That parser stops on a JSONC file that
//! markdownlint accepts on its own, such as one with a trailing comma.
//!
//! So the option gives a temporary file instead, which holds only one key.
//! The key tells markdownlint to extend the configuration file of the
//! project. Markdownlint then parses that file with its own parsers. It
//! applies the same configuration that it finds without the option.

use std::collections::BTreeMap;
use std::io::{Error as IoError, Write};
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

/// The files that markdownlint reads its configuration from, in its order
const CONFIGURATION_FILES: [&str; 4] = [
    ".markdownlint.jsonc",
    ".markdownlint.json",
    ".markdownlint.yaml",
    ".markdownlint.yml",
];

/// The key that tells markdownlint to read another configuration file
const EXTENDS: &str = "extends";

/// The start of what markdownlint writes about a configuration it rejects
///
/// The path of the file that the option gives follows, in single quotes, and
/// then a colon, a space, and the message of the parsers.
const REJECTION: &str = "Cannot read or parse config file '";

/// The start of the report that follows the message of a rejection
///
/// Markdownlint writes the report on the next line, as a JSON array whose
/// first line holds only the bracket.
const REPORT: &str = "\n[\n";

/// A temporary file that refers markdownlint to the configuration of a project
///
/// The file exists as long as the value, so a run of markdownlint must end
/// before the value goes out of scope.
#[derive(Debug)]
pub(crate) struct Reference {
    /// The temporary file that holds the reference
    file: NamedTempFile,

    /// The configuration file that the reference points at, relative to the
    /// root of the project
    configuration: PathBuf,
}

impl Reference {
    /// Returns the reference to the configuration file of a project
    ///
    /// The configuration file is the first of the four names of markdownlint
    /// that exists in the root. Markdownlint takes a later one only when it
    /// cannot parse an earlier one, so a later file does not hide that
    /// failure. A project without one of these files has no configuration for
    /// the reference to point at, and the result is `None`. Markdownlint still
    /// reads the other places that it knows, such as a `.markdownlintrc`.
    ///
    /// # Errors
    ///
    /// Returns the error of the file system when the temporary file cannot be
    /// written. Returns an error when the root is relative and the current
    /// directory of the process is unknown. Returns an error when the path of
    /// the configuration file is not valid Unicode, because JSON can hold only
    /// such a path.
    pub(crate) fn to_configuration_of(root: &Path) -> Result<Option<Self>, IoError> {
        let Some(name) = CONFIGURATION_FILES
            .iter()
            .find(|name| root.join(name).exists())
        else {
            return Ok(None);
        };

        let target = std::path::absolute(root.join(name))?;

        let mut file = NamedTempFile::new()?;
        serde_json::to_writer(&mut file, &BTreeMap::from([(EXTENDS, target)]))?;
        file.flush()?;

        Ok(Some(Self {
            file,
            configuration: PathBuf::from(name),
        }))
    }

    /// Returns the configuration file, relative to the root of the project
    pub(crate) fn configuration(&self) -> &Path {
        &self.configuration
    }

    /// Returns the path of the temporary file, for the option of markdownlint
    pub(crate) fn path(&self) -> &Path {
        self.file.path()
    }

    /// Returns what markdownlint wrote about a configuration that it rejected
    ///
    /// The text is the message of the parsers of markdownlint, without the
    /// path of the temporary file and without the report that follows it.
    /// The message gives the file that markdownlint cannot parse. This is
    /// the configuration file of the project, or a file that it extends. The
    /// result is `None` when markdownlint accepted the configuration.
    pub(crate) fn rejection(&self, stderr: &str) -> Option<String> {
        let start = format!("{REJECTION}{}': ", self.path().display());
        let (_, message) = stderr.split_once(&start)?;
        let message = message
            .split_once(REPORT)
            .map_or(message, |(message, _)| message);

        Some(message.trim_end().to_owned())
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "An assertion in a test panics by design, and the helpers of this module exist only for tests. A `# Panics` section on every test would repeat that and give the reader no information."
    )]
    #![allow(clippy::missing_panics_doc)]

    use tempfile::TempDir;

    use super::*;

    /// What markdownlint wrote about a configuration that it rejected
    const MESSAGE: &str = "Unable to parse '/project/.markdownlint.yaml'; Parser 0: bad";

    /// Returns a root that holds the given configuration files, each empty
    fn root(files: &[&str]) -> TempDir {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        for file in files {
            std::fs::write(directory.path().join(file), "")
                .expect("the test writes a configuration file");
        }

        directory
    }

    /// Returns the reference to the configuration of a root that has one
    fn reference(root: &TempDir) -> Reference {
        Reference::to_configuration_of(root.path())
            .expect("the test writes a reference")
            .expect("the test root holds a configuration file")
    }

    #[test]
    fn reference_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Reference>();
    }

    #[test]
    fn rejection_of_a_rejection_before_a_report_leaves_the_report_out() {
        let root = root(&[".markdownlint.yaml"]);
        let reference = reference(&root);
        let stderr = format!(
            "{REJECTION}{}': {MESSAGE}\n[\n  {{\n    \"fileName\": \"notes.md\"\n  }}\n]\n",
            reference.path().display()
        );

        let rejection = reference.rejection(&stderr);

        assert_eq!(rejection.as_deref(), Some(MESSAGE));
    }

    #[test]
    fn rejection_of_a_rejection_without_a_report_is_the_message() {
        let root = root(&[".markdownlint.yaml"]);
        let reference = reference(&root);
        let stderr = format!("{REJECTION}{}': {MESSAGE}\n", reference.path().display());

        let rejection = reference.rejection(&stderr);

        assert_eq!(rejection.as_deref(), Some(MESSAGE));
    }

    #[test]
    fn rejection_of_a_report_without_a_rejection_is_none() {
        let root = root(&[".markdownlint.yaml"]);
        let reference = reference(&root);

        let rejection = reference.rejection("[\n  {\n    \"fileName\": \"notes.md\"\n  }\n]\n");

        assert_eq!(rejection, None);
    }

    #[test]
    fn rejection_of_another_configuration_file_is_none() {
        let root = root(&[".markdownlint.yaml"]);
        let reference = reference(&root);
        let stderr = format!("{REJECTION}/elsewhere/config.json': {MESSAGE}\n");

        let rejection = reference.rejection(&stderr);

        assert_eq!(rejection, None);
    }

    #[test]
    fn to_configuration_of_a_root_with_a_configuration_extends_it() {
        let root = root(&[".markdownlint.yaml"]);
        let reference = reference(&root);

        let content = std::fs::read_to_string(reference.path())
            .expect("the test reads the reference that it wrote");

        let expected = std::path::absolute(root.path().join(".markdownlint.yaml"))
            .expect("the test gives a path below an absolute directory");
        assert_eq!(
            serde_json::from_str::<BTreeMap<String, PathBuf>>(&content)
                .expect("the reference holds a JSON object of paths"),
            BTreeMap::from([("extends".to_owned(), expected)])
        );
    }

    #[test]
    fn to_configuration_of_a_root_with_every_configuration_takes_the_first_name() {
        let root = root(&CONFIGURATION_FILES);

        let reference = reference(&root);

        assert_eq!(reference.configuration(), Path::new(".markdownlint.jsonc"));
    }

    #[test]
    fn to_configuration_of_a_root_with_the_last_configuration_takes_it() {
        let root = root(&[".markdownlint.yml"]);

        let reference = reference(&root);

        assert_eq!(reference.configuration(), Path::new(".markdownlint.yml"));
    }

    #[test]
    fn to_configuration_of_a_root_without_a_configuration_is_none() {
        let root = root(&[".markdownlintrc"]);

        let reference =
            Reference::to_configuration_of(root.path()).expect("the test writes no reference");

        assert!(
            reference.is_none(),
            "expected no reference, got {reference:?}"
        );
    }
}
