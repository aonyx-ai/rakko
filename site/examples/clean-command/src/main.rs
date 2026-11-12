//! The example harness of the guide "Add a Command"
//!
//! The harness mounts three actions and a command of its own. The site shows the
//! regions of this file and of `clean.rs` between the `// region:` and the
//! `// endregion:` comments. Cargo compiles and tests both files, so the code
//! on the site always compiles against the current crates of Rakko.

// region: main
mod clean;

use clean::Clean;

/// Builds the command line of the project and runs it
fn main() -> std::process::ExitCode {
    rakko_cli::builder()
        .mount(rakko_format_rust::FormatRust)
        .mount(rakko_lint_rust::LintRust)
        .mount(rakko_test_rust::TestRust)
        .mount(Clean)
        .run()
}
// endregion: main
