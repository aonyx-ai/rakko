#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

/// The group of the actions that guard a commit
mod pre_commit;

pub use self::pre_commit::PreCommit;
