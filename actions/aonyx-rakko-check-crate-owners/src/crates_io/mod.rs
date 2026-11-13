//! The registry crates.io
//!
//! A run reads the owners of a crate from the public API of the registry. That
//! API needs no token, so a run in CI can read the owners without a secret.
//! A run adds an owner with `cargo owner --add`. Cargo uses the credentials
//! that it already has for the user that starts the run. Only a user owner of
//! a crate can add owners, so the addition needs those credentials.
//!
//! crates.io asks an automated client to send at most one request per second,
//! and to give a User-Agent that identifies the client. The registry of this
//! module waits one second before each lookup and before each run of
//! `cargo owner --add`. Cargo sends requests of its own in such a run. The
//! User-Agent gives the name of this crate and the address of its repository.

mod error;
mod pace;

use std::time::Duration;

use aonyx_rakko_cargo::{Cargo, CargoRoot, PackageName};
use rakko_tool::{Execution, Launch};
use reqwest::Client;
use serde::Deserialize;

pub use self::error::LookUpOwnersError;
use self::pace::Pace;
use crate::crate_owner::CrateOwner;
use crate::registry::{Addition, Ownership, Registry};

/// The address of the API of crates.io that lists the owners of the crates
const CRATES: &str = "https://crates.io/api/v1/crates";

/// The User-Agent that identifies the action to crates.io
const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

/// The shortest time between two requests to crates.io
const INTERVAL: Duration = Duration::from_secs(1);

/// The longest time that one request to crates.io takes before it fails
const TIMEOUT: Duration = Duration::from_secs(30);

/// The status with which crates.io answers for a crate that it does not have
const NOT_FOUND: u16 = 404;

/// The status with which crates.io answers with the owners of a crate
const OK: u16 = 200;

/// What cargo writes into its error when crates.io refuses a request
///
/// crates.io refuses an addition with this status when the user that asks
/// for it is not a user owner of the crate.
const FORBIDDEN: &str = "403 Forbidden";

/// The marker of the line where cargo starts to describe its error
const ERROR_MARKER: &str = "error:";

/// The details of a run of cargo that failed and wrote nothing
const NO_DIAGNOSIS: &str = "cargo wrote nothing about it";

/// The registry crates.io
///
/// The registry reads the owners of a crate over HTTPS. It verifies the
/// certificate of crates.io against the root certificates that the client
/// carries itself. So a run needs no certificate store of the system and no
/// TLS library of the system. It adds an owner with the cargo of the
/// project, so cargo uses the credentials that the user gave it with
/// `cargo login`.
#[derive(Debug)]
pub(crate) struct CratesIo {
    /// The client that sends the requests to the API
    client: Client,

    /// The cargo that adds an owner
    cargo: Cargo,

    /// The wait between two requests
    pace: Pace,
}

impl CratesIo {
    /// Builds the registry with the cargo that adds an owner
    ///
    /// The client sets up its TLS with the root certificates that it carries.
    ///
    /// # Errors
    ///
    /// Returns a [`reqwest::Error`] when the client cannot set up its TLS.
    pub(crate) fn build(cargo: Cargo) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(TIMEOUT)
            .build()?;

        Ok(Self {
            client,
            cargo,
            pace: Pace::new(INTERVAL),
        })
    }
}

impl Registry for CratesIo {
    async fn owners(&self, name: &PackageName) -> Result<Ownership, LookUpOwnersError> {
        self.pace.wait().await;

        let unavailable = |source: reqwest::Error| LookUpOwnersError::RegistryUnavailable {
            source: std::io::Error::other(source),
        };

        let response = self
            .client
            .get(format!("{CRATES}/{name}/owners"))
            .send()
            .await
            .map_err(unavailable)?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(unavailable)?;

        ownership(status, &body)
    }

    async fn add(&self, root: &CargoRoot, name: &PackageName, owner: &CrateOwner) -> Addition {
        self.pace.wait().await;

        let launched = self
            .cargo
            .invocation(root)
            .args(["owner", "--add", owner.get(), name.get()])
            .launch()
            .await;

        match launched {
            Ok(execution) => addition(&execution),
            Err(error) => Addition::Failed {
                details: error.to_string(),
            },
        }
    }
}

/// The list of owners that crates.io reports for a crate
#[derive(Deserialize)]
struct Owners {
    /// The owners of the crate, users and teams alike
    users: Vec<Owner>,
}

/// One owner that crates.io reports for a crate
#[derive(Deserialize)]
struct Owner {
    /// The login of the owner, which `cargo owner --add` takes as well
    login: String,
}

/// Returns what an answer of crates.io says about the owners of a crate
///
/// # Errors
///
/// Returns [`UnexpectedStatus`][status] when crates.io answers with a
/// status other than the owners or a crate that it does not have. Returns
/// [`UnrecognizedOwners`][owners] when the owners have a shape that the
/// action cannot read.
///
/// [owners]: LookUpOwnersError::UnrecognizedOwners
/// [status]: LookUpOwnersError::UnexpectedStatus
fn ownership(status: u16, body: &str) -> Result<Ownership, LookUpOwnersError> {
    match status {
        OK => {
            let owners: Owners = serde_json::from_str(body)
                .map_err(|source| LookUpOwnersError::UnrecognizedOwners { source })?;

            Ok(Ownership::Published {
                owners: owners
                    .users
                    .into_iter()
                    .map(|owner| CrateOwner::new(owner.login))
                    .collect(),
            })
        }
        NOT_FOUND => Ok(Ownership::Unpublished),
        _ => Err(LookUpOwnersError::UnexpectedStatus {
            status,
            body: body.trim().to_owned(),
        }),
    }
}

