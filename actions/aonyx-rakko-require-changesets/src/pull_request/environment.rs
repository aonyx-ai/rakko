//! The variables of GitHub Actions that point a run to its pull request

use std::ffi::OsString;
use std::path::PathBuf;

use getset::Getters;

/// The variable that holds the name of the event that started the workflow
const EVENT_NAME: &str = "GITHUB_EVENT_NAME";

/// The variable that holds the path of the file that describes the event
const EVENT_PATH: &str = "GITHUB_EVENT_PATH";

/// The variables of GitHub Actions that tell a run about its event
///
/// GitHub Actions sets both variables in every job. A run elsewhere, such as
/// on the machine of a contributor, usually has neither. The action reads them
/// once, at the start of a run. The rest of the run, and every test, then
/// works on a value instead of the environment of the process.
#[derive(Clone, Eq, PartialEq, Debug, Default, bon::Builder, Getters)]
pub(crate) struct Environment {
    /// The name of the event, such as `pull_request`, or `None` when the
    /// variable is not set
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    event_name: Option<OsString>,

    /// The path of the event file, or `None` when the variable is not set
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    event_path: Option<PathBuf>,
}

impl Environment {
    /// Returns the variables of the environment of the process
    pub(crate) fn of_process() -> Self {
        Self {
            event_name: std::env::var_os(EVENT_NAME),
            event_path: std::env::var_os(EVENT_PATH).map(PathBuf::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Environment>();
    }
}
