//! The pull request that a run judges
//!
//! GitHub Actions writes the event that started a job to a JSON file, and the
//! variable `GITHUB_EVENT_PATH` gives its path. For the event of a pull
//! request, the file holds the body of the pull request, its base, and its
//! head. The action reads the pull request from that file, without a call over
//! the network and without a token.

mod body;
mod branch;
mod commit;
mod environment;
mod error;
mod origin;

use std::path::Path;

use getset::{CopyGetters, Getters};
use serde::Deserialize;

pub(crate) use self::body::Body;
pub(crate) use self::branch::Branch;
pub(crate) use self::commit::Commit;
pub(crate) use self::environment::Environment;
pub(crate) use self::error::ReadPullRequestError;
pub(crate) use self::origin::Origin;

/// The names of the events of GitHub Actions that carry a pull request
const PULL_REQUEST_EVENTS: [&str; 2] = ["pull_request", "pull_request_target"];

/// The facts of a pull request that the check judges
///
/// The body holds the opt-out lines. The base and the head mark the change.
/// The branch and its origin mark a release pull request of Otto.
#[derive(Clone, Eq, PartialEq, Debug, bon::Builder, CopyGetters, Getters)]
pub(crate) struct PullRequest {
    /// The body of the pull request, which is empty when the author wrote none
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    body: Body,

    /// The head commit of the base branch when the event happened
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    base: Commit,

    /// The head commit of the pull request
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    head: Commit,

    /// The head branch of the pull request
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    branch: Branch,

    /// The repository that holds the head branch
    #[getset(get_copy = "pub(crate)")]
    origin: Origin,
}

impl PullRequest {
    /// Returns the pull request of the run, or `None` when the run has none
    ///
    /// A run has a pull request when GitHub Actions started it for the event
    /// of a pull request. Any other run, such as one on the machine of a
    /// contributor, or a job that a push started, has none.
    ///
    /// # Errors
    ///
    /// Returns [`MissingEventPath`][missing] when the event is one of a pull
    /// request and no variable gives the event file. Returns
    /// [`UnreadableEvent`][unreadable] when the file cannot be read, and
    /// [`MalformedEvent`][malformed] when it holds no pull request.
    ///
    /// [malformed]: ReadPullRequestError::MalformedEvent
    /// [missing]: ReadPullRequestError::MissingEventPath
    /// [unreadable]: ReadPullRequestError::UnreadableEvent
    pub(crate) fn read(environment: &Environment) -> Result<Option<Self>, ReadPullRequestError> {
        let Some(name) = environment.event_name() else {
            return Ok(None);
        };

        if !PULL_REQUEST_EVENTS.iter().any(|event| name == *event) {
            return Ok(None);
        }

        let Some(path) = environment.event_path() else {
            return Err(ReadPullRequestError::MissingEventPath);
        };

        Self::read_event(path).map(Some)
    }

    /// Returns the pull request of the event file at the path
    ///
    /// # Errors
    ///
    /// Returns [`UnreadableEvent`][unreadable] when the file cannot be read,
    /// and [`MalformedEvent`][malformed] when it holds no pull request.
    ///
    /// [malformed]: ReadPullRequestError::MalformedEvent
    /// [unreadable]: ReadPullRequestError::UnreadableEvent
    fn read_event(path: &Path) -> Result<Self, ReadPullRequestError> {
        let text = std::fs::read_to_string(path).map_err(|source| {
            ReadPullRequestError::UnreadableEvent {
                path: path.to_path_buf(),
                source,
            }
        })?;

        let event: Event =
            serde_json::from_str(&text).map_err(|source| ReadPullRequestError::MalformedEvent {
                path: path.to_path_buf(),
                source,
            })?;

        let Payload {
            body,
            base:
                Reference {
                    sha: base,
                    repo: base_repository,
                    ..
                },
            head:
                Reference {
                    name,
                    sha: head,
                    repo: head_repository,
                },
        } = event.pull_request;

        let origin = match (base_repository, head_repository) {
            (Some(base), Some(head)) if base.id == head.id => Origin::BaseRepository,
            _ => Origin::OtherRepository,
        };

        Ok(Self::builder()
            .body(body.unwrap_or_default())
            .base(base)
            .head(head)
            .branch(name)
            .origin(origin)
            .build())
    }
}

/// The part of the event file that the action reads
#[derive(Debug, Deserialize)]
struct Event {
    /// The pull request of the event
    pull_request: Payload,
}

/// The pull request in the event file
#[derive(Debug, Deserialize)]
struct Payload {
    /// The body, which GitHub writes as `null` when the author wrote none
    body: Option<String>,

    /// The base of the pull request
    base: Reference,

    /// The head of the pull request
    head: Reference,
}

/// A branch and its commit in the event file
#[derive(Debug, Deserialize)]
struct Reference {
    /// The name of the branch
    #[serde(rename = "ref")]
    name: String,

    /// The hash of the head commit of the branch
    sha: String,

    /// The repository that holds the branch
    ///
    /// GitHub writes `null` for the head of a pull request whose fork it no
    /// longer has.
    repo: Option<Repository>,
}

