//! The reading of the log that the validator wrote
//!
//! The validator writes one JSON record per line on request. Most records are
//! the log of the run, and some of them carry the answer: the configurations
//! that the validator read, the problems that it found in them, and the note
//! that it found no configuration at all. This module turns the records into
//! that answer, and it judges nothing: the caller decides what the answer
//! means for the outcome of a run.

/// The error that leaves a caller without the answer of a run
mod error;

use std::collections::BTreeSet;
use std::path::PathBuf;

use rakko_action::Position;
use serde::Deserialize;
use serde_json::{Map, Value};

pub use self::error::ReadReportError;
use crate::observation::Observation;
use crate::problem::RenovateProblem;

/// The level of a record that reports a warning
///
/// The log gives each record a number for its level: 30 for information, 40
/// for a warning, 50 for an error, and 60 for a fatal record. A problem of a
/// configuration arrives as a warning or as an error, and a record below this
/// level is the log of the run.
const WARNING: u32 = 40;

/// The level of a record that the validator writes before it stops
const FATAL: u32 = 60;

/// The text that starts the record that announces a configuration
///
/// The validator writes it before it reads a configuration, and the rest of
/// the text names the configuration in the same words as the problems of it.
const ANNOUNCEMENT: &str = "Validating ";

/// The text that starts the record that counts what a passing run validated
///
/// The validator writes it at the end of a run that found no problem, with the
/// number of configurations that it validated between this text and
/// [`COUNT_CLOSE`]. A key of `package.json` that holds several presets counts
/// once for each of them, so the number is the answer of the validator and not
/// a count of its announcements.
const COUNT_OPEN: &str = "Config validated successfully against ";

/// The text that ends the record that counts what a passing run validated
const COUNT_CLOSE: &str = " file(s)";

/// The text that separates a file from the key that holds the configuration
///
/// A configuration can live in a key of `package.json`, and the validator
/// names it as the file, this text, and the key.
const KEY_SEPARATOR: &str = " > ";

/// The text that starts the record that draws a migration for a reader
///
/// The record repeats the migration that the record before it reports, as the
/// difference between the two configurations, so the reading leaves it out.
const MIGRATION_DRAWING: &str = "Config migration diff:";

/// What the validator writes when it found no configuration to validate
const UNCONFIGURED: &str = "No files to perform configuration validation against";

/// The text between the two parts of the details of a fatal record
const CAUSE_OPEN: &str = ": ";

/// The text between two options that a migration changes
const OPTION_SEPARATOR: &str = ", ";

/// The answer that the log of a run holds
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Report {
    /// The validator read every configuration that it found
    Finished(Observation),

    /// The validator wrote a fatal record and stopped
    ///
    /// The validator stops when it cannot read the global configuration that
    /// a run of Renovate would start with, before it reads a configuration of
    /// the project.
    Halted {
        /// What the validator wrote in the fatal record
        details: String,
    },
}

/// One record of the log, with the fields that the reading needs
///
/// The validator writes more than these fields, and the reading ignores the
/// rest, so a field that a new version adds does not break it.
#[derive(Deserialize)]
struct Record {
    /// The level of the record
    level: u32,

    /// The message of the record
    #[serde(default)]
    msg: String,

    /// The configuration that the record is about, where it names one
    file: Option<String>,

    /// The kind of global configuration that the record is about
    ///
    /// Before it looks for a configuration, the validator reads the global
    /// configuration of the environment as Renovate would start with it, and
    /// the records of that reading carry this field. Where the global
    /// configuration is a file, the validator reads it again as a
    /// configuration that it validates, and reports its errors and warnings
    /// again with the file. The second reading starts from the configuration
    /// that the first one migrated, so a migration of the file is reported
    /// only by the first reading. A global configuration that the environment
    /// holds in a variable is only read the first time. The validator fails a
    /// run for neither of the two.
    #[serde(rename = "configType")]
    config_type: Option<Value>,

