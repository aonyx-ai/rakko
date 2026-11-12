//! The harness of the guide "Build an Action"
//!
//! A harness is the small binary that a project runs to maintain itself. This
//! one mounts the action `check-readme`, so its command line has the command
//! `check-readme`.

// region: main
use otter_check_readme::CheckReadme;

/// Builds the command line of the project and runs it
fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(CheckReadme).run()
}
// endregion: main
