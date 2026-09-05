use serde::Deserialize;

use crate::problem::{DenyProblem, Package, Severity};

/// One record of the report that cargo-deny wrote
///
/// Cargo-deny writes a line of JSON per record, and the records of one run
/// answer different questions. This is what one line means to a caller, with
/// everything that the caller does not read left behind.
pub(super) enum Entry {
    /// Cargo-deny recognized a shape in the dependencies of the workspace
    Problem(DenyProblem),

    /// Cargo-deny finished its checks and counted what they found
    Summary,

    /// A record that this crate does not read
    ///
    /// Cargo-deny logs what it is doing on the same stream as the report, and
    /// the log says nothing about the project that a finding could carry.
    Ignored,
}

/// One record of the report, in the shape that cargo-deny writes it
///
/// Cargo-deny writes more fields than these, and the reading ignores the
/// rest, so a field that a new version adds does not break it. A record of a
/// kind that this crate does not know is ignored rather than refused: the
/// records that a run answers from are the two below, and a new kind next to
/// them says nothing about the workspace.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Record {
    /// One shape that cargo-deny recognized
    Diagnostic {
        /// What cargo-deny recognized, and how the project weighed it
        fields: Diagnostic,
    },

    /// The counts that cargo-deny ends a run of its checks with
    Summary {},

    /// A record of a kind that this crate does not read
    #[serde(other)]
    Other,
}

/// One shape that cargo-deny recognized in the dependencies of a workspace
#[derive(Deserialize)]
struct Diagnostic {
    /// The check that recognized the shape
    code: String,

    /// How the configuration of the project weighed the shape
    severity: Severity,

    /// What cargo-deny wrote about the shape
    message: String,

    /// The inclusion graph of every package that the shape is about
    ///
    /// A record that is about no package of the graph, such as a license that
    /// the configuration allows and no dependency carries, holds no graph at
    /// all.
    #[serde(default)]
    graphs: Vec<Graph>,
}

/// The inclusion graph of one package that a shape is about
///
/// The graph names the package at its root and then the packages that depend
/// on it, up to the workspace. The reading takes the root and leaves the rest,
/// because a finding names what the answer is about and not how the project
/// arrived at it.
#[derive(Deserialize)]
struct Graph {
    /// The package that the graph is rooted at
    #[serde(rename = "Krate")]
    package: Krate,
}

/// One package, in the shape that cargo-deny writes it
#[derive(Deserialize)]
struct Krate {
    /// The name of the package
    name: String,

    /// The version of the package
    version: String,
}

impl Diagnostic {
    /// Returns the problem that this record reports
    // checkdependencies[impl check.finding]
    fn into_problem(self) -> DenyProblem {
        let packages = self
            .graphs
            .into_iter()
            .map(|graph| Package::new(graph.package.name, graph.package.version))
            .collect();

        DenyProblem::new(self.severity, self.code, self.message, packages)
    }
}

