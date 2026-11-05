#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

pub mod pre_commit;

pub use self::pre_commit::PreCommit;
