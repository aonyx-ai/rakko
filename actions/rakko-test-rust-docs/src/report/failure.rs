use std::path::PathBuf;

use getset::Getters;
use rakko_action::{Finding, Location, Position, ProjectRoot};
use rakko_cargo::CargoRoot;

/// The words that the test harness of Rust writes before the location of a
/// panic
const PANICKED_AT: &str = "panicked at ";

/// The line that the test harness of Rust writes after the message of a
/// panic, which is not part of the message
const BACKTRACE_NOTE: &str = "note: run with `RUST_BACKTRACE=1`";

/// The words that the harness writes before the line of an example, and the
/// closing bracket that follows the line
const LINE_OPENS: &str = " (line ";

/// The word that the harness writes between the file of an example and the
/// item that the example documents
const ITEM_OPENS: &str = " - ";

/// One example of the documentation that failed
///
/// The harness names an example by the file that documents it and the line
/// that the example starts on, and it keeps what the example wrote: the
/// diagnostics of the compiler for an example that does not compile, and the
/// panic for an example that ran and failed. The failure keeps the output as
/// it is and reads it on demand, so an example that wrote nothing the crate
/// knows still names the example.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct DoctestFailure {
    /// The name of the example, as the harness wrote it
    #[getset(get = "pub")]
    name: String,

    /// What the example wrote while it compiled and ran
    #[getset(get = "pub")]
    output: String,
}

impl DoctestFailure {
    /// Creates a failure from the name of the example and its output
    pub fn new(name: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: output.into(),
        }
    }

    /// Returns the finding that reports the failure
    ///
    /// The finding names the example and carries what the example wrote, at
    /// the line that documents it, with the path relative to the project
    /// root.
    ///
    /// An example whose name carries no line, and an example in a file
    /// outside the project, get a finding at the level of the project.
    // testrustdocs[impl finding.failed]
    // testrustdocs[impl finding.position]
    pub fn finding(&self, root: &CargoRoot, project: &ProjectRoot) -> Finding {
        let message = match self.diagnosis() {
            Some(diagnosis) => format!("example `{}` failed: {diagnosis}", self.name),
            None => format!("example `{}` failed", self.name),
        };

        Finding::builder()
            .message(message)
            .location(self.location(root, project))
            .build()
    }

    /// Returns what the example wrote about its failure, in one line
    ///
    /// An example fails in two ways. The compiler refuses it, and the output
    /// opens with what the compiler wrote about it. Or the example ran and
    /// panicked, and the harness wrote the location of the panic and its
    /// message. The reading takes the message of the panic when the output
    /// names one, and the first line of the output otherwise.
    // testrustdocs[impl finding.failed]
    fn diagnosis(&self) -> Option<String> {
        let mut lines = self.output.lines();

        if lines.any(|line| line.contains(PANICKED_AT)) {
            let message: Vec<&str> = lines
                .map(str::trim)
                .take_while(|line| !line.starts_with(BACKTRACE_NOTE))
                .filter(|line| !line.is_empty())
                .collect();

            if !message.is_empty() {
                return Some(message.join(" "));
            }
        }

        self.output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(ToOwned::to_owned)
    }

    /// Returns where the example that failed is documented
    ///
    /// The place where an example panicked is not that place: the compiler
    /// builds the examples of a workspace into one program, and the panic
    /// then names a file of that program. The name of the example names the
    /// file of the project instead, relative to the workspace that cargo
    /// tested.
    // testrustdocs[impl finding.position]
    fn location(&self, root: &CargoRoot, project: &ProjectRoot) -> Location {
        let Some((path, line)) = documented(&self.name) else {
            return Location::Project;
        };

        match root.relative_path(&path, project) {
            Some(path) => Location::Position {
                path,
                position: Position::builder().line(line).build(),
            },
            None => Location::Project,
        }
    }
}

