use std::ffi::OsString;

use clap::Command;
use clawless::output::OutputFlags;

/// The description that the command line shows above its help
const ABOUT: &str = "Runs the maintenance actions of this project";

/// The name that the command line carries
///
/// A reader sees the name of the harness binary instead of this name, because
/// clap takes the name of a usage line from the command that started the
/// process. This name is the fallback for the places that have nothing else.
const NAME: &str = "rakko";

/// Creates the builder of the command line of a harness
///
/// A harness calls this function in its `main`, and it runs what it gets back.
///
/// # Examples
///
/// ```no_run
/// rakko_cli::builder().run();
/// ```
// cli[impl builder.create]
#[must_use]
pub fn builder() -> Builder {
    Builder
}

/// The command line of a harness
///
/// A harness is the small binary that a project runs to maintain itself. This
/// type is the whole surface that the harness touches: the harness creates it
/// with [`builder`], and then it calls [`run`].
///
/// The command line of every project has the same shape, because one type
/// builds all of them. A harness cannot name a flag of its own.
///
/// [`run`]: Builder::run
#[derive(Debug)]
pub struct Builder;

impl Builder {
    /// Runs the command line against the arguments of the process
    ///
    /// A request for help, and a request that the command line cannot read,
    /// end the process in this method. The user gets the message of the parser
    /// and the process gets an exit code.
    // cli[impl builder.run]
    pub fn run(self) {
        if let Err(error) = self.run_from(std::env::args_os()) {
            error.exit();
        }
    }

    /// Runs the command line against the arguments that it gets
    ///
    /// The method takes the arguments as a parameter, so that a test drives
    /// the command line without the arguments of the test process.
    ///
    /// # Errors
    ///
    /// Returns the error of the parser when the arguments do not describe a
    /// run, and when the user asked for help.
    fn run_from<I, T>(self, arguments: I) -> Result<(), clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Clone + Into<OsString>,
    {
        self.command().try_get_matches_from(arguments)?;

        Ok(())
    }

    /// Returns the command line that the builder describes
    ///
    /// The parser behind the command line is an implementation detail. It
    /// appears in no signature that a harness or an action can see.
    // cli[impl command.action]
    // cli[impl command.help]
    // cli[impl command.output]
    fn command(&self) -> Command {
        let command = Command::new(NAME)
            .about(ABOUT)
            .arg_required_else_help(true)
            .subcommand_required(true);

        OutputFlags::augment_command(command)
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use clap::error::ErrorKind;

    use super::*;

    /// Returns the kind of the error that a run of the given arguments gives
    fn error_kind(arguments: &[&str]) -> ErrorKind {
        let error = builder()
            .run_from(arguments)
            .expect_err("expected the run to report an error");

        error.kind()
    }

    // cli[verify builder.create]
    #[test]
    fn builder_returns_a_builder_of_a_command_line() {
        let builder = builder();

        let name = builder.command().get_name().to_owned();

        assert_eq!(name, NAME);
    }

    // cli[verify command.action]
    #[test]
    fn command_refuses_a_run_that_names_no_action() {
        let kind = error_kind(&["rakko", "--json"]);

        assert_eq!(kind, ErrorKind::MissingSubcommand);
    }

    // cli[verify command.output]
    #[test]
    fn command_carries_the_flags_that_control_the_output() {
        let command = builder().command();

        let flags: Vec<&str> = command
            .get_arguments()
            .map(|argument| argument.get_id().as_str())
            .collect();

        assert!(
            ["quiet", "verbose", "json"]
                .iter()
                .all(|flag| flags.contains(flag))
        );
    }

    // cli[verify command.help]
    #[test]
    fn run_from_shows_the_help_for_a_run_without_arguments() {
        let error = builder()
            .run_from(["rakko"])
            .expect_err("expected the run to report an error");

        let help = error.render().to_string();

        assert!(help.contains(ABOUT));
    }

    // cli[verify builder.run]
    #[test]
    fn run_from_reports_a_run_that_it_cannot_read() {
        let kind = error_kind(&["rakko", "--unknown"]);

        assert_eq!(kind, ErrorKind::UnknownArgument);
    }
}