/// Returns what one record of the report means to a caller
///
/// # Errors
///
/// Returns the error of a record that is not one that cargo-deny writes. A
/// reading that skipped such a record would drop a problem of the project
/// without a word, so the caller stops instead.
// checkdependencies[impl check.unreadable]
pub(super) fn read(record: &str) -> Result<Entry, serde_json::Error> {
    Ok(match serde_json::from_str::<Record>(record)? {
        Record::Diagnostic { fields } => Entry::Problem(fields.into_problem()),
        Record::Summary {} => Entry::Summary,
        Record::Other => Entry::Ignored,
    })
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that reads a record
    // which cargo-deny could have written expects the reading to succeed. A
    // `# Panics` section on every test would repeat that and give the reader
    // no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A record about a license that the configuration of a project rejects
    const REJECTED: &str = r#"{"fields":{"code":"rejected","graphs":[{"Krate":{"name":"option-ext","version":"0.2.0"},"parents":[{"Krate":{"name":"dirs-sys","version":"0.5.0"}}]}],"labels":[{"column":12,"line":21,"message":"rejected: license is not explicitly allowed","span":"MPL-2.0"}],"message":"failed to satisfy license requirements","notes":["MPL-2.0 - Mozilla Public License 2.0:"],"severity":"error"},"type":"diagnostic"}"#;

    /// A record about two versions of one package in the graph
    const DUPLICATE: &str = r#"{"fields":{"code":"duplicate","graphs":[{"Krate":{"name":"syn","version":"2.0.119"}},{"Krate":{"name":"syn","version":"3.0.4"}}],"labels":[{"column":1,"line":86,"message":"lock entries","span":"syn 2.0.119"}],"message":"found 2 duplicate entries for crate 'syn'","severity":"warning"},"type":"diagnostic"}"#;

    /// A record about a license that no dependency of the project carries
    const UNENCOUNTERED: &str = r#"{"fields":{"code":"license-not-encountered","labels":[],"message":"license was not encountered","severity":"warning"},"type":"diagnostic"}"#;

    /// The record that cargo-deny ends a run of its checks with
    const SUMMARY: &str = r#"{"fields":{"bans":{"errors":0,"helps":0,"notes":0,"warnings":1},"licenses":{"errors":0,"helps":101,"notes":0,"warnings":0},"sources":{"errors":0,"helps":0,"notes":0,"warnings":0}},"type":"summary"}"#;

    /// A record that cargo-deny writes about what it is doing
    const LOG: &str = r#"{"fields":{"level":"WARN","message":"unable to find a config path, falling back to default config","timestamp":"2026-09-05T11:21:17.220970Z"},"type":"log"}"#;

    /// A record of a kind that this crate does not know
    const UNKNOWN: &str = r#"{"fields":{"count":3},"type":"progress"}"#;

    /// Returns the problem that a record of a diagnostic reports
    fn problem(record: &str) -> DenyProblem {
        let entry = read(record).expect("the test reads a record that cargo-deny could write");

        let Entry::Problem(problem) = entry else {
            panic!("expected the record to report a problem");
        };

        problem
    }

    // checkdependencies[verify check.finding]
    #[test]
    fn read_a_diagnostic_holds_the_check_and_the_words_of_cargo_deny() {
        let problem = problem(REJECTED);

        assert_eq!(
            problem.description(),
            "[rejected] failed to satisfy license requirements (option-ext 0.2.0)"
        );
    }

    // checkdependencies[verify check.finding]
    #[test]
    fn read_a_diagnostic_names_the_package_at_the_root_of_every_graph() {
        let problem = problem(DUPLICATE);

        assert_eq!(
            problem
                .packages()
                .iter()
                .map(Package::to_string)
                .collect::<Vec<_>>(),
            ["syn 2.0.119", "syn 3.0.4"]
        );
    }

    // checkdependencies[verify check.finding]
    #[test]
    fn read_a_diagnostic_without_a_graph_names_no_package() {
        let problem = problem(UNENCOUNTERED);

        assert!(
            problem.packages().is_empty(),
            "expected no package, got {:?}",
            problem.packages()
        );
    }

    // checkdependencies[verify check.warning]
    #[test]
    fn read_a_diagnostic_carries_the_weight_of_the_project() {
        let problem = problem(DUPLICATE);

        assert_eq!(problem.severity(), Severity::Warning);
    }

    // checkdependencies[verify check.incomplete]
    #[test]
    fn read_the_summary_reports_a_run_that_finished_its_checks() {
        let entry = read(SUMMARY).expect("the test reads a record that cargo-deny could write");

        assert!(
            matches!(entry, Entry::Summary),
            "expected the summary of the checks"
        );
    }

    #[test]
    fn read_a_log_record_reports_nothing_about_the_project() {
        let entry = read(LOG).expect("the test reads a record that cargo-deny could write");

        assert!(
            matches!(entry, Entry::Ignored),
            "expected the log to be left"
        );
    }

    #[test]
    fn read_a_record_of_an_unknown_kind_reports_nothing_about_the_project() {
        let entry = read(UNKNOWN).expect("the test reads a record that cargo-deny could write");

        assert!(
            matches!(entry, Entry::Ignored),
            "expected the record to be left"
        );
    }

    // checkdependencies[verify check.unreadable]
    #[test]
    fn read_a_diagnostic_in_a_shape_the_crate_does_not_know_stops_the_reading() {
        let entry = read(r#"{"fields":{"code":"rejected"},"type":"diagnostic"}"#);

        assert!(
            entry.is_err(),
            "expected the reading to stop, got a record that the crate cannot answer from"
        );
    }
}
