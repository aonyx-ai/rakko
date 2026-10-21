//! The flag that holds cargo to the lockfile of a workspace
//!
//! Cargo records the versions that it resolved for a workspace in
//! `Cargo.lock`. A build whose manifests ask for something that the lockfile
//! does not hold resolves the dependencies again and rewrites the file. A job
//! that only examines the project would then change a tracked file, and it
//! would examine a resolution that is not the one in the history of the
//! project.
//!
//! The flag of this module forbids that rewrite, and cargo refuses the job
//! instead. [`refused`][refused] tells such a refusal apart from every other
//! failure of a run, so that a caller can say that the lockfile needs an
//! update instead of reporting an output that it cannot read.
//!
//! # Examples
//!
//! ```no_run
//! use rakko_action::ProjectRoot;
//! use rakko_cargo::{Cargo, lockfile};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let cargo = Cargo::resolve(ProjectRoot::new("/home/otter/project".into())).await?;
//!
//! for root in cargo.roots().await? {
//!     let execution = cargo
//!         .invocation(&root)
//!         .args(["check", lockfile::LOCKED])
//!         .run()
//!         .await?;
//!
//!     if lockfile::refused(&execution.stderr().to_string_lossy()) {
//!         println!("{}: the lockfile needs an update", root.directory().display());
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [refused]: crate::lockfile::refused

/// The flag that forbids cargo to write the lockfile of a workspace
///
/// Cargo builds the versions that the lockfile holds. It ends without
/// success when the build needs another version, and when the lockfile is
/// missing. Cargo does not give a flag of its own command line to a
/// subcommand that another program supplies, such as clippy or nextest, so
/// the flag goes after the name of the subcommand.
pub const LOCKED: &str = "--locked";

/// The words that cargo writes when the flag stops it
///
/// Cargo says the refusal in more than one way: a lockfile that needs an
/// update, one that it cannot update, and one that it cannot create. Each of
/// these sentences ends with these words, in the releases of cargo before
/// 1.99 as well.
const REFUSAL: &str = "--locked was passed to prevent this";

/// Returns whether cargo refused a run because it would have to write the
/// lockfile
///
/// The text is what a run wrote to its standard error stream. Cargo writes
/// the refusal there. A program that runs cargo for itself, such as
/// cargo-deny, writes the words of cargo into its own log on that stream.
/// The answer is `false` for every other failure, and for a run without
/// [`LOCKED`].
pub fn refused(stderr: &str) -> bool {
    stderr.contains(REFUSAL)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn refused_with_another_failure_answers_no() {
        let refused = refused("error: failed to parse manifest at `/project/Cargo.toml`\n");

        assert!(!refused);
    }

    #[test]
    fn refused_with_the_log_of_cargo_deny_answers_yes() {
        let refused = refused(
            r#"{"fields":{"level":"ERROR","message":"failed to fetch crates: error: cannot update the lock file /project/Cargo.lock because --locked was passed to prevent this\nhelp: to generate the lock file without accessing the network, remove the --locked flag and use --offline instead.\n"},"type":"log"}"#,
        );

        assert!(refused);
    }

    #[test]
    fn refused_with_the_words_about_a_lockfile_to_create_answers_yes() {
        let refused = refused(
            "error: cannot create the lock file /project/Cargo.lock because --locked was passed to prevent this\n",
        );

        assert!(refused);
    }

    #[test]
    fn refused_with_the_words_about_a_lockfile_to_update_answers_yes() {
        let refused = refused(
            "error: cannot update the lock file /project/Cargo.lock because --locked was passed to prevent this\n",
        );

        assert!(refused);
    }

    #[test]
    fn refused_with_the_words_of_an_older_cargo_answers_yes() {
        let refused = refused(
            "error: the lock file /project/Cargo.lock needs to be updated but --locked was passed to prevent this\n",
        );

        assert!(refused);
    }
}
