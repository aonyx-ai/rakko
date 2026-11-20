//! The Otto that lists the release units of a project
//!
//! Otto finds the release units of a repository, with its edge cases such as
//! lockstep workspaces and internal crates. The action asks it for them with
//! `otto list-units --json`, so that the check and a release of Otto agree on
//! the units.

mod error;
mod report;

use rakko::{Action, Context, ToolOutput};
use rakko_tool::{Launch, ResolveToolError, ShownStreams, Tool};

pub(crate) use self::error::ListUnitsError;
use self::report::Answer;
use crate::git;
use crate::unit_listing::UnitListing;

/// The name that mise knows Otto by
///
/// Otto is a crate on crates.io, and mise builds it from source with its
/// `cargo` backend.
const OTTO: &str = "cargo:otto";

/// The program of Otto
const PROGRAM: &str = "otto";

/// The command of Otto that lists the release units
const LIST_UNITS: &str = "list-units";

/// The flag that asks Otto for its answer as JSON
const JSON: &str = "--json";

/// The Otto that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. A run of it starts in the root of the project, so Otto
/// lists the units at the root and below it.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Otto {
    /// The program that mise installed at the declared version
    tool: Tool,

    /// The tool output of the context that resolved Otto
    output: ToolOutput,
}

#[bon::bon]
impl Otto {
    /// Returns the release units at the root of the project and below it,
    /// with the directories that no unit owns
    ///
    /// Otto writes its answer on its standard output stream, and its log on
    /// its standard error stream. Otto runs Git, and a variable such as
    /// `GIT_DIR` from a hook would point Git to another repository. So the run
    /// removes the variables of Git from the environment of Otto.
    ///
    /// # Errors
    ///
    /// Returns [`OttoUnavailable`][unavailable] when Otto does not run, and
    /// [`StoppedList`][stopped] when it stops at a condition of the repository.
    /// Returns [`FailedList`][failed] when it reaches no list, and
    /// [`UnreadableList`][unreadable] when it writes a list that this crate
    /// cannot read.
    ///
    /// [failed]: ListUnitsError::FailedList
    /// [stopped]: ListUnitsError::StoppedList
    /// [unavailable]: ListUnitsError::OttoUnavailable
    /// [unreadable]: ListUnitsError::UnreadableList
    pub(crate) async fn list_units(&self) -> Result<UnitListing, ListUnitsError> {
        let execution = git::without_variables(self.tool.invocation())
            .arg(LIST_UNITS)
            .arg(JSON)
            .launch_with(&self.output, ShownStreams::StandardError)
            .await
            .map_err(|source| ListUnitsError::OttoUnavailable { source })?;

        let ending = if execution.status().success() {
            Ending::Success
        } else {
            Ending::Failure
        };

        units_of()
            .stdout(&execution.stdout().to_string_lossy())
            .stderr(&execution.stderr().to_string_lossy())
            .ending(ending)
            .call()
    }

    /// Returns the Otto that mise installed at the version that the action
    /// declares
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when the action declares no Otto, when
    /// mise does not run, or when mise reports no Otto at that version.
    #[builder]
    pub(crate) async fn resolve<A: Action>(
        context: &Context,
        action: &A,
    ) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve()
            .action(action)
            .tool(OTTO)
            .executable(PROGRAM)
            .context(context)
            .call()
            .await?;

        Ok(Self {
            tool,
            output: context.tool_output().clone(),
        })
    }
}

/// How a run of Otto ended
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Ending {
    /// Otto exited with success
    Success,

    /// Otto exited without success
    Failure,
}

/// Returns the units of what a run of `otto list-units --json` wrote
///
/// Otto writes its answer as JSON on its standard output stream, also for a
/// stop and for an error. A run that ended without success and wrote no
/// answer gives what it wrote on its standard error stream.
///
/// # Errors
///
/// Returns [`StoppedList`][stopped] for a stop, and [`FailedList`][failed]
/// for an error or for a failed run without an answer. Returns
/// [`UnreadableList`][unreadable] for a run that ended with success and wrote
/// no answer that this crate reads.
///
/// [failed]: ListUnitsError::FailedList
/// [stopped]: ListUnitsError::StoppedList
/// [unreadable]: ListUnitsError::UnreadableList
#[bon::builder]
fn units_of(stdout: &str, stderr: &str, ending: Ending) -> Result<UnitListing, ListUnitsError> {
    match self::report::answer(stdout) {
        Ok(Answer::Reached(listed)) => Ok(listed.into_listing()),
        Ok(Answer::Stopped(diagnostic)) => Err(ListUnitsError::StoppedList {
            message: diagnostic.message().to_owned(),
        }),
        Ok(Answer::Error(diagnostic)) => Err(ListUnitsError::FailedList {
            details: diagnostic.message().to_owned(),
        }),
        Err(_) if ending == Ending::Failure => Err(ListUnitsError::FailedList {
            details: stderr.trim().to_owned(),
        }),
        Err(source) => Err(ListUnitsError::UnreadableList {
            output: stdout.to_owned(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::PathBuf;

    use super::*;
    use crate::release_unit::ReleaseUnit;

    #[test]
    fn ending_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Ending>();
    }

    #[test]
    fn otto_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Otto>();
    }

    #[test]
    fn units_of_a_failed_run_without_an_answer_gives_its_error_stream() {
        let result = units_of()
            .stdout("")
            .stderr("error: unexpected argument '--json'\n")
            .ending(Ending::Failure)
            .call();

        assert!(
            matches!(&result, Err(ListUnitsError::FailedList { details }) if details == "error: unexpected argument '--json'"),
            "expected the error stream of otto, got {result:?}"
        );
    }

    #[test]
    fn units_of_a_list_gives_its_units() {
        let result = units_of()
            .stdout(
                r#"{"reached":{"units":[{"name":"rakko","root":"crates/rakko","manifest":"crates/rakko/Cargo.toml","crates":[{"directory":"crates/rakko"}]}],"unowned":[]}}"#,
            )
            .stderr("")
            .ending(Ending::Success)
            .call();

        assert_eq!(
            result.unwrap().units(),
            &[ReleaseUnit::builder()
                .name("rakko")
                .root("crates/rakko")
                .manifest("crates/rakko/Cargo.toml")
                .crates(vec![PathBuf::from("crates/rakko")])
                .build()]
        );
    }

    #[test]
    fn units_of_a_stop_gives_its_message() {
        let result = units_of()
            .stdout(r#"{"stopped":{"message":"two units have the same name"}}"#)
            .stderr("")
            .ending(Ending::Failure)
            .call();

        assert!(
            matches!(&result, Err(ListUnitsError::StoppedList { message }) if message == "two units have the same name"),
            "expected the stop, got {result:?}"
        );
    }

    #[test]
    fn units_of_a_successful_run_without_an_answer_is_unreadable() {
        let result = units_of()
            .stdout("the unit rakko in crates/rakko")
            .stderr("")
            .ending(Ending::Success)
            .call();

        assert!(
            matches!(&result, Err(ListUnitsError::UnreadableList { .. })),
            "expected an unreadable list, got {result:?}"
        );
    }

    #[test]
    fn units_of_an_error_gives_its_message() {
        let result = units_of()
            .stdout(r#"{"error":{"message":"failed to find the top of the repository"}}"#)
            .stderr("")
            .ending(Ending::Failure)
            .call();

        assert!(
            matches!(&result, Err(ListUnitsError::FailedList { details }) if details == "failed to find the top of the repository"),
            "expected the error, got {result:?}"
        );
    }
}
