#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

/// The command and the errors that stop a run
pub mod pre_commit;

pub use self::pre_commit::{PreCommit, PreCommitError};
