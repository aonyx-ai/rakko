//! The action that formats the TypeScript of a project

mod format_typescript;
mod observation;
mod oxfmt;
mod problem;

pub use self::format_typescript::{
    FormatTypeScript, FormatTypeScriptArgs, FormatTypeScriptError, Operation,
};
