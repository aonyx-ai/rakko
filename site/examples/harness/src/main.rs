//! The example harness of the tutorial "Get Started"
//!
//! The site shows the region of this file between the `// region:` and the
//! `// endregion:` comments. Cargo compiles and tests the file, so the code on
//! the site always compiles against the current crates of Rakko.

// region: main
/// Builds the command line of the project and runs it
fn main() -> std::process::ExitCode {
    rakko_cli::builder()
        .mount(aonyx_rakko_format_rust::FormatRust)
        .mount(aonyx_rakko_lint_rust::LintRust)
        .mount(aonyx_rakko_test_rust::TestRust)
        .run()
}
// endregion: main
