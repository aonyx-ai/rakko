//! What cargo-udeps reported about a workspace
//!
//! Cargo-udeps ends a run with one JSON document that names every dependency
//! no target loaded. Cargo writes its own records to the same stream, so the
//! report is one line among many, and this module picks it out and reads it.
//! The shape belongs to a version of cargo-udeps, and keeping the reading in
//! one place keeps the version surface in one place as well.

/// One dependency that a package declares and never uses
mod dependency;
/// The error that stops the reading
mod error;
/// The table of a manifest that declares a dependency
mod kind;

use std::collections::BTreeMap;
use std::path::PathBuf;

use getset::Getters;
use serde::Deserialize;
use serde::de::IgnoredAny;

/// The field that only the report of cargo-udeps carries
const UNUSED_DEPS: &str = "unused_deps";

pub use self::dependency::UnusedDependency;
pub use self::error::ReadUdepsReportError;
pub use self::kind::DependencyKind;

/// What cargo-udeps reported about one workspace
///
/// The report holds the dependencies that no target of the workspace loaded,
/// in the order that cargo-udeps named the packages. A caller that ran
/// cargo-udeps with the JSON output reads its standard output into a report
/// and turns each entry into a finding.
///
/// A build that does not finish leaves cargo-udeps without an answer, and it
/// writes no report at all. The reading answers `None` for such a run, so a
/// caller can tell "nothing is unused" from "nobody looked".
///
/// # Examples
///
/// ```
/// use rakko_check_unused_deps::UdepsReport;
///
/// let stdout = r#"{"success":true,"unused_deps":{},"note":null}"#;
///
/// let report = UdepsReport::read(stdout)?;
///
/// assert_eq!(report.map(|report| report.dependencies().len()), Some(0));
/// # Ok::<(), rakko_check_unused_deps::ReadUdepsReportError>(())
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct UdepsReport {
    /// The dependencies that no target of the workspace loaded
    #[getset(get = "pub")]
    dependencies: Vec<UnusedDependency>,
}

impl UdepsReport {
    /// Reads the report of a run from what cargo-udeps wrote to its standard
    /// output
    ///
    /// The reading looks for the one line that names the unused
    /// dependencies. It ignores every other line, because cargo writes the
    /// records of the build to the same stream, and it answers `None` when
    /// no line carries the report.
    ///
    /// # Errors
    ///
    /// Returns [`UnrecognizedReport`][unrecognized] when a line carries the
    /// report and the crate cannot read its body. The shape belongs to a
    /// version of cargo-udeps, and a reading that skipped such a line would
    /// let a workspace pass with its unused dependencies unread.
    ///
    /// [unrecognized]: ReadUdepsReportError::UnrecognizedReport
    // checkunuseddeps[impl check.unreadable]
    pub fn read(stdout: &str) -> Result<Option<Self>, ReadUdepsReportError> {
        let Some(line) = stdout.lines().find(|line| carries_report(line)) else {
            return Ok(None);
        };

        let record: Record = serde_json::from_str(line).map_err(|source| {
            ReadUdepsReportError::UnrecognizedReport {
                line: line.to_owned(),
                source,
            }
        })?;

        Ok(Some(Self {
            dependencies: record.into_dependencies(),
        }))
    }
}

/// Returns whether a line carries the report of cargo-udeps
///
/// Cargo writes its records to the same stream, and none of them names the
/// unused dependencies of a run, so the field that does tells the report
/// apart. The reading looks at the names of the fields and steps over the
/// values, which is what keeps the look cheap on a line of a build.
fn carries_report(line: &str) -> bool {
    serde_json::from_str::<BTreeMap<String, IgnoredAny>>(line)
        .is_ok_and(|fields| fields.contains_key(UNUSED_DEPS))
}

/// The report of cargo-udeps, in the shape that it writes
///
/// Cargo-udeps writes more fields than this one, and the reading ignores the
/// rest, so a field that a new version adds does not break it. Whether the
/// run found anything is the emptiness of the map, so the flag that says so
/// stays behind.
#[derive(Deserialize)]
struct Record {
    /// The packages that declare a dependency they never use, by the
    /// identifier that cargo-udeps names them with
    unused_deps: BTreeMap<String, Package>,
}

/// One package that declares a dependency it never uses
#[derive(Deserialize)]
struct Package {
    /// The manifest of the package
    manifest_path: PathBuf,

