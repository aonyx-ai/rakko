//! The action that formats the TypeScript of a project

/// Types for the action that formats the TypeScript of a project
pub mod format_typescript;
/// Types for what one run of oxfmt produced
pub mod observation;
/// Types for the oxfmt that a project runs
pub mod oxfmt;
/// Types for one problem that oxfmt reported about a project
pub mod problem;

pub use self::format_typescript::{FormatTypeScript, FormatTypeScriptArgs, FormatTypeScriptError};
pub use self::observation::Observation;
pub use self::oxfmt::{ObserveOxfmtError, Oxfmt};
pub use self::problem::OxfmtProblem;
