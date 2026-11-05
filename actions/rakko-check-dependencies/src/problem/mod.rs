//! One thing that cargo-deny reported about a workspace
//!
//! A run of cargo-deny reports one entry for every shape that it recognized
//! in the dependencies of a workspace. This module holds one of those
//! entries: how the project weighed the shape, which check recognized it,
//! what cargo-deny wrote about it, and the packages that it is about.

mod package;
mod severity;

use bon::Builder;
use getset::{CopyGetters, Getters};

pub use self::package::Package;
pub use self::severity::Severity;

/// The character that opens the check of a problem in a message
const CHECK_OPEN: char = '[';

/// The text that closes the check of a problem in a message
const CHECK_CLOSE: &str = "] ";

/// The text that opens the packages of a problem in a message
const PACKAGES_OPEN: &str = " (";

/// The text between two packages of a problem in a message
const PACKAGES_SEPARATOR: &str = ", ";

/// The character that closes the packages of a problem in a message
const PACKAGES_CLOSE: char = ')';

/// One thing that cargo-deny reported about a workspace
///
/// The value holds one entry of a report: the weight that the configuration
/// of the project gave it, the check that recognized the shape, the sentence
/// that cargo-deny wrote about it, and the packages that the entry is about.
///
/// The packages travel with the entry because the message often does not name
/// them. Cargo-deny draws a block for a reader with the manifest of the
/// package above it, so the identity of a package whose license was rejected
/// sits in the path of that block. The report keeps the same identity in the
/// inclusion graph of the entry, and a problem carries it in a field, so that
/// a reader of a finding learns which package the answer is about.
///
/// An entry names no file of the project. The place that cargo-deny
/// underlines is a line of a lock file, or of a manifest in the registry
/// cache of the machine, and the report leaves the file out of the entry
/// altogether. A caller therefore places a problem at the workspace that it
/// came from, and never at a path.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub struct DenyProblem {
    /// How the configuration of the project weighed the shape
    #[getset(get_copy = "pub")]
    severity: Severity,

    /// The check that recognized the shape, in the words of cargo-deny
    #[builder(into)]
    #[getset(get = "pub")]
    check: String,

    /// What cargo-deny wrote about the shape
    #[builder(into)]
    #[getset(get = "pub")]
    message: String,

    /// The packages that the entry is about, in the order of the report
    #[getset(get = "pub")]
    packages: Vec<Package>,
}

impl DenyProblem {
    /// Returns whether the project said that the shape must not appear
    ///
    /// A run fails over a problem that answers `true`, and passes over one
    /// that answers `false`. The weight comes from the configuration of the
    /// project, so this asks what the project already decided.
    pub fn denied(&self) -> bool {
        self.severity >= Severity::Error
    }

    /// Returns the sentence that cargo-deny wrote about the problem
    ///
    /// The sentence opens with the check that recognized the shape, in the
    /// place where cargo-deny writes it, and it ends with the packages that
    /// the entry is about. A problem that names no package ends with the
    /// sentence of cargo-deny, because empty parentheses tell a reader
    /// nothing.
    pub fn description(&self) -> String {
        let head = format!("{CHECK_OPEN}{}{CHECK_CLOSE}{}", self.check, self.message);

        if self.packages.is_empty() {
            return head;
        }

        let packages: Vec<String> = self.packages.iter().map(Package::to_string).collect();

        format!(
            "{head}{PACKAGES_OPEN}{}{PACKAGES_CLOSE}",
            packages.join(PACKAGES_SEPARATOR)
        )
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns a problem of the given weight that names one package
    fn problem(severity: Severity) -> DenyProblem {
        DenyProblem::builder()
            .severity(severity)
            .check("rejected".to_owned())
            .message("failed to satisfy license requirements".to_owned())
            .packages(vec![
                Package::builder()
                    .name("option-ext".to_owned())
                    .version("0.2.0".to_owned())
                    .build(),
            ])
            .build()
    }

    #[test]
    fn denied_of_a_warning_reports_a_shape_that_the_project_reads_about() {
        let denied = problem(Severity::Warning).denied();

        assert!(!denied, "expected the warning to leave a run passing");
    }

    #[test]
    fn denied_of_an_error_reports_a_shape_that_must_not_appear() {
        let denied = problem(Severity::Error).denied();

        assert!(denied, "expected the error to fail a run");
    }

    #[test]
    fn deny_problem_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<DenyProblem>();
    }

    #[test]
    fn description_holds_the_check_and_the_words_of_cargo_deny() {
        let description = problem(Severity::Error).description();

        assert_eq!(
            description,
            "[rejected] failed to satisfy license requirements (option-ext 0.2.0)"
        );
    }

    #[test]
    fn description_of_a_problem_about_no_package_ends_with_the_words_of_cargo_deny() {
        let problem = DenyProblem::builder()
            .severity(Severity::Warning)
            .check("license-not-encountered".to_owned())
            .message("license was not encountered".to_owned())
            .packages(Vec::new())
            .build();

        let description = problem.description();

        assert_eq!(
            description,
            "[license-not-encountered] license was not encountered"
        );
    }

    #[test]
    fn description_of_a_problem_about_several_packages_names_every_one() {
        let problem = DenyProblem::builder()
            .severity(Severity::Error)
            .check("duplicate".to_owned())
            .message("found 2 duplicate entries for crate 'syn'".to_owned())
            .packages(vec![
                Package::builder()
                    .name("syn".to_owned())
                    .version("2.0.119".to_owned())
                    .build(),
                Package::builder()
                    .name("syn".to_owned())
                    .version("3.0.4".to_owned())
                    .build(),
            ])
            .build();

        let description = problem.description();

        assert_eq!(
            description,
            "[duplicate] found 2 duplicate entries for crate 'syn' (syn 2.0.119, syn 3.0.4)"
        );
    }
}