    /// The dependencies of `[dependencies]` that no target loaded
    normal: Vec<String>,

    /// The dependencies of `[dev-dependencies]` that no target loaded
    development: Vec<String>,

    /// The dependencies of `[build-dependencies]` that no target loaded
    build: Vec<String>,
}

impl Record {
    /// Returns the unused dependencies of every package of the report
    ///
    /// The packages arrive in the order of their identifiers, and the
    /// dependencies of a package in the order that cargo-udeps named them,
    /// so two runs over the same workspace report the same findings in the
    /// same order.
    // checkunuseddeps[impl check.finding]
    fn into_dependencies(self) -> Vec<UnusedDependency> {
        self.unused_deps
            .into_values()
            .flat_map(Package::into_dependencies)
            .collect()
    }
}

impl Package {
    /// Returns the unused dependencies of this package, by the table that
    /// declares each of them
    // checkunuseddeps[impl check.finding]
    fn into_dependencies(self) -> Vec<UnusedDependency> {
        let manifest = self.manifest_path;
        let tables = [
            (DependencyKind::Normal, self.normal),
            (DependencyKind::Development, self.development),
            (DependencyKind::Build, self.build),
        ];

        tables
            .into_iter()
            .flat_map(|(kind, names)| {
                let manifest = manifest.clone();

                names
                    .into_iter()
                    .map(move |name| UnusedDependency::new(kind, name, manifest.clone()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use super::*;

    /// The report of a run that found one unused dependency of every kind
    const FOUND: &str = r#"{"success":false,"unused_deps":{"probe 0.1.0 (path+file:///project)":{"manifest_path":"/project/Cargo.toml","normal":["serde"],"development":["tempfile"],"build":["cc"]}},"note":"they might be false-positive"}"#;

    /// A record that cargo wrote about the build
    const RECORD: &str = r#"{"reason":"build-finished","success":true}"#;

    // checkunuseddeps[verify check.unreadable]
    #[test]
    fn read_a_report_in_a_shape_the_crate_does_not_know_holds_the_line() {
        let line = r#"{"unused_deps":{"probe":{"manifest_path":5}}}"#;

        let report = UdepsReport::read(line);

        assert!(
            matches!(
                &report,
                Err(ReadUdepsReportError::UnrecognizedReport { line: held, .. })
                    if held == line
            ),
            "expected an unrecognized report, got {report:?}"
        );
    }

    // checkunuseddeps[verify check.unrecognized]
    #[test]
    fn read_a_stream_without_a_report_answers_nothing() {
        let report = UdepsReport::read(RECORD);

        assert_eq!(report.ok(), Some(None));
    }

    // checkunuseddeps[verify check.finding]
    #[test]
    fn read_a_report_beside_the_records_of_cargo_names_every_dependency() {
        let stdout = format!("{RECORD}\n{FOUND}\n");

        let report = UdepsReport::read(&stdout);

        assert_eq!(
            report
                .ok()
                .flatten()
                .map(|report| report.dependencies().to_vec()),
            Some(vec![
                UnusedDependency::new(
                    DependencyKind::Normal,
                    "serde".to_owned(),
                    PathBuf::from("/project/Cargo.toml"),
                ),
                UnusedDependency::new(
                    DependencyKind::Development,
                    "tempfile".to_owned(),
                    PathBuf::from("/project/Cargo.toml"),
                ),
                UnusedDependency::new(
                    DependencyKind::Build,
                    "cc".to_owned(),
                    PathBuf::from("/project/Cargo.toml"),
                ),
            ])
        );
    }

    // checkunuseddeps[verify check.passed]
    #[test]
    fn read_a_report_of_a_clean_run_names_no_dependency() {
        let report = UdepsReport::read(r#"{"success":true,"unused_deps":{},"note":null}"#);

        assert_eq!(
            report
                .ok()
                .flatten()
                .map(|report| report.dependencies().len()),
            Some(0)
        );
    }

    // checkunuseddeps[verify check.finding]
    #[test]
    fn read_a_report_names_the_manifest_of_the_package() {
        let report = UdepsReport::read(FOUND);

        assert_eq!(
            report
                .ok()
                .flatten()
                .map(|report| report.dependencies()[0].manifest().clone()),
            Some(Path::new("/project/Cargo.toml").to_path_buf())
        );
    }
}
