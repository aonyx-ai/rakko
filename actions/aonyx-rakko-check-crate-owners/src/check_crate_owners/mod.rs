//! The action that checks the owners of the crates of a project
//!
//! This module holds the action, its arguments, and the error that stops a
//! run. The action asks cargo for the crates of every workspace. It asks
//! crates.io for the owners of each crate that cargo can publish. Then it
//! compares them with the owners that the harness gave the action.

mod args;
mod error;
mod operation;

use aonyx_rakko_cargo::{Cargo, CargoRoot, PackageName, Publication};
use rakko::{
    Access, Action, Context, Documented, Finding, Location, Name, Outcome, SkipReason, Summary,
    ToolRequirement, action_name,
};

pub use self::args::CheckCrateOwnersArgs;
pub use self::error::CheckCrateOwnersError;
pub use self::operation::Operation;
use crate::crate_owner::{CrateOwner, OwnerKind};
use crate::crates_io::CratesIo;
use crate::registry::{Addition, Ownership, Registry};

/// The reason of a run whose project holds no crate that cargo can publish to
/// crates.io
const NO_CRATE: &str = "the project holds no crate that cargo can publish to crates.io";

/// The word that a summary counts one crate with
const CRATE: &str = "crate";

/// The word that a summary counts several crates with
const CRATES: &str = "crates";

/// Checks that each crate of a project has every owner on crates.io
///
/// A crate that one person owns depends on that person. When their account
/// becomes unavailable, nobody can publish the crate, add an owner, or
/// configure trusted publishing. The action compares the owners of each crate on
/// [crates.io] with the owners that the harness gives it. It fails when a
/// crate misses one of them.
///
/// The action checks each crate of each workspace of the project that cargo
/// can publish to crates.io, as its manifest says. A manifest without the
/// `publish` key allows the publish, and so does a list of registries that
/// holds `crates-io`. The action asks cargo for those crates. The action
/// skips a crate that crates.io does not have yet. A run that passes gives
/// the skipped crates in its summary, and a run that skips gives them in its
/// reason. A new crate stays off crates.io until cargo publishes it for the
/// first time, and a binary can stay off for good.
///
/// The action reports only a missing owner. An owner that crates.io lists,
/// but that the harness does not give, is no finding. A repair never removes
/// an owner,
/// because a wrong removal can lock everybody out of a crate.
///
/// crates.io adds a team as an owner at once. It sends a user an invitation,
/// and the public list of owners shows the user only after they accept it.
/// So a user who did not accept the invitation yet still counts as missing.
/// The finding says that such an invitation can be the cause. A second run with
/// `fix` asks crates.io again to add such a user. The answer of crates.io
/// becomes a repair or a finding, as for any other addition.
///
/// A run reports by default. Each missing owner becomes one finding, which
/// gives the `cargo owner --add` command that adds the owner. With `fix`,
/// the run runs that command for each missing owner. Cargo uses the
/// credentials that it has for the user that starts the run. crates.io has
/// two kinds of owners, users and teams, and only a user owner of a crate can
/// add owners. For any other user, crates.io refuses the change,
/// and the finding says that a user owner of the crate must run the fix.
/// Each owner that the run added becomes a repair. An owner that the run
/// did not add stays a finding, and the run goes on with the next owner.
///
/// The run reads the owners from the public API of crates.io, which needs no
/// token. It waits one second before each lookup and before each run of
/// `cargo owner --add`, as crates.io asks of an automated client. Cargo sends
/// requests of its own in such a run, such as for the index of the registry.
/// The User-Agent of the action gives the name of this crate and the address
/// of its repository. A project with many crates therefore takes about one
/// second per crate.
///
/// A run declares a read of the project, also with `fix`, because it changes
/// nothing in the project. It changes the owners on crates.io.
///
/// A project that holds no crate that cargo can publish to crates.io skips
/// visibly, and so
/// does a project whose crates crates.io has none of. A run stops with an
/// error when the project pins no stable Rust toolchain that the action
/// accepts. It stops when it cannot discover the workspaces of the project,
/// or when the client for crates.io cannot configure its TLS. It
/// stops when it cannot read the owners of a crate from crates.io.
///
/// # Examples
///
/// A harness gives the action the owners that every crate must have:
///
/// ```
/// use rakko::Bundle;
/// use aonyx_rakko_check_crate_owners::{CheckCrateOwners, CrateOwner};
///
/// let actions = Bundle::default().with(CheckCrateOwners::new([
///     CrateOwner::new("jdno"),
///     CrateOwner::new("github:aonyx-ai:crates-io"),
/// ]));
///
/// assert_eq!(actions.actions().len(), 1);
/// ```
///
/// [crates.io]: https://crates.io
#[derive(Clone, Debug, Documented)]
pub struct CheckCrateOwners {
    /// The owners that every crate of the project must have
    owners: Vec<CrateOwner>,
}

