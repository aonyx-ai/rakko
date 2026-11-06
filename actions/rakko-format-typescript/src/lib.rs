//! The action that formats the TypeScript of a project

/// Types for the action that formats the TypeScript of a project
mod format_typescript;
/// Types for what one run of oxfmt produced
mod observation;
/// Types for the oxfmt that a project runs
mod oxfmt;
/// Types for one problem that oxfmt reported about a project
mod problem;

pub use self::format_typescript::{FormatTypeScript, FormatTypeScriptArgs, FormatTypeScriptError};
