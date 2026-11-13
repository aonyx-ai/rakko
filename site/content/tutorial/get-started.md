+++
title = "Get Started"
description = "Set up a maintenance command line for a Rust project and run your first action."
weight = 10
+++

Give a Rust project its own maintenance command line. You will write a
harness, a small binary crate that runs maintenance actions. You will add
three Rust actions for formatting, linting, and tests. Then you will install
their tools and run the commands.

At the end, your project has these parts:

- A [mise] task that builds and runs the harness.
- A harness with `format-rust`, `lint-rust`, and `test-rust` commands.
- A generated [mise] file that pins each external tool to an exact version.

You need basic Rust knowledge, [mise], and Rust installed for the first
`cargo new` command. [Mise][mise] installs the pinned toolchains and external tools
in the steps below.

## Create a Project

Start with a new Rust project:

```sh
cargo new otter
cd otter
```

If you use an existing Rust project, start in its root directory.
Run the remaining steps from that directory.

## Mark the Root of the Project

The harness finds the project root by searching upward from your current
directory for `.config/rakko.toml`. The first directory that contains the
file is the root. Rakko only looks for the file, so it can stay empty.

Create the file:

```toml,name=.config/rakko.toml
# This file marks the root of the project for Rakko.
```

Actions resolve and report paths relative to this root.

## Pin the Toolchains and Add the Task

A toolchain is a compiler or runtime for your project. Your project pins
its toolchain versions. Each action accepts a range of versions and uses the
version that your project pins.

The three actions need two Rust toolchains: a stable release with clippy and
rustfmt, and a nightly release with rustfmt. The formatter uses nightly
features. Pin the nightly by date so that every machine uses the same release.

A [mise] task builds and runs the harness. The task works from any directory
in the project and on every supported platform.

Create `mise.toml` in the root of the project:

{% raw %}

```toml,name=mise.toml
[tools]
cargo-binstall = "1.27.0"
rust = [
  { version = "1.99.1", components = ["clippy", "rustfmt"] },
  { version = "nightly-2026-11-08", components = ["rustfmt"] },
]

[tasks.rakko]
description = "Run the maintenance actions of this project"
raw_args = true
run = 'cargo run --quiet --manifest-path "{{ config_root }}/tools/rakko/Cargo.toml" --'
```

{% endraw %}

Some actions use tools that [mise] installs through Cargo, such as
cargo-nextest. With `cargo-binstall` available, [mise] can download a prebuilt
binary instead of compiling the tool from source.

`raw_args = true` passes arguments unchanged to the harness. For example,
`--help` shows the help for the harness rather than for [mise].

## Choose the Actions

An action is a Rust crate for one maintenance task. This tutorial uses three
actions that wrap familiar Rust tools.

### Format Rust

`aonyx-rakko-format-rust` adds the `format-rust` command. It runs `cargo fmt`
with your pinned nightly toolchain, so rustfmt can use its unstable formatting
options. By default, it reports formatting problems. With `--fix`, it
rewrites the files and reports each repair.

### Lint Rust

`aonyx-rakko-lint-rust` adds the `lint-rust` command. It runs `cargo clippy` on
every target with every feature enabled, using your pinned stable toolchain. It
reports warnings and errors as findings with their file locations.

### Test Rust

`aonyx-rakko-test-rust` adds the `test-rust` command. It runs your tests with
[cargo-nextest][nextest] on your pinned stable toolchain. The action declares
the exact nextest version, and [mise] installs it. Failed tests become
findings with the panic message and location.

## Write the Harness

The harness is a separate Cargo package in `tools/rakko`, with its own
workspace and `Cargo.lock`. Cargo resolves its dependencies separately from
your project. This keeps maintenance dependencies from raising the minimum
Rust version that your project supports.

Create `tools/rakko/Cargo.toml`:

