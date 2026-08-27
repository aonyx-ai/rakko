//! The contract that every Rakko action depends on
//!
//! An action is a library crate that does one piece of project maintenance.
//! This crate holds what every action and every harness shares: the name that
//! identifies an action, the context that an action reads when it runs, the
//! finding that an action produces, and the outcome that the run returns.
//!
//! The crate stays small on purpose. Everything in it reaches every action and
//! every project that mounts one, so an addition here is a change for all of
//! them.

/// Types for the data that an action reads when it runs
pub mod context;
/// Types for a problem that an action found in a project
pub mod finding;
/// Types for the identifier of an action
pub mod name;
/// Types for the result of an action run
pub mod outcome;

pub use context::{CacheDirectory, ConfigDirectory, Context, Layout, ProjectRoot};
pub use finding::{
    ColumnNumber, FilePath, Finding, FindingMessage, LineNumber, Location, Position,
};
pub use name::{Name, ParseNameError};
pub use outcome::{Outcome, SkipReason};
