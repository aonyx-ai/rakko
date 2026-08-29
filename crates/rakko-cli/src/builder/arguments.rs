use std::collections::BTreeSet;
use std::num::ParseIntError;

use clap::{Arg, ArgAction, ArgMatches};
use rakko_action::{ArgsSchema, ArgsValues, Argument, ArgumentShape, ArgumentValue, ErasedAction};

/// The value that a run holds for a boolean flag that the user gave
///
/// An action reads the value into a `bool`, and this is the text that the
/// standard library reads as true.
const TRUE: &str = "true";

/// Returns the flags that render the arguments of an action
///
/// Every argument becomes one long flag that carries the name of the
/// argument. The projection gives no short flag: one letter is a name that
/// only one argument in the whole fleet can hold, and no action has a claim
/// on it.
// cli[impl argument.flag]
pub(super) fn render(schema: &ArgsSchema) -> Vec<Arg> {
    schema.arguments().iter().map(flag).collect()
}

/// Returns the values that a run holds for the arguments of an action
///
/// A flag that the user left out gets no value, because the action decides
/// what an absent value means. A flag that the user gave carries the text
/// that the user wrote, so the action reads the same text that a run from a
/// test or from a scheduler would give it.
///
/// # Panics
///
/// Panics when the matches did not come from the flags that [`render`] built
/// for this schema.
// cli[impl argument.values]
pub(super) fn collect(schema: &ArgsSchema, matches: &ArgMatches) -> ArgsValues {
    ArgsValues::new(schema.arguments().iter().filter_map(|argument| {
        value(argument, matches).map(|value| (argument.name().clone(), value))
    }))
}

/// Stops the harness when an action declares a reserved argument
///
/// The command line holds flags of its own, and a user reaches every one of
/// them in the command of an action. An argument with such a name would take
/// a name that means something else in every other command of the fleet.
/// Only a change of the action corrects that, so the failure happens where
/// the harness mounts the action and not where a user runs it.
///
/// # Panics
///
/// Panics when an argument of an action carries the name of a flag of the
/// command line, and reports the action and the argument.
// cli[impl mount.reserved]
pub(super) fn refuse_reserved(actions: &[Box<dyn ErasedAction>]) {
    let reserved = reserved();

    for action in actions {
        let schema = action.arguments();

        for argument in schema.arguments() {
            assert!(
                !reserved.contains(argument.name().get()),
                "the action '{}' declares the argument '{}', which the command line carries itself",
                action.name(),
                argument.name(),
            );
        }
    }
}

/// Returns the flag that renders one argument
// cli[impl argument.flag]
// cli[impl argument.help]
// cli[impl argument.switch]
// cli[impl argument.value]
fn flag(argument: &Argument) -> Arg {
    let name = argument.name().get().to_owned();
    let flag = Arg::new(name.clone())
        .long(name.clone())
        .help(argument.documentation().get().to_owned());

    match argument.shape() {
        ArgumentShape::Boolean => flag.action(ArgAction::SetTrue),
        ArgumentShape::Integer => flag.value_name(placeholder(&name)).value_parser(integer),
        // A path and a text both travel as text, so they render alike. A path
        // that is not valid UTF-8 reaches no action, and the parser reports
        // it.
        ArgumentShape::Path | ArgumentShape::Text => flag.value_name(placeholder(&name)),
    }
}

/// Returns the text of a value that is a whole number
///
/// The value travels as the user wrote it, because the action reads it into a
/// type of its own. This function answers whether the value is a number at
/// all, so that a user who wrote something else gets the usage message of the
/// command line instead of an error from the action.
///
/// # Errors
///
/// Returns an error when the value is not a whole number.
// cli[impl argument.integer]
fn integer(value: &str) -> Result<String, ParseIntError> {
    value.parse::<i64>()?;

    Ok(value.to_owned())
}

/// Returns the name that the help shows for the value of an argument
///
/// The name comes from the argument, so a command that Rakko builds reads
/// like a command that a person wrote.
fn placeholder(name: &str) -> String {
    name.to_uppercase().replace('-', "_")
}

/// Returns the names of the flags that the command line carries itself
///
/// The names come from the command that the builder assembles, so a flag that
/// arrives with a new version of a dependency reaches this list without a
/// change here. The command is built first, because the parser adds the flag
/// for the help while it builds.
fn reserved() -> BTreeSet<String> {
    let mut command = super::shell();
    command.build();

    command
        .get_arguments()
        .map(|argument| argument.get_id().as_str().to_owned())
        .collect()
}

/// Returns the value that a run holds for one argument
///
/// # Panics
///
/// Panics when the matches did not come from the flag that [`flag`] built for
/// this argument.
// cli[impl argument.values]
// cli[impl argument.absent]
fn value(argument: &Argument, matches: &ArgMatches) -> Option<ArgumentValue> {
    let name = argument.name().get();

    match argument.shape() {
        ArgumentShape::Boolean => matches.get_flag(name).then(|| ArgumentValue::new(TRUE)),
        ArgumentShape::Integer | ArgumentShape::Path | ArgumentShape::Text => matches
            .get_one::<String>(name)
            .cloned()
            .map(ArgumentValue::new),
    }
}