/// A repository in the event file
#[derive(Debug, Deserialize)]
struct Repository {
    /// The number that GitHub gives the repository, which a rename keeps
    id: u64,
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "An assertion in a test panics by design, and the helpers of this module exist only for tests."
    )]
    #![allow(clippy::missing_panics_doc)]

    use std::ffi::OsString;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    /// An event of a pull request, with the fields that GitHub writes and the
    /// action reads
    const EVENT: &str = r#"{
        "action": "edited",
        "number": 7,
        "pull_request": {
            "body": "No changeset: a refactor",
            "base": {
                "ref": "main",
                "sha": "1111111111111111111111111111111111111111",
                "repo": { "id": 81, "full_name": "aonyx-ai/rakko" }
            },
            "head": {
                "ref": "fix-parser",
                "sha": "2222222222222222222222222222222222222222",
                "repo": { "id": 81, "full_name": "aonyx-ai/rakko" }
            }
        }
    }"#;

    /// Returns the environment of a job for the event, with an event file that
    /// holds the text
    #[bon::builder]
    fn environment(name: &str, text: &str) -> (TempDir, Environment) {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let path = directory.path().join("event.json");
        std::fs::write(&path, text).expect("the test writes the event file");

        let environment = Environment::builder()
            .event_name(OsString::from(name))
            .event_path(path)
            .build();

        (directory, environment)
    }

    #[test]
    fn event_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Event>();
    }

    #[test]
    fn payload_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Payload>();
    }

    #[test]
    fn pull_request_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<PullRequest>();
    }

    #[test]
    fn read_of_a_malformed_event_fails() {
        let (_directory, environment) = environment()
            .name("pull_request")
            .text(r#"{ "pull_request": {} }"#)
            .call();

        let error = PullRequest::read(&environment).unwrap_err();

        assert!(
            matches!(error, ReadPullRequestError::MalformedEvent { .. }),
            "expected a malformed event, got {error:?}"
        );
    }

    #[test]
    fn read_of_a_missing_event_file_fails() {
        let environment = Environment::builder()
            .event_name(OsString::from("pull_request"))
            .event_path(PathBuf::from("/nonexistent/event.json"))
            .build();

        let error = PullRequest::read(&environment).unwrap_err();

        assert!(
            matches!(error, ReadPullRequestError::UnreadableEvent { .. }),
            "expected an unreadable event, got {error:?}"
        );
    }

    #[test]
    fn read_of_a_pull_request_event_reads_the_pull_request() {
        let (_directory, environment) = environment().name("pull_request").text(EVENT).call();

        let pull_request = PullRequest::read(&environment).unwrap();

        assert_eq!(
            pull_request,
            Some(
                PullRequest::builder()
                    .body("No changeset: a refactor")
                    .base("1111111111111111111111111111111111111111")
                    .head("2222222222222222222222222222222222222222")
                    .branch("fix-parser")
                    .origin(Origin::BaseRepository)
                    .build()
            )
        );
    }

    #[test]
    fn read_of_a_pull_request_event_without_a_body_reads_an_empty_body() {
        let (_directory, environment) = environment()
            .name("pull_request")
            .text(&EVENT.replace(r#""No changeset: a refactor""#, "null"))
            .call();

        let pull_request = PullRequest::read(&environment).unwrap();

        assert_eq!(
            pull_request.map(|pull_request| pull_request.body),
            Some(Body::new(""))
        );
    }

    #[test]
    fn read_of_a_pull_request_event_without_an_event_file_fails() {
        let environment = Environment::builder()
            .event_name(OsString::from("pull_request"))
            .build();

        let error = PullRequest::read(&environment).unwrap_err();

        assert!(
            matches!(error, ReadPullRequestError::MissingEventPath),
            "expected a missing event, got {error:?}"
        );
    }

    #[test]
    fn read_of_a_pull_request_from_a_deleted_fork_has_another_origin() {
        let event = EVENT.replacen(
            r#""repo": { "id": 81, "full_name": "aonyx-ai/rakko" }
            }
        }"#,
            r#""repo": null
            }
        }"#,
            1,
        );
        let (_directory, environment) = environment().name("pull_request").text(&event).call();

        let pull_request = PullRequest::read(&environment).unwrap();

        assert_eq!(
            pull_request.map(|pull_request| pull_request.origin),
            Some(Origin::OtherRepository)
        );
    }

    #[test]
    fn read_of_a_pull_request_from_a_fork_has_another_origin() {
        let event = EVENT.replacen(
            r#""repo": { "id": 81, "full_name": "aonyx-ai/rakko" }
            }
        }"#,
            r#""repo": { "id": 92, "full_name": "otter/rakko" }
            }
        }"#,
            1,
        );
        let (_directory, environment) = environment().name("pull_request").text(&event).call();

        let pull_request = PullRequest::read(&environment).unwrap();

        assert_eq!(
            pull_request.map(|pull_request| pull_request.origin),
            Some(Origin::OtherRepository)
        );
    }

    #[test]
    fn read_of_a_pull_request_target_event_reads_the_pull_request() {
        let (_directory, environment) =
            environment().name("pull_request_target").text(EVENT).call();

        let pull_request = PullRequest::read(&environment).unwrap();

        assert!(pull_request.is_some(), "expected a pull request");
    }

    #[test]
    fn read_of_a_push_event_has_no_pull_request() {
        let (_directory, environment) = environment().name("push").text(EVENT).call();

        let pull_request = PullRequest::read(&environment).unwrap();

        assert_eq!(pull_request, None);
    }

    #[test]
    fn read_without_an_event_has_no_pull_request() {
        let pull_request = PullRequest::read(&Environment::default()).unwrap();

        assert_eq!(pull_request, None);
    }

    #[test]
    fn reference_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Reference>();
    }

    #[test]
    fn repository_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Repository>();
    }
}