    /// The headers of a host rule that Renovate would not send
    ///
    /// The validator lists them in a warning without a file before it
    /// validates the configuration, and it reports each of them again as an
    /// error of the configuration with the file.
    denied: Option<Value>,

    /// The errors that the validator found in the configuration
    #[serde(default)]
    errors: Vec<Entry>,

    /// The warnings that the validator found in the configuration
    #[serde(default)]
    warnings: Vec<Entry>,

    /// The error that stopped the reading of the configuration
    err: Option<Cause>,

    /// The configuration as it is, in a record that reports a migration
    #[serde(rename = "oldConfig")]
    old_config: Option<Map<String, Value>>,

    /// The configuration that the migration makes of it
    #[serde(rename = "newConfig")]
    new_config: Option<Map<String, Value>>,
}

/// One error or warning that the validator found in a configuration
#[derive(Deserialize)]
struct Entry {
    /// The kind of the problem, or the option that it is about
    topic: String,

    /// What the validator found
    message: String,
}

/// The error that stopped the reading of a configuration
#[derive(Deserialize)]
struct Cause {
    /// What the error says
    message: Option<String>,

    /// The line that the error is on, starting at 1
    #[serde(rename = "lineNumber")]
    line_number: Option<u32>,

    /// The column that the error is at, starting at 1
    #[serde(rename = "columnNumber")]
    column_number: Option<u32>,
}

impl Cause {
    /// Returns the position that the error names, if it names a line
    fn position(&self) -> Option<Position> {
        let line = self.line_number?;

        Some(
            Position::builder()
                .line(line)
                .maybe_column(self.column_number)
                .build(),
        )
    }
}

/// What one record means for the answer of a run
enum Meaning {
    /// The validator starts to read the configuration in the given file
    Announcement(PathBuf),

    /// The validator found no problem in the given number of configurations
    Validated(u32),

    /// The validator found these problems
    Problems(Vec<RenovateProblem>),

    /// The configuration that the validator read last needs a migration
    Migration {
        /// The words of the validator for a migration
        topic: String,

        /// The options that the migration changes
        options: String,
    },

    /// The validator found no configuration, and wrote these words about it
    Unconfigured(String),

    /// The validator stopped, and wrote these words about it
    Fatal(String),

    /// The record is the log of the run, and carries no answer
    Log,
}

/// Returns the answer that the log of a run holds
///
/// Each record that reports an error or a warning becomes one problem per
/// entry. A record that reports a migration names no file, so it becomes a
/// problem of the configuration that the validator announced last. The count
/// of the configurations is the number that the validator wrote at the end of
/// a run that found no problem.
///
/// A record that the validator writes only as a log of the run carries no
/// answer, and the reading leaves it out. A warning or an error that the
/// reading does not know stops it instead, because a reading that tolerated
/// such a record would let a run pass while a problem of the project went
/// unread.
///
/// # Errors
///
/// Returns [`MalformedRecord`][malformed] for a line that is no JSON record,
/// [`UnattributedMigration`][unattributed] for a migration before the first
/// announcement, and [`UnrecognizedRecord`][unrecognized] for a warning or an
/// error that the reading does not know.
///
/// [malformed]: ReadReportError::MalformedRecord
/// [unattributed]: ReadReportError::UnattributedMigration
/// [unrecognized]: ReadReportError::UnrecognizedRecord
// checkrenovateconfig[impl check.error]
// checkrenovateconfig[impl check.migration]
// checkrenovateconfig[impl check.summary]
// checkrenovateconfig[impl check.unreadable]
// checkrenovateconfig[impl check.warning]
pub fn read(log: &str) -> Result<Report, ReadReportError> {
    let mut problems = Vec::new();
    let mut validated = 0;
    let mut announced: Option<PathBuf> = None;
    let mut unconfigured = None;

    for line in log.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }

        let record: Record =
            serde_json::from_str(line).map_err(|source| ReadReportError::MalformedRecord {
                line: line.to_owned(),
                source,
            })?;

        let Some(meaning) = meaning(record) else {
            return Err(ReadReportError::UnrecognizedRecord {
                line: line.to_owned(),
            });
        };

        match meaning {
            Meaning::Announcement(path) => announced = Some(path),
            Meaning::Validated(count) => validated = count,
            Meaning::Problems(found) => problems.extend(found),
            Meaning::Migration { topic, options } => {
                let Some(path) = announced.clone() else {
                    return Err(ReadReportError::UnattributedMigration {
                        line: line.to_owned(),
                    });
                };

                problems.push(RenovateProblem::new(path, None, topic, options));
            }
            // checkrenovateconfig[impl skip.unconfigured]
            Meaning::Unconfigured(words) => unconfigured = Some(words),
            // checkrenovateconfig[impl check.aborted]
            Meaning::Fatal(details) => return Ok(Report::Halted { details }),
            Meaning::Log => {}
        }
    }

    Ok(Report::Finished(
        Observation::builder()
            .problems(problems)
            .validated(validated)
            .maybe_unconfigured(unconfigured)
            .build(),
    ))
}

