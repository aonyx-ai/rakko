//! The action that type checks the TypeScript of a project
//!
//! [Tsc] does the work. It reads the `tsconfig.json` of the project and
//! collects the files that the configuration selects. It reports every rule of
//! the language that the code breaks. So a run of the action agrees with an
//! editor and with a contributor that runs tsc bare. The action starts the tsc
//! that [mise] installed for the project. It uses the version that the project
//! pinned. It translates what tsc reported into the run outcome.
//!
//! A run only reports, so the action takes no argument. It emits nothing
//! either. Tsc builds a project as well as it checks one, and a check that left
//! output behind would write files that nobody asked for.
//!
//! Type checking is not linting. The compiler asks whether the program holds
//! together, and a linter asks whether the code does something that the project
//! turned a rule against. A design that rests on the compiler to reject a wrong
//! composition has no guarantee at all while no check runs the compiler. That
//! check is what this action is for.
//!
//! The diagnostics of tsc are the answer. The status of the process decides
//! only whether a run without any is trusted. Tsc reports a diagnostic for
//! everything that it finds, including a configuration that it refuses and a
//! project that it finds no configuration of. So a run that reported nothing
//! and ended without success stopped for a reason of its own. It stops the
//! action instead of passing.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that, and
//! it panics without one.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_check_typescript::CheckTypeScript;
//!
//! let actions = Bundle::default().with(CheckTypeScript);
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs
//! [tsc]: https://www.typescriptlang.org/docs/handbook/compiler-options.html

mod check_typescript;
mod diagnostic;
mod tsc;

pub use self::check_typescript::{CheckTypeScript, CheckTypeScriptError};
pub use self::tsc::report::ReadReportError;
