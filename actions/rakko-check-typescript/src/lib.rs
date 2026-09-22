//! The action that type checks the TypeScript of a project
//!
//! [Tsc] does the work: it reads the `tsconfig.json` of the project, collects
//! the files that the configuration selects, and reports every rule of the
//! language that the code breaks, so a run of the action agrees with an editor
//! and with a contributor that runs tsc bare. The action starts the tsc that
//! [mise] installed for the project, at the version that the project pinned,
//! and translates what tsc reported into the outcome of the run.
//!
//! A run only reports, so the action takes no argument. It emits nothing
//! either: tsc builds a project as well as it checks one, and a check that
//! left output behind would write files that nobody asked for.
//!
//! Type checking is not linting. The compiler asks whether the program holds
//! together, and a linter asks whether the code does something that the
//! project turned a rule against. A design that rests on the compiler to
//! reject a wrong composition has no guarantee at all while no check runs the
//! compiler, which is what this action is for.
//!
//! The diagnostics of tsc are the answer, and the status of the process
//! decides only whether a run without any is trusted. Tsc reports a diagnostic
//! for everything that it finds, including a configuration that it refuses and
//! a project that it finds no configuration of, so a run that reported nothing
//! and ended without success stopped for a reason of its own, and it stops the
//! action instead of passing.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that, and
//! it panics without one.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_check_typescript::CheckTypeScript;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckTypeScript)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs
//! [tsc]: https://www.typescriptlang.org/docs/handbook/compiler-options.html

/// Types for the action that type checks the TypeScript of a project
pub mod check_typescript;
/// Types for one thing that tsc reported about a project
pub mod diagnostic;
/// Types for the tsc that a project runs
pub mod tsc;

pub use self::check_typescript::{CheckTypeScript, CheckTypeScriptError};
pub use self::diagnostic::{Category, Diagnostic, Origin};
pub use self::tsc::report::ReadReportError;
pub use self::tsc::{ObserveTscError, Tsc};