/// Returns what a record means for the answer, or `None` for a record that
/// the reading does not know
///
/// A fatal record stops the validator whatever else it carries. A record of
/// the first reading of the global configuration counts as the log of the run,
/// because the validator decides on the global configuration only where it
/// reads it again as a file, and a migration that only the first reading
/// reports does not fail the run of the validator either. A record that lists
/// the denied headers of a host rule counts as the log as well, because the
/// error that follows it reports the same headers with the file. Below the
/// level of a warning, only the announcement of a configuration and the count
/// of a passing run carry an answer.
fn meaning(record: Record) -> Option<Meaning> {
    if record.level >= FATAL {
        return Some(Meaning::Fatal(fatal(&record)));
    }

    if record.config_type.is_some() || record.denied.is_some() {
        return Some(Meaning::Log);
    }

    if record.level < WARNING {
        if let Some(configuration) = record.msg.strip_prefix(ANNOUNCEMENT) {
            return Some(Meaning::Announcement(file(configuration)));
        }

        // checkrenovateconfig[impl check.summary]
        if let Some(count) = record.msg.strip_prefix(COUNT_OPEN) {
            let count = count.strip_suffix(COUNT_CLOSE)?.parse().ok()?;

            return Some(Meaning::Validated(count));
        }

        return Some(Meaning::Log);
    }

    if !record.errors.is_empty() || !record.warnings.is_empty() {
        let path = file(record.file.as_deref()?);

        return Some(Meaning::Problems(
            record
                .errors
                .into_iter()
                .chain(record.warnings)
                .map(|entry| RenovateProblem::new(path.clone(), None, entry.topic, entry.message))
                .collect(),
        ));
    }

    if let (Some(old), Some(new)) = (&record.old_config, &record.new_config) {
        return Some(Meaning::Migration {
            options: changed(old, new),
            topic: record.msg,
        });
    }

    if record.msg.starts_with(MIGRATION_DRAWING) {
        return Some(Meaning::Log);
    }

    // checkrenovateconfig[impl check.unparsable]
    if let (Some(configuration), Some(cause)) = (record.file.as_deref(), &record.err) {
        return Some(Meaning::Problems(vec![RenovateProblem::new(
            file(configuration),
            cause.position(),
            record.msg.clone(),
            cause.message.clone().unwrap_or_default(),
        )]));
    }

    if record.msg == UNCONFIGURED {
        return Some(Meaning::Unconfigured(record.msg));
    }

    None
}

/// Returns the options at the top of a configuration that a migration changes
///
/// The options are sorted by name, so the message of a finding does not
/// depend on the order in which the validator wrote them.
fn changed(old: &Map<String, Value>, new: &Map<String, Value>) -> String {
    old.keys()
        .chain(new.keys())
        .filter(|option| old.get(*option) != new.get(*option))
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(OPTION_SEPARATOR)
}