impl CheckCrateOwners {
    /// Creates the action from the owners that every crate must have
    ///
    /// An owner is a user, such as `jdno`, or a team, such as
    /// `github:aonyx-ai:crates-io`. The order of the owners is the order of
    /// the findings for each crate.
    ///
    /// # Examples
    ///
    /// ```
    /// use aonyx_rakko_check_crate_owners::{CheckCrateOwners, CrateOwner};
    ///
    /// let action = CheckCrateOwners::new([CrateOwner::new("jdno")]);
    /// ```
    pub fn new(owners: impl IntoIterator<Item = CrateOwner>) -> Self {
        Self {
            owners: owners.into_iter().collect(),
        }
    }
}

impl Action for CheckCrateOwners {
    type Args = CheckCrateOwnersArgs;

    fn name(&self) -> Name {
        action_name!("check-crate-owners")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![Cargo::requirement()]
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        match drive(context, args.operation(), &self.owners).await {
            Ok(outcome) => outcome,
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run resolves cargo, discovers the workspaces, and examines their
/// crates on crates.io.
///
/// # Errors
///
/// Returns the error of the step that failed. That step is the
/// resolution of cargo, the discovery of the workspaces, the setup of the
/// client, or a lookup of owners.
async fn drive(
    context: &Context,
    operation: Operation,
    owners: &[CrateOwner],
) -> Result<Outcome, CheckCrateOwnersError> {
    let cargo = Cargo::resolve(context)
        .await
        .map_err(|source| CheckCrateOwnersError::UnresolvedCargo { source })?;

    let roots = cargo
        .roots()
        .await
        .map_err(|source| CheckCrateOwnersError::UndiscoveredRoots { source })?;

    let registry =
        CratesIo::build(cargo).map_err(|source| CheckCrateOwnersError::ClientUnavailable {
            source: std::io::Error::other(source),
        })?;

    examine()
        .roots(&roots)
        .owners(owners)
        .operation(operation)
        .registry(&registry)
        .call()
        .await
}

/// Compares the owners of each crate of the roots with the owners that every
/// crate must have
///
/// A crate whose manifest keeps it off crates.io is not asked about. A crate
/// that the registry does not have is skipped, and the summary or the reason
/// of the skip gives its name. In a
/// report, each missing owner becomes a finding. In a repair, the registry
/// adds each missing owner, and each addition becomes a repair or a finding.
///
/// # Errors
///
/// Returns [`UnreadableOwners`][owners] when the registry cannot say who owns
/// a crate.
///
/// [owners]: CheckCrateOwnersError::UnreadableOwners
#[bon::builder]
async fn examine<R: Registry>(
    roots: &[CargoRoot],
    owners: &[CrateOwner],
    operation: Operation,
    registry: &R,
) -> Result<Outcome, CheckCrateOwnersError> {
    let mut findings = Vec::new();
    let mut repairs = Vec::new();
    let mut checked = 0_usize;
    let mut unpublished = Vec::new();

    for root in roots {
        let publishable = root
            .packages()
            .iter()
            .filter(|package| package.publication() == Publication::CratesIo);

        for package in publishable {
            let name = package.name();
            let ownership = registry.owners(name).await.map_err(|source| {
                CheckCrateOwnersError::UnreadableOwners {
                    name: name.clone(),
                    source,
                }
            })?;

            let present = match ownership {
                Ownership::Published { owners } => owners,
                Ownership::Unpublished => {
                    unpublished.push(name.clone());
                    continue;
                }
            };
            checked = checked.saturating_add(1);

            let missing = owners
                .iter()
                .filter(|owner| !present.iter().any(|login| owner.matches(login)));

            for owner in missing {
                match operation {
                    Operation::Report => findings.push(missing_owner(name, owner)),
                    Operation::Repair => match registry.add(root, name, owner).await {
                        Addition::Added => repairs.push(added_owner(name, owner)),
                        Addition::Refused { details } => {
                            findings.push(refused_owner(name, owner, &details));
                        }
                        Addition::Failed { details } => {
                            findings.push(failed_owner(name, owner, &details));
                        }
                    },
                }
            }
        }
    }

    if !findings.is_empty() {
        return Ok(Outcome::Failed { findings, repairs });
    }

    if !repairs.is_empty() {
        return Ok(Outcome::Changed { repairs });
    }

    if checked == 0 {
        return Ok(Outcome::Skipped {
            reason: skip_reason(&unpublished),
        });
    }

    Ok(Outcome::Passed {
        summary: Some(summary(checked, &unpublished)),
    })
}

/// Returns the finding of an owner that a crate misses
///
/// The finding gives the command that adds the owner. For a user, it also
/// says that the user possibly did not accept the invitation yet. crates.io
/// lists a user only after the user accepts the invitation.
fn missing_owner(name: &PackageName, owner: &CrateOwner) -> Finding {
    let message = match owner.kind() {
        OwnerKind::Team => format!(
            "`{name}` misses the owner `{owner}`. Run `cargo owner --add {owner} {name}` to add it."
        ),
        OwnerKind::User => format!(
            "`{name}` misses the owner `{owner}`, or `{owner}` did not accept the invitation yet. Run `cargo owner --add {owner} {name}` to invite the user."
        ),
    };

    finding(message)
}

/// Returns the repair of an owner that the run added to a crate
///
/// crates.io adds a team at once, and it invites a user. So the repair of a
/// user says that the user becomes an owner only after they accept.
fn added_owner(name: &PackageName, owner: &CrateOwner) -> Finding {
    let message = match owner.kind() {
        OwnerKind::Team => format!("added the owner `{owner}` to `{name}`"),
        OwnerKind::User => format!(
            "invited `{owner}` to own `{name}`. The user becomes an owner after they accept the invitation."
        ),
    };

    finding(message)
}

/// Returns the finding of an owner that crates.io refused to add to a crate
fn refused_owner(name: &PackageName, owner: &CrateOwner, details: &str) -> Finding {
    finding(format!(
        "crates.io refused to add `{owner}` to `{name}`. Only a user owner of the crate can add owners, so a user owner must run the fix. Cargo reported: {details}"
    ))
}

/// Returns the finding of an owner that the run did not add to a crate
fn failed_owner(name: &PackageName, owner: &CrateOwner, details: &str) -> Finding {
    finding(format!("failed to add `{owner}` to `{name}`: {details}"))
}

/// Returns a finding about the crates of the project
///
/// The problem is on crates.io and not in a file, so the finding belongs to
/// the whole project. The message gives the name of the crate.
fn finding(message: String) -> Finding {
    Finding::builder()
        .message(message)
        .location(Location::Project)
        .build()
}

/// Returns the reason of a run that checked no crate
///
/// The reason lists the crates that crates.io does not have, so a reader can
/// question a skip that covered more than they expect.
fn skip_reason(unpublished: &[PackageName]) -> SkipReason {
    if unpublished.is_empty() {
        return SkipReason::new(NO_CRATE);
    }

    SkipReason::new(format!(
        "crates.io has no crate of the project yet: {}",
        names(unpublished)
    ))
}

/// Returns the summary of a run that checked crates
///
/// The summary counts the crates, and it lists each crate that crates.io
/// does not have yet. That is the notice of a skipped crate.
fn summary(checked: usize, unpublished: &[PackageName]) -> Summary {
    let word = if checked == 1 { CRATE } else { CRATES };
    let counted = format!("checked {checked} {word}");

    if unpublished.is_empty() {
        return Summary::new(counted);
    }

    Summary::new(format!(
        "{counted}, skipped what crates.io does not have yet: {}",
        names(unpublished)
    ))
}

/// Returns the names of crates as one list
fn names(crates: &[PackageName]) -> String {
    crates
        .iter()
        .map(PackageName::get)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_errors_doc,
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design, and a helper of a test returns the error of the run that it drives. A section on every test would repeat that and give the reader no information."
    )]

    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use aonyx_rakko_cargo::{CargoPackage, Documentation, DocumentedNames};
    use rakko::{Args, ArgsValues, ArgumentValue, argument_name};

    use super::*;
    use crate::crates_io::LookUpOwnersError;

    /// A registry that answers from memory and records each addition
    ///
    /// A crate that the registry does not know stops the run, so a test
    /// shows that a run did not ask about a crate.
    #[derive(Default)]
    struct Memory {
        /// The owners of each crate that the registry has
        owners: HashMap<String, Ownership>,

        /// The answer to the addition of each owner, which is `Added` when
        /// it is not here
        answers: HashMap<String, Addition>,

        /// Each crate and owner that the run added
        added: Mutex<Vec<(String, String)>>,
    }

    impl Memory {
        /// Gives a crate the owners that crates.io shows for it
        fn published(mut self, name: &str, owners: &[&str]) -> Self {
            self.owners.insert(
                name.to_owned(),
                Ownership::Published {
                    owners: owners.iter().map(|owner| CrateOwner::new(*owner)).collect(),
                },
            );

            self
        }

        /// Makes a crate one that crates.io does not have
        fn unpublished(mut self, name: &str) -> Self {
            self.owners.insert(name.to_owned(), Ownership::Unpublished);

            self
        }

        /// Gives the addition of an owner an answer
        fn answering(mut self, owner: &str, addition: Addition) -> Self {
            self.answers.insert(owner.to_owned(), addition);

            self
        }

        /// Returns each crate and owner that the run added
        fn added(&self) -> Vec<(String, String)> {
            self.added
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }
    }

    impl Registry for Memory {
        async fn owners(&self, name: &PackageName) -> Result<Ownership, LookUpOwnersError> {
            self.owners.get(name.get()).cloned().ok_or_else(|| {
                LookUpOwnersError::UnexpectedStatus {
                    status: 500,
                    body: format!("the test does not know {name}"),
                }
            })
        }

        async fn add(&self, _root: &CargoRoot, name: &PackageName, owner: &CrateOwner) -> Addition {
            self.added
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((name.get().to_owned(), owner.get().to_owned()));

            self.answers
                .get(owner.get())
                .cloned()
                .unwrap_or(Addition::Added)
        }
    }

    /// Returns the owners that the tests ask every crate to have
    fn owners() -> Vec<CrateOwner> {
        vec![
            CrateOwner::new("jdno"),
            CrateOwner::new("github:aonyx-ai:crates-io"),
        ]
    }

    /// Returns a package that cargo can publish
    fn allowed(name: &str) -> CargoPackage {
        CargoPackage::builder()
            .name(PackageName::new(name))
            .documented_names(DocumentedNames::Unique)
            .build()
    }

    /// Returns a package whose manifest keeps it off crates.io
    fn kept_off(name: &str, publication: Publication) -> CargoPackage {
        CargoPackage::builder()
            .name(PackageName::new(name))
            .documented_names(DocumentedNames::Unique)
            .publication(publication)
            .build()
    }

    /// Returns a workspace root that holds the packages
    fn root(packages: Vec<CargoPackage>) -> CargoRoot {
        CargoRoot::builder()
            .directory(PathBuf::from("/home/otter/project"))
            .documentation(Documentation::Testable)
            .packages(packages)
            .build()
    }

    /// Examines the roots against the registry
    async fn examine_with(
        roots: &[CargoRoot],
        operation: Operation,
        registry: &Memory,
    ) -> Result<Outcome, CheckCrateOwnersError> {
        examine()
            .roots(roots)
            .owners(&owners())
            .operation(operation)
            .registry(registry)
            .call()
            .await
    }

    /// Returns the messages of the findings and repairs of an outcome
    fn messages(outcome: &Outcome) -> (Vec<String>, Vec<String>) {
        let text = |findings: &[Finding]| -> Vec<String> {
            findings
                .iter()
                .map(|finding| finding.message().get().to_owned())
                .collect()
        };

        match outcome {
            Outcome::Failed { findings, repairs } => (text(findings), text(repairs)),
            Outcome::Changed { repairs } => (Vec::new(), text(repairs)),
            _ => (Vec::new(), Vec::new()),
        }
    }

    #[test]
    fn access_with_fix_is_read() {
        let context = Context::builder().root("/tmp/project").build();
        let args = CheckCrateOwnersArgs::from_values(&ArgsValues::new([(
            argument_name!("fix"),
            ArgumentValue::new("true"),
        )]))
        .unwrap();

        let access = CheckCrateOwners::new(owners()).access(&context, &args);

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn check_crate_owners_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CheckCrateOwners>();
    }

    #[tokio::test]
    async fn examine_a_crate_that_crates_io_does_not_have_skips_it_with_a_notice() {
        let registry = Memory::default().unpublished("otter");

        let outcome = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Report,
            &registry,
        )
        .await
        .unwrap();

        assert!(
            matches!(&outcome, Outcome::Skipped { reason } if reason.get() == "crates.io has no crate of the project yet: otter"),
            "expected a skip that names the crate, got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn examine_a_crate_whose_manifest_forbids_a_publish_skips_it() {
        let registry = Memory::default();

        let result = examine_with(
            &[root(vec![kept_off("harness", Publication::Forbidden)])],
            Operation::Report,
            &registry,
        )
        .await;

        assert!(
            matches!(&result, Ok(Outcome::Skipped { reason }) if reason.get() == NO_CRATE),
            "expected a skip without a crate, got {result:?}"
        );
    }

    #[tokio::test]
    async fn examine_a_crate_whose_manifest_lists_only_other_registries_skips_it() {
        let registry = Memory::default();

        let result = examine_with(
            &[root(vec![kept_off(
                "internal",
                Publication::OtherRegistries,
            )])],
            Operation::Report,
            &registry,
        )
        .await;

        assert!(
            matches!(&result, Ok(Outcome::Skipped { reason }) if reason.get() == NO_CRATE),
            "expected a skip without a crate, got {result:?}"
        );
    }

    #[tokio::test]
    async fn examine_a_crate_with_an_owner_in_another_case_passes() {
        let registry = Memory::default().published("otter", &["JDNO", "github:aonyx-ai:crates-io"]);

        let outcome = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Report,
            &registry,
        )
        .await
        .unwrap();

        assert!(
            matches!(&outcome, Outcome::Passed { .. }),
            "expected a pass, got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn examine_a_crate_with_an_owner_too_many_passes() {
        let registry = Memory::default().published(
            "otter",
            &["jdno", "github:aonyx-ai:crates-io", "someone-else"],
        );

        let outcome = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Report,
            &registry,
        )
        .await
        .unwrap();

        assert!(
            matches!(&outcome, Outcome::Passed { .. }),
            "expected a pass, got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn examine_a_crate_with_every_owner_passes() {
        let registry = Memory::default().published("otter", &["jdno", "github:aonyx-ai:crates-io"]);

        let outcome = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Report,
            &registry,
        )
        .await
        .unwrap();

        assert!(
            matches!(&outcome, Outcome::Passed { summary: Some(summary) } if summary.get() == "checked 1 crate"),
            "expected a pass, got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn examine_a_crate_without_an_owner_fails_for_it() {
        let registry = Memory::default().published("otter", &["jdno"]);

        let outcome = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Report,
            &registry,
        )
        .await
        .unwrap();

        assert_eq!(
            messages(&outcome),
            (
                vec![
                    "`otter` misses the owner `github:aonyx-ai:crates-io`. Run `cargo owner --add github:aonyx-ai:crates-io otter` to add it."
                        .to_owned()
                ],
                Vec::new()
            )
        );
    }

