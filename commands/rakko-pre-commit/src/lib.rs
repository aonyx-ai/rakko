#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

/// The command, the arguments that a run reads, and the errors that stop a run
pub mod pre_commit;

pub use self::pre_commit::{PreCommit, PreCommitArgs, PreCommitError};
