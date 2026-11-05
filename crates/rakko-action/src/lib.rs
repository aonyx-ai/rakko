//! The former name of the contract that every Rakko action depends on
//!
//! The contract now lives in the [`rakko`] crate. This crate re-exports every
//! item of `rakko` under its old path, so a crate that imports `rakko_action`
//! still compiles. A path through this crate and a path through `rakko` lead
//! to the same item. So a harness can mount an action that imports
//! `rakko_action` beside an action that imports `rakko`.
//!
//! New code imports the contract from `rakko`. Rakko removes this crate when
//! no crate imports it.

pub use rakko::*;