    #[tokio::test]
    async fn examine_a_crate_without_an_owner_in_a_report_adds_nothing() {
        let registry = Memory::default().published("otter", &["jdno"]);

        examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Report,
            &registry,
        )
        .await
        .unwrap();

        assert!(registry.added().is_empty(), "got {:?}", registry.added());
    }

    #[tokio::test]
    async fn examine_crates_of_which_crates_io_has_some_names_the_others_in_the_summary() {
        let registry = Memory::default()
            .published("otter", &["jdno", "github:aonyx-ai:crates-io"])
            .unpublished("seal")
            .unpublished("walrus");

        let outcome = examine_with(
            &[
                root(vec![allowed("otter"), allowed("seal")]),
                root(vec![allowed("walrus")]),
            ],
            Operation::Report,
            &registry,
        )
        .await
        .unwrap();

        assert!(
            matches!(&outcome, Outcome::Passed { summary: Some(summary) } if summary.get() == "checked 1 crate, skipped what crates.io does not have yet: seal, walrus"),
            "expected a summary that names the skipped crates, got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn examine_with_a_fix_adds_each_missing_owner() {
        let registry = Memory::default().published("otter", &[]);

        examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Repair,
            &registry,
        )
        .await
        .unwrap();

        assert_eq!(
            registry.added(),
            [
                ("otter".to_owned(), "jdno".to_owned()),
                ("otter".to_owned(), "github:aonyx-ai:crates-io".to_owned()),
            ]
        );
    }

    #[tokio::test]
    async fn examine_with_a_fix_of_one_owner_changes_the_crate() {
        let registry = Memory::default().published("otter", &["jdno"]);

        let outcome = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Repair,
            &registry,
        )
        .await
        .unwrap();

        assert!(
            matches!(&outcome, Outcome::Changed { repairs } if repairs.len() == 1),
            "expected one repair, got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn examine_with_a_fix_reports_each_added_owner_as_a_repair() {
        let registry = Memory::default().published("otter", &[]);

        let outcome = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Repair,
            &registry,
        )
        .await
        .unwrap();

        assert_eq!(
            messages(&outcome),
            (
                Vec::new(),
                vec![
                    "invited `jdno` to own `otter`. The user becomes an owner after they accept the invitation."
                        .to_owned(),
                    "added the owner `github:aonyx-ai:crates-io` to `otter`".to_owned(),
                ]
            )
        );
    }

    #[tokio::test]
    async fn examine_with_a_fix_that_crates_io_refuses_says_that_an_owner_must_run_it() {
        let registry = Memory::default().published("otter", &["jdno"]).answering(
            "github:aonyx-ai:crates-io",
            Addition::Refused {
                details: "status 403 Forbidden".to_owned(),
            },
        );

        let outcome = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Repair,
            &registry,
        )
        .await
        .unwrap();

        assert_eq!(
            messages(&outcome),
            (
                vec![
                    "crates.io refused to add `github:aonyx-ai:crates-io` to `otter`. Only a user owner of the crate can add owners, so a user owner must run the fix. Cargo reported: status 403 Forbidden"
                        .to_owned()
                ],
                Vec::new()
            )
        );
    }

    #[tokio::test]
    async fn examine_with_a_fix_that_fails_reports_the_failure_beside_the_repairs() {
        let registry = Memory::default().published("otter", &[]).answering(
            "jdno",
            Addition::Failed {
                details: "no token found".to_owned(),
            },
        );

        let outcome = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Repair,
            &registry,
        )
        .await
        .unwrap();

        assert_eq!(
            messages(&outcome),
            (
                vec!["failed to add `jdno` to `otter`: no token found".to_owned()],
                vec!["added the owner `github:aonyx-ai:crates-io` to `otter`".to_owned()]
            )
        );
    }

    #[tokio::test]
    async fn examine_with_an_unreadable_list_of_owners_stops() {
        let registry = Memory::default();

        let error = examine_with(
            &[root(vec![allowed("otter")])],
            Operation::Report,
            &registry,
        )
        .await
        .unwrap_err();

        assert!(
            matches!(&error, CheckCrateOwnersError::UnreadableOwners { name, .. } if name.get() == "otter"),
            "expected the crate whose owners the run could not read, got {error:?}"
        );
    }

    #[test]
    fn missing_owner_that_is_a_user_says_that_an_invitation_can_wait() {
        let finding = missing_owner(&PackageName::new("otter"), &CrateOwner::new("jdno"));

        assert_eq!(
            finding.message().get(),
            "`otter` misses the owner `jdno`, or `jdno` did not accept the invitation yet. Run `cargo owner --add jdno otter` to invite the user."
        );
    }

    #[test]
    fn name_is_check_crate_owners() {
        let name = CheckCrateOwners::new(owners()).name();

        assert_eq!(name.get(), "check-crate-owners");
    }

    #[test]
    fn summary_of_several_crates_counts_them() {
        let summary = summary(2, &[]);

        assert_eq!(summary.get(), "checked 2 crates");
    }
}