/// Returns the file and the line that the name of an example carries
///
/// A name reads `file - item (line 5)`, and the item is empty for an example
/// in the documentation of a crate. The reading starts from the end, because
/// a file can hold the words that open the item of another name.
// testrustdocs[impl finding.position]
fn documented(name: &str) -> Option<(PathBuf, u32)> {
    let (documenting, line) = name.rsplit_once(LINE_OPENS)?;
    let line = line.strip_suffix(')')?.parse().ok()?;
    let path = match documenting.rsplit_once(ITEM_OPENS) {
        Some((path, _)) => path,
        None => documenting.trim_end().trim_end_matches('-').trim_end(),
    };

    (!path.is_empty()).then(|| (PathBuf::from(path), line))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::FilePath;
    use rakko_cargo::Documentation;

    use super::*;

    /// What an example that the compiler refused wrote
    const REFUSED: &str = "error[E0425]: cannot find function `nope` in crate `probe`\n  --> src/lib.rs:16:21\n   |\n16 | let x: i32 = probe::nope();\n   |                     ^^^^ not found in `probe`\n\nerror: aborting due to 1 previous error\n\nFor more information about this error, try `rustc --explain E0425`.\nCouldn't compile the test.\n";

    /// What an example that ran and panicked wrote
    const PANICKED: &str = "Test executable failed (exit status: 101).\n\nstderr:\n\nthread 'main' (15406046) panicked at /tmp/rustdoctest/doctest_bundle_2024.rs:6:1:\nassertion `left == right` failed\n  left: 3\n right: 4\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n\n";

    /// The root of the workspace that a test reports a failure of
    fn root() -> CargoRoot {
        CargoRoot::builder()
            .directory(PathBuf::from("/home/otter/project/crates"))
            .documentation(Documentation::Testable)
            .build()
    }

    /// The root of the project that a test reports a failure of
    fn project() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    // testrustdocs[verify finding.failed]
    #[test]
    fn finding_of_a_refused_example_carries_what_the_compiler_wrote() {
        let failure = DoctestFailure::new("probe/src/lib.rs - sub (line 14)", REFUSED);

        let finding = failure.finding(&root(), &project());

        assert_eq!(
            finding.message().get(),
            "example `probe/src/lib.rs - sub (line 14)` failed: error[E0425]: cannot find function `nope` in crate `probe`"
        );
    }

    // testrustdocs[verify finding.failed]
    #[test]
    fn finding_of_an_example_that_panicked_carries_the_message() {
        let failure = DoctestFailure::new("probe/src/lib.rs - add (line 5)", PANICKED);

        let finding = failure.finding(&root(), &project());

        assert_eq!(
            finding.message().get(),
            "example `probe/src/lib.rs - add (line 5)` failed: assertion `left == right` failed left: 3 right: 4"
        );
    }

    // testrustdocs[verify finding.failed]
    #[test]
    fn finding_of_an_example_that_wrote_nothing_names_it() {
        let failure = DoctestFailure::new("probe/src/lib.rs - add (line 5)", "");

        let finding = failure.finding(&root(), &project());

        assert_eq!(
            finding.message().get(),
            "example `probe/src/lib.rs - add (line 5)` failed"
        );
    }

    // testrustdocs[verify finding.position]
    #[test]
    fn finding_of_an_example_is_at_the_line_that_documents_it() {
        let failure = DoctestFailure::new("probe/src/lib.rs - add (line 5)", PANICKED);

        let finding = failure.finding(&root(), &project());

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: FilePath::try_from("crates/probe/src/lib.rs").unwrap(),
                position: Position::builder().line(5).build(),
            }
        );
    }

    // testrustdocs[verify finding.position]
    #[test]
    fn finding_of_an_example_of_a_crate_reads_the_file_without_an_item() {
        let failure = DoctestFailure::new("probe/src/lib.rs - (line 3)", PANICKED);

        let finding = failure.finding(&root(), &project());

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: FilePath::try_from("crates/probe/src/lib.rs").unwrap(),
                position: Position::builder().line(3).build(),
            }
        );
    }

    // testrustdocs[verify finding.position]
    #[test]
    fn finding_of_an_example_whose_name_carries_no_line_is_at_the_project() {
        let failure = DoctestFailure::new("probe/src/lib.rs - add", PANICKED);

        let finding = failure.finding(&root(), &project());

        assert_eq!(finding.location(), &Location::Project);
    }

    // testrustdocs[verify finding.position]
    #[test]
    fn finding_of_an_example_outside_the_project_is_at_the_project() {
        let failure = DoctestFailure::new("/elsewhere/src/lib.rs - add (line 5)", PANICKED);

        let finding = failure.finding(&root(), &project());

        assert_eq!(finding.location(), &Location::Project);
    }
}