/// Returns what a run of `cargo owner --add` says about the addition
///
/// A run that succeeded added the owner. [`failure`] reads a run that
/// failed.
fn addition(execution: &Execution) -> Addition {
    if execution.status().success() {
        return Addition::Added;
    }

    failure(details(&execution.stderr().to_string_lossy()))
}

/// Returns what the error of a failed run of `cargo owner --add` says
///
/// A run that failed with the status `403 Forbidden` of crates.io was
/// refused. Every other failure did not reach the registry or failed there
/// for another reason.
fn failure(details: String) -> Addition {
    if details.contains(FORBIDDEN) {
        Addition::Refused { details }
    } else {
        Addition::Failed { details }
    }
}

/// Returns what cargo wrote about its error
///
/// Cargo writes a line of progress before its error, such as the update of
/// the index. The details start at the line of the error, so the reader of a
/// finding reads the error and not the progress. Text without such a line
/// stays whole.
fn details(stderr: &str) -> String {
    let text = stderr.trim();
    let error = text
        .find(ERROR_MARKER)
        .map_or(text, |start| text.get(start..).unwrap_or(text));

    if error.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        error.to_owned()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// What cargo writes when crates.io refuses an addition
    const REFUSED: &str = "    Updating crates.io index\nerror: failed to invite owners to crate `otter` on registry at https://crates.io\n\nCaused by:\n  the remote server responded with an error (status 403 Forbidden): only owners have permission to modify owners\n";

    /// What cargo writes when it has no token
    const NO_TOKEN: &str = "    Updating crates.io index\nerror: no token found, please run `cargo login`\nor use environment variable CARGO_REGISTRY_TOKEN\n";

    #[test]
    fn crates_io_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CratesIo>();
    }

    #[test]
    fn details_of_an_error_start_at_its_line() {
        let details = details(NO_TOKEN);

        assert_eq!(
            details,
            "error: no token found, please run `cargo login`\nor use environment variable CARGO_REGISTRY_TOKEN"
        );
    }

    #[test]
    fn details_of_nothing_say_so() {
        let details = details("  \n");

        assert_eq!(details, NO_DIAGNOSIS);
    }

    #[test]
    fn details_without_an_error_line_keep_the_text() {
        let details = details("the network is down\n");

        assert_eq!(details, "the network is down");
    }

    #[test]
    fn failure_with_another_error_failed() {
        let addition = failure(details(NO_TOKEN));

        assert!(
            matches!(&addition, Addition::Failed { details } if details.contains("no token found")),
            "expected a failure with the error of cargo, got {addition:?}"
        );
    }

    #[test]
    fn failure_with_the_refusal_of_crates_io_is_refused() {
        let addition = failure(details(REFUSED));

        assert!(
            matches!(&addition, Addition::Refused { details } if details.contains("only owners")),
            "expected a refusal with the error of cargo, got {addition:?}"
        );
    }

    #[test]
    fn owner_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Owner>();
    }

    #[test]
    fn owners_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Owners>();
    }

    #[test]
    fn ownership_of_a_crate_that_crates_io_does_not_have_is_unpublished() {
        let ownership = ownership(
            NOT_FOUND,
            r#"{"errors":[{"detail":"crate `otter` does not exist"}]}"#,
        )
        .unwrap();

        assert_eq!(ownership, Ownership::Unpublished);
    }

    #[test]
    fn ownership_of_a_published_crate_lists_its_users_and_teams() {
        let ownership = ownership(
            OK,
            r#"{"users":[{"kind":"user","id":1,"login":"jdno","name":"Jan David"},{"kind":"team","id":2,"login":"github:aonyx-ai:crates-io","name":"crates-io"}]}"#,
        )
        .unwrap();

        assert_eq!(
            ownership,
            Ownership::Published {
                owners: vec![
                    CrateOwner::new("jdno"),
                    CrateOwner::new("github:aonyx-ai:crates-io"),
                ],
            }
        );
    }

    #[test]
    fn ownership_of_an_unexpected_status_reports_it() {
        let error = ownership(429, "slow down\n").unwrap_err();

        assert!(
            matches!(&error, LookUpOwnersError::UnexpectedStatus { status: 429, body } if body == "slow down"),
            "expected the status and the body, got {error:?}"
        );
    }

    #[test]
    fn ownership_of_an_unreadable_list_reports_it() {
        let error = ownership(OK, r#"{"owners":[]}"#).unwrap_err();

        assert!(
            matches!(&error, LookUpOwnersError::UnrecognizedOwners { .. }),
            "expected an unrecognized list, got {error:?}"
        );
    }

    #[test]
    fn user_agent_names_the_crate_and_its_repository() {
        let expected = format!(
            "aonyx-rakko-check-crate-owners/{} (https://github.com/aonyx-ai/rakko)",
            env!("CARGO_PKG_VERSION")
        );

        assert_eq!(USER_AGENT, expected);
    }
}
