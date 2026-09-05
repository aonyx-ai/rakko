use std::path::PathBuf;

use getset::Getters;
use rakko_action::{Finding, Location, Position, ProjectRoot};
use rakko_cargo::CargoRoot;

use super::Panic;

/// The words that the test harness of Rust writes before the location of a
/// panic
const PANICKED_AT: &str = "panicked at ";

/// The line that the test harness of Rust writes after the message of a
/// panic, which is not part of the message
const BACKTRACE_NOTE: &str = "note: run with `RUST_BACKTRACE=1`";

/// One test that failed
///
/// Nextest names the test and keeps what the test wrote, and the test
/// harness of Rust writes the location and the message of a panic there. The
/// failure keeps the output as it is and reads the panic out of it on
/// demand, so a test that failed without a panic, or with an output that
/// the crate cannot read, still names the test.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct TestFailure {
    /// The name of the test, as nextest wrote it
    #[getset(get = "pub")]
    name: String,

    /// What the test wrote while it ran
    #[getset(get = "pub")]
    output: String,
}

impl TestFailure {
    /// Creates a failure from the name of the test and its output
    pub fn new(name: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            output: output.into(),
        }
    }

    /// Returns the finding that reports the failure
    ///
    /// The finding names the test and carries the message of the panic, at
    /// the position where the test panicked, with the path relative to the
    /// project root.
    ///
    /// A test whose output names no panic, and a test that panicked in a
    /// file outside the project, get a finding at the level of the project.
    // nextest[impl finding.failed]
    // nextest[impl finding.position]
    pub fn finding(&self, root: &CargoRoot, project: &ProjectRoot) -> Finding {
        let Some(panic) = self.panic() else {
            return Finding::builder()
                .message(format!("test `{}` failed", self.name))
                .location(Location::Project)
                .build();
        };

        let message = match panic.message() {
            Some(message) => format!("test `{}` failed: {message}", self.name),
            None => format!("test `{}` panicked", self.name),
        };

        let location = match root.relative_path(panic.path(), project) {
            Some(path) => Location::Position {
                path,
                position: Position::builder()
                    .line(panic.line())
                    .column(panic.column())
                    .build(),
            },
            None => Location::Project,
        };

        Finding::builder()
            .message(message)
            .location(location)
            .build()
    }

    /// Returns where and why the test panicked, when its output says
    ///
    /// The test harness of Rust writes one line that names the location,
    /// and the message of the panic follows on the lines below it, up to
    /// the note about the backtrace. A test that ended another way, such as
    /// one that the process killed, wrote no such line, and the method
    /// answers `None`.
    // nextest[impl finding.failed]
    // nextest[impl finding.position]
    pub fn panic(&self) -> Option<Panic> {
        let mut lines = self.output.lines();
        let header = lines.find(|line| line.contains(PANICKED_AT))?;
        let location = header
            .split_once(PANICKED_AT)?
            .1
            .trim()
            .trim_end_matches(':');
        let (path, line, column) = coordinates(location)?;

        let message: Vec<&str> = lines
            .map(str::trim)
            .take_while(|line| !line.starts_with(BACKTRACE_NOTE))
            .filter(|line| !line.is_empty())
            .collect();

        Some(Panic::new(
            path,
            line,
            column,
            (!message.is_empty()).then(|| message.join(" ")),
        ))
    }
}

/// Returns the path, the line, and the column of a location
///
/// A location reads `path:line:column`, and a path can hold a colon of its
/// own, so the reading starts from the end.
fn coordinates(location: &str) -> Option<(PathBuf, u32, u32)> {
    let mut parts = location.rsplitn(3, ':');
    let column = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
    let path = parts.next()?;

    (!path.is_empty()).then(|| (PathBuf::from(path), line, column))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// What a test that failed an assertion wrote
    const ASSERTION: &str = "\nthread 'fails' (149446493) panicked at tests/suite.rs:8:5:\nassertion `left == right` failed: the probe fails on purpose\n  left: 1\n right: 2\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n";

    /// What a test that panicked without a message wrote
    const BARE: &str = "thread 'fails' panicked at src/lib.rs:6:9:\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n";

    // nextest[verify finding.failed]
    #[test]
    fn panic_joins_the_lines_of_the_message() {
        let failure = TestFailure::new("probe::suite$fails", ASSERTION);

        let panic = failure.panic();

        assert_eq!(
            panic.and_then(|panic| panic.message().clone()),
            Some(
                "assertion `left == right` failed: the probe fails on purpose left: 1 right: 2"
                    .to_owned()
            )
        );
    }

    // nextest[verify finding.position]
    #[test]
    fn panic_reads_the_location() {
        let failure = TestFailure::new("probe::suite$fails", ASSERTION);

        let panic = failure.panic();

        assert_eq!(
            panic.map(|panic| (panic.path().clone(), panic.line(), panic.column())),
            Some((PathBuf::from("tests/suite.rs"), 8, 5))
        );
    }

    // nextest[verify finding.failed]
    #[test]
    fn panic_without_a_message_has_none() {
        let failure = TestFailure::new("probe::probe$tests::fails", BARE);

        let panic = failure.panic();

        assert_eq!(panic.and_then(|panic| panic.message().clone()), None);
    }

    // nextest[verify finding.position]
    #[test]
    fn panic_without_a_location_is_none() {
        let failure = TestFailure::new("probe::suite$killed", "the process was killed\n");

        let panic = failure.panic();

        assert_eq!(panic, None);
    }
}