/// Returns what a fatal record says, with the error that it carries
fn fatal(record: &Record) -> String {
    match record
        .err
        .as_ref()
        .and_then(|cause| cause.message.as_deref())
    {
        Some(message) => format!("{}{CAUSE_OPEN}{message}", record.msg),
        None => record.msg.clone(),
    }
}

/// Returns the file that holds a configuration that the validator named
///
/// A configuration in a key of `package.json` belongs to that file, because
/// the key is not a file of its own.
// checkrenovateconfig[impl check.embedded]
fn file(configuration: &str) -> PathBuf {
    let name = configuration
        .split_once(KEY_SEPARATOR)
        .map_or(configuration, |(name, _key)| name);

    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that reads a log
    // which the validator could have written expects the reading to succeed.
    // A `# Panics` section on every test would repeat that and give the reader
    // no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A log that announces one configuration and reports nothing about it
    const VALID: &str = r#"{"name":"renovate","level":30,"msg":"Validating renovate.json","v":0}
{"name":"renovate","level":30,"msg":"Config validated successfully against 1 file(s)","v":0}"#;

    /// A log that counts two presets in one announced configuration
    const PRESETS: &str = r#"{"level":30,"msg":"Validating package.json > renovate-config"}
{"level":30,"msg":"Config validated successfully against 2 file(s)"}"#;

    /// A log whose count the reading cannot read
    const UNREADABLE_COUNT: &str =
        r#"{"level":30,"msg":"Config validated successfully against many file(s)"}"#;

    /// A log that lists the denied headers of a host rule before their error
    const DENIED_HEADER: &str = r#"{"level":30,"msg":"Validating renovate.json"}
{"level":40,"denied":["Authorization"],"msg":"Ignoring hostRules headers not permitted by this Renovate instance's `allowedHeaders`"}
{"level":50,"file":"renovate.json","errors":[{"topic":"Config security error","message":"hostRules header `Authorization` is not allowed by this Renovate instance's `allowedHeaders`."}],"msg":"Found errors in configuration"}"#;

    /// A log that reports an error of one configuration
    const ERROR: &str = r#"{"level":30,"msg":"Validating renovate.json"}
{"level":50,"file":"renovate.json","errors":[{"topic":"Configuration Error","message":"Invalid configuration option: foo"}],"msg":"Found errors in configuration"}"#;

    /// A log that reports a warning of one configuration
    const WARNING_ONLY: &str = r#"{"level":30,"msg":"Validating .github/renovate.json"}
{"level":40,"file":".github/renovate.json","warnings":[{"topic":"Configuration Warning","message":"Setting `registryUrls` at the top level of your config will apply it to all managers and datasources."}],"msg":"Found errors in configuration"}"#;

    /// A log that reports a migration of one configuration
    const MIGRATION: &str = r#"{"level":30,"msg":"Validating renovate.json"}
{"level":40,"oldConfig":{"extends":["config:base"],"customManagers":[{"fileMatch":["x"]}],"automerge":true},"newConfig":{"extends":["config:recommended"],"customManagers":[{"managerFilePatterns":["/x/"]}],"automerge":true},"msg":"Config migration necessary"}
{"level":40,"msg":"Config migration diff:\n  {\n-   \"extends\": [\"config:base\"]\n+   \"extends\": [\"config:recommended\"]\n  }"}"#;

    /// A log that reports a migration before it names a configuration
    const UNATTRIBUTED: &str = r#"{"level":40,"oldConfig":{"extends":["config:base"]},"newConfig":{"extends":["config:recommended"]},"msg":"Config migration necessary"}"#;

    /// A log that reports a configuration that the validator cannot parse
    const SYNTAX_ERROR: &str = r#"{"level":40,"file":"renovate.json","err":{"lineNumber":2,"columnNumber":1,"message":"JSON5: invalid end of input at 2:1","stack":"SyntaxError: JSON5"},"msg":"File could not be parsed"}"#;

    /// A log that reports a configuration that the validator rejected whole
    const INVALID: &str = r#"{"level":30,"msg":"Validating renovate.json"}
{"level":40,"file":"renovate.json","err":{"message":"boom"},"msg":"File is not valid Renovate config"}"#;

    /// A log that reports an error of the configuration in `package.json`
    const EMBEDDED: &str = r#"{"level":30,"msg":"Validating package.json > renovate"}
{"level":50,"file":"package.json > renovate","errors":[{"topic":"Configuration Error","message":"Invalid configuration option: foo"}],"msg":"Found errors in configuration"}"#;

    /// A log that reports an error of the global configuration twice
    const GLOBAL: &str = r#"{"level":40,"configType":"config.js","errors":[{"topic":"Configuration Error","message":"Invalid configuration option: foo"}],"msg":"Config validation errors found"}
{"level":30,"msg":"Validating config.js"}
{"level":50,"file":"config.js","errors":[{"topic":"Configuration Error","message":"Invalid configuration option: foo"}],"msg":"Found errors in configuration"}"#;

    /// A log of a project without a configuration
    const UNCONFIGURED_LOG: &str =
        r#"{"level":40,"msg":"No files to perform configuration validation against"}"#;

    /// A log that ends with a fatal record
    const FATAL_LOG: &str = r#"{"level":60,"err":{"message":"Unexpected end of input","stack":"SyntaxError"},"msg":"Could not parse config file"}"#;

    /// A log with a warning that the reading does not know
    const UNKNOWN_WARNING: &str = r#"{"level":40,"msg":"Something new happened"}"#;

    /// Returns the observation of a log that the validator finished
    fn observation(log: &str) -> Observation {
        match read(log).expect("the test reads a log that the validator could write") {
            Report::Finished(observation) => observation,
            halted @ Report::Halted { .. } => panic!("expected a finished run, got {halted:?}"),
        }
    }

    /// Returns the messages of the problems of a log
    fn messages(log: &str) -> Vec<String> {
        observation(log)
            .problems()
            .iter()
            .map(RenovateProblem::message)
            .collect()
    }

    /// Returns the paths of the problems of a log
    fn paths(log: &str) -> Vec<PathBuf> {
        observation(log)
            .problems()
            .iter()
            .map(|problem| problem.path().clone())
            .collect()
    }

    // checkrenovateconfig[verify check.embedded]
    #[test]
    fn read_of_a_configuration_in_package_json_names_package_json() {
        let paths = paths(EMBEDDED);

        assert_eq!(paths, [PathBuf::from("package.json")]);
    }

    // checkrenovateconfig[verify check.unparsable]
    #[test]
    fn read_of_a_configuration_that_the_validator_rejected_holds_its_words() {
        let messages = messages(INVALID);

        assert_eq!(messages, ["File is not valid Renovate config: boom"]);
    }

    // checkrenovateconfig[verify check.error]
    #[test]
    fn read_of_a_denied_header_reports_the_error_once() {
        let messages = messages(DENIED_HEADER);

        assert_eq!(
            messages,
            [
                "Config security error: hostRules header `Authorization` is not allowed by this \
                 Renovate instance's `allowedHeaders`."
            ]
        );
    }

    // checkrenovateconfig[verify check.aborted]
    #[test]
    fn read_of_a_fatal_record_reports_that_the_validator_stopped() {
        let report = read(FATAL_LOG).expect("the test reads a log that the validator could write");

        assert_eq!(
            report,
            Report::Halted {
                details: "Could not parse config file: Unexpected end of input".to_owned(),
            }
        );
    }

    #[test]
    fn read_of_a_global_configuration_reports_each_problem_once() {
        let messages = messages(GLOBAL);

        assert_eq!(
            messages,
            ["Configuration Error: Invalid configuration option: foo"]
        );
    }

    // checkrenovateconfig[verify check.unreadable]
    #[test]
    fn read_of_a_line_that_is_no_record_stops() {
        let report = read("Validating renovate.json");

        assert!(
            matches!(report, Err(ReadReportError::MalformedRecord { .. })),
            "expected the reading to stop, got {report:?}"
        );
    }

    // checkrenovateconfig[verify check.migration]
    #[test]
    fn read_of_a_migration_names_each_option_that_it_changes() {
        let messages = messages(MIGRATION);

        assert_eq!(
            messages,
            ["Config migration necessary: customManagers, extends"]
        );
    }

    // checkrenovateconfig[verify check.migration]
    #[test]
    fn read_of_a_migration_names_the_configuration_that_the_validator_announced() {
        let paths = paths(MIGRATION);

        assert_eq!(paths, [PathBuf::from("renovate.json")]);
    }

    // checkrenovateconfig[verify check.unreadable]
    #[test]
    fn read_of_a_migration_without_an_announcement_stops() {
        let report = read(UNATTRIBUTED);

        assert!(
            matches!(report, Err(ReadReportError::UnattributedMigration { .. })),
            "expected the reading to stop, got {report:?}"
        );
    }

    // checkrenovateconfig[verify skip.unconfigured]
    #[test]
    fn read_of_a_project_without_a_configuration_holds_the_words_of_the_validator() {
        let unconfigured = observation(UNCONFIGURED_LOG).unconfigured().clone();

        assert_eq!(
            unconfigured.as_deref(),
            Some("No files to perform configuration validation against")
        );
    }

    // checkrenovateconfig[verify check.unparsable]
    #[test]
    fn read_of_a_syntax_error_names_the_position() {
        let problems = observation(SYNTAX_ERROR).problems().clone();

        assert_eq!(
            problems,
            [RenovateProblem::new(
                PathBuf::from("renovate.json"),
                Some(Position::builder().line(2).column(1).build()),
                "File could not be parsed".to_owned(),
                "JSON5: invalid end of input at 2:1".to_owned(),
            )]
        );
    }

    // checkrenovateconfig[verify check.passed]
    #[test]
    fn read_of_a_valid_configuration_holds_no_problem() {
        let observation = observation(VALID);

        assert!(
            observation.problems().is_empty(),
            "expected no problem, got {observation:?}"
        );
    }

    // checkrenovateconfig[verify check.summary]
    #[test]
    fn read_of_a_valid_configuration_holds_the_count_of_the_validator() {
        let validated = observation(VALID).validated();

        assert_eq!(validated, 1);
    }

    // checkrenovateconfig[verify check.warning]
    #[test]
    fn read_of_a_warning_holds_the_words_of_the_validator() {
        let messages = messages(WARNING_ONLY);

        assert_eq!(
            messages,
            [
                "Configuration Warning: Setting `registryUrls` at the top level of your config \
                 will apply it to all managers and datasources."
            ]
        );
    }

    // checkrenovateconfig[verify check.error]
    #[test]
    fn read_of_an_error_holds_the_words_of_the_validator() {
        let messages = messages(ERROR);

        assert_eq!(
            messages,
            ["Configuration Error: Invalid configuration option: foo"]
        );
    }

    // checkrenovateconfig[verify check.error]
    #[test]
    fn read_of_an_error_names_the_file() {
        let paths = paths(ERROR);

        assert_eq!(paths, [PathBuf::from("renovate.json")]);
    }

    // checkrenovateconfig[verify check.unreadable]
    #[test]
    fn read_of_an_unknown_warning_stops() {
        let report = read(UNKNOWN_WARNING);

        assert!(
            matches!(report, Err(ReadReportError::UnrecognizedRecord { .. })),
            "expected the reading to stop, got {report:?}"
        );
    }

    // checkrenovateconfig[verify check.unreadable]
    #[test]
    fn read_of_an_unreadable_count_stops() {
        let report = read(UNREADABLE_COUNT);

        assert!(
            matches!(report, Err(ReadReportError::UnrecognizedRecord { .. })),
            "expected the reading to stop, got {report:?}"
        );
    }

    // checkrenovateconfig[verify check.summary]
    #[test]
    fn read_of_presets_holds_the_count_of_the_validator() {
        let validated = observation(PRESETS).validated();

        assert_eq!(validated, 2);
    }
}