```toml,name=tools/rakko/Cargo.toml
[workspace]

[package]
name = "harness"
version = "0.1.0"
edition = "2024"
publish = false

[[bin]]
name = "rakko"
path = "src/main.rs"

[dependencies]
aonyx-rakko-format-rust = "0.1"
aonyx-rakko-lint-rust = "0.1"
aonyx-rakko-test-rust = "0.1"
rakko-cli = "0.1"
```

The empty `[workspace]` table keeps the harness out of any parent workspace.
The binary name is `rakko`, which also appears in its help text.

[`rakko-cli`][rakko-cli] builds the command line. Create
`tools/rakko/src/main.rs`, and mount the three actions:

{{ <example
  crate="harness"
  file="src/main.rs"
  region="main"
  name="tools/rakko/src/main.rs" /> }}

To mount an entry is to add it to the harness. `mount` accepts an action, a
bundle, a group, or a command. Each mounted action adds one command with the
action's name.

## Install the Tools and Run the Harness

[Mise][mise] asks you to trust a new project's configuration before it uses
it. Trust the configuration, and install the toolchains:

```sh
mise trust
mise install
```

Now run the harness without a command:

```sh
mise run rakko
```

The first run compiles the harness. Later runs rebuild only what changed.
Without a command, the harness lists its available commands. These include
the three action commands and the built-in `doctor` command.

## Write the Tool Pins

Each action pins its external tools, such as cargo-nextest, to exact
versions. Rakko collects these pins in a generated [mise] file in your project.

Try to run the formatter now:

```sh
mise run rakko -- format-rust
```

The run stops because the generated tool file is missing. The error points
you to `doctor --fix`, which writes the file. Generate the pins and install
the tools:

```sh
mise run rakko -- doctor --fix
mise install
```

`doctor --fix` writes `.config/mise/conf.d/rakko.toml` and runs `mise lock`.
This creates `.config/mise/mise.lock` for the generated pins and `mise.lock`
for the tools in your `mise.toml`. [Mise][mise] reads the generated file alongside
your own configuration. Keep these files in version control. Use
`doctor --fix` to update the generated pins.

## Run an Action

Run the formatter again:

```sh
mise run rakko -- format-rust
```

This time, the action runs and passes. It examines every Cargo workspace in
the project, including the harness.

Now give the action something to find. Open `src/main.rs`, and add four
spaces in front of the line with `println!`. Run the formatter again:

```sh
mise run rakko -- format-rust
```

A finding describes a problem and its location. Here, the action reports a
formatting problem in `src/main.rs`. Run the formatter with `--fix` to repair
it:

```sh
mise run rakko -- format-rust --fix
```

The action formats the file and reports the repair.

Run the other two actions:

```sh
mise run rakko -- lint-rust
mise run rakko -- test-rust
```

In a new project, Clippy reports no findings. The project has no tests yet,
so `test-rust` reports that no tests ran.

## Commit the Harness

The harness builds into `tools/rakko/target`. Add that directory to
`.gitignore`:

```text,name=.gitignore
/target
/tools/rakko/target
```

Commit the harness, the root marker, the [mise] files, and the generated
files:

```sh
git add .
git commit -m "Maintain the project with Rakko"
```

Each contributor and each CI job now runs the same actions, with the same
tools, through `mise run rakko`.

## Next Steps

You have a harness with formatting, linting, and tests. From here, you can:

- [Add a command][add-a-command] for a task that does not fit an action.
- [Keep the harness current][keep-current] as Rakko and its actions release
  new versions.
- [Build an action][build-an-action] for a check that your project needs.
- Read why [the harness is a separate binary][harness-and-commands] and how
  [Rakko pins tools][tools-and-mise].

[add-a-command]: @/how-to/add-a-command.md
[build-an-action]: @/how-to/build-an-action.md
[harness-and-commands]: @/explanation/harness-and-commands.md
[keep-current]: @/how-to/keep-a-harness-current.md
[mise]: https://mise.jdx.dev
[nextest]: https://nexte.st
[rakko-cli]: https://docs.rs/rakko-cli
[tools-and-mise]: @/explanation/tools-and-mise.md
