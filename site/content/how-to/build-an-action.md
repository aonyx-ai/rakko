+++
title = "Build an Action"
description = "Write a reusable maintenance action with a --fix flag and add it to your harness."
weight = 10
+++

Build `check-readme`, an action that reports a missing `README.md` and creates
one with `--fix`. You will write a Rust library crate and mount it in your
harness. Mounting adds a command with the action's name.

You need a working harness. If you do not have one, follow
[Get Started][get-started] first.

## Create the Crate

An action is an ordinary library crate that depends on the contract crate,
`rakko`:

```sh
cargo new --lib otter-check-readme
cd otter-check-readme
cargo add rakko
```

The contract defines the interfaces that actions and harnesses share. The
action does not need [`rakko-cli`][rakko-cli], which builds the command line.
This keeps the action independent of a command-line framework.

Everything that this action needs comes from `rakko`. Put these imports at the
top of `src/lib.rs`:

{{ <example crate="check-readme" file="src/lib.rs" region="imports" /> }}

## Add an Argument

Arguments let callers control a run, for example with a `--fix` flag.
Define them in a type that implements `Args`. Add this type to `src/lib.rs`:

{{ <example crate="check-readme" file="src/lib.rs" region="args" /> }}

`schema` describes each argument's name, shape, and help text. The command
line turns a boolean argument into a switch, such as `--fix`. Other shapes
become flags that take a value.

`from_values` converts the values for a run into your argument type. Your
type decides what an omitted value means. If conversion fails, it returns
`ReadArgsError` before the action runs.

The command line reserves names such as `json`, `quiet`, and `verbose`.
If an action uses a reserved argument name, the harness stops at startup
and identifies the action and argument.

An action that reads no arguments uses the unit type, `type Args = ();`.

## Write the Action

An action is a type that implements `Action`. Add it to `src/lib.rs`:

{{ <example crate="check-readme" file="src/lib.rs" region="action" /> }}

The code has these parts:

- The `Documented` derive uses the first paragraph of the doc comment as
  command help. Write one sentence without Markdown or links. A type without
  a doc comment does not compile.
- The action name is also its command name. It starts with a lowercase ASCII
  letter and contains lowercase ASCII letters, digits, and single hyphens.
  `action_name!` rejects invalid names at compile time. The names `doctor` and
  `help` are reserved.
- `access` declares whether a run reads or writes the project. In this action,
  `--fix` requires exclusive access. Without it, the run can share the project
  with other readers. This method must not read files or start tools. If the
  arguments cannot determine access, return `Access::Write`, which is also
  the default.
- `run` examines the project and returns an `Outcome`. `Context` supplies
  the project root. Every result, including an error, belongs in the outcome.

### Choose the Outcome

Choose an outcome that describes the run:

- `Outcome::Passed` means that the action found no problems. A `Summary`,
  such as `found README.md`, describes what it examined.
- `Outcome::Failed` carries a `Finding` for each remaining problem. A
  finding describes a problem and its location. `Location` identifies the
  project, a directory, a file, or a position or range within a file. Include
  any completed repairs alongside the remaining findings.
- `Outcome::Changed` means that the action repaired every problem it found.
  Each repair has the same structure as a finding.
- `Outcome::Skipped` means that the action had nothing to examine. Include
  a `SkipReason` for the user.
- `Outcome::Errored` means that an error prevented the action from reaching
  a result.

[Actions and Groups][actions-and-groups] explains how the command line turns
each outcome into output and an exit code.

## Mount It in a Harness

Add your crate to the harness dependencies. Then mount the action in the
harness's `main` function:

{{ <example
  crate="check-readme"
  file="examples/check_readme_harness.rs"
  region="main"
  name="src/main.rs" /> }}

The harness now has a `check-readme` command. Its help includes the doc
comment and the `--fix` flag:

```sh
mise run rakko -- check-readme --help
```

Run the action, and let it repair the project:

```sh
mise run rakko -- check-readme
mise run rakko -- check-readme --fix
```

## Next Steps

- [Run a Tool From an Action][run-a-tool] to use an external linter or formatter.
- [Test an Action][test-an-action] against a temporary project, including its
  access declaration.
- [Form a Group][form-a-group] to run several actions under one command.
- [Publish an Action][publish-an-action] so that others can find and use it.

[actions-and-groups]: @/explanation/actions-and-groups.md
[form-a-group]: @/how-to/form-a-group.md
[get-started]: @/tutorial/get-started.md
[publish-an-action]: @/how-to/publish-an-action.md
[rakko-cli]: https://docs.rs/rakko-cli
[run-a-tool]: @/how-to/run-a-tool.md
[test-an-action]: @/how-to/test-an-action.md
