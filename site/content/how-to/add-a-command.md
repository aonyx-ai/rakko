+++
title = "Add a Command"
description = "Add a custom command for tasks such as cleanup, setup, or a development server."
weight = 50
+++

Add a custom command for work that does not fit an action. An action runs
once and reports an outcome. A development server runs until you stop it,
while a cleanup task removes files rather than reporting findings. A
command lets you write that behavior and expose it beside your actions.

This guide adds `clean`, a command that removes the build output of the
project. You need a working harness. If you do not have one yet, follow
[Get Started][get-started] first.

If you want to run several actions together, [form a group][form-a-group].
Rakko then runs the members and reports their results for you.

## Add the Contract Crate

The command uses the types of the contract crate, `rakko`. Add it to the
dependencies of the harness:

```toml,name=tools/rakko/Cargo.toml
[dependencies]
rakko = "0.1"
rakko-cli = "0.1"
rakko-format-rust = "0.1"
rakko-lint-rust = "0.1"
rakko-test-rust = "0.1"
```

`rakko-cli` also exports [Clawless][clawless], the command-line framework.
Your harness does not need a separate [Clawless][clawless] dependency.

## Write the Command

Put the command in its own module so that `main.rs` stays a short list of
mounts. Create `tools/rakko/src/clean.rs` with these imports:

{{ <example
  crate="clean-command"
  file="src/clean.rs"
  region="imports"
  name="tools/rakko/src/clean.rs" /> }}

A command is a type that implements the trait `Command` of `rakko-cli`. Add
the command below the imports:

{{ <example
  crate="clean-command"
  file="src/clean.rs"
  region="command"
  name="tools/rakko/src/clean.rs" /> }}

The code has these parts:

- The `Documented` derive uses the first paragraph of the doc comment as help
  text. Write that paragraph as one sentence.
- The name is what users type after `mise run rakko --`. It starts with a
  lowercase ASCII letter and contains lowercase ASCII letters, digits, and
  single hyphens. `action_name!` rejects invalid names at compile time.
- `Args` is the unit type because `clean` takes no arguments. Commands
  declare arguments in the same way as actions. [Build an
  Action][add-an-argument] shows how those declarations become flags.
- `run` is asynchronous code that receives the project context and the
  [Clawless][clawless] context. Use the [Clawless][clawless] context to write
  output.

`run` returns success or an error. Success produces exit code 0. An error
produces exit code 2 and a report of the error and its causes. The `?`
operator stops the command when a step fails, such as removing the directory.

## Mount the Command

Declare the module in `tools/rakko/src/main.rs`, and give the command to
`mount`:

{{ <example
  crate="clean-command"
  file="src/main.rs"
  region="main"
  name="tools/rakko/src/main.rs" /> }}

Actions, groups, and commands share one namespace. If two mounted entries
have the same name, the harness stops at startup and reports the duplicate.
The names `doctor` and `help` are reserved.

## Run the Command

Run the command like an action:

```sh
mise run rakko -- clean
```

The help of the harness lists the command beside the actions, with the
sentence of its doc comment:

```sh
mise run rakko -- --help
```

Custom commands use the same environment checks as actions. If the tool pins
do not match the mounted actions, the command stops and asks you to run
`mise run rakko -- doctor --fix`.

## Run Actions From a Command

A command can run actions, for example to repeat a check when files change.
Pass the required actions from the harness to the command. Commands cannot
look up mounted actions themselves.

Run each action through `ErasedAction::run` in `rakko`. This path also skips
an action when its tool has no build for the current platform.

`doctor` reads tool declarations from mounted actions and group members.
If only a custom command holds an action, `doctor` cannot see it. Mount that
action as well so that `doctor --fix` includes its tools.

[add-an-argument]: @/how-to/build-an-action.md#add-an-argument
[clawless]: https://clawless.rs
[form-a-group]: @/how-to/form-a-group.md
[get-started]: @/tutorial/get-started.md
