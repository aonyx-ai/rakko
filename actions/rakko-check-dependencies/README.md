# Rakko Check Dependencies

`rakko-check-dependencies` provides `check-dependencies`, an action for [Rakko]
that examines the dependencies of a project.

A project depends on code that it did not write, and [cargo-deny] answers three
questions about that code. Does every package carry a license that the project
accepts? Do the packages come from a registry that the project trusts? Does the
graph hold a shape that the project banned? Examples are two versions of one
package and a version requirement that accepts any future release. The action
declares the version of cargo-deny that it runs. It starts that version, which
[mise] installed, and translates what cargo-deny reported into the outcome of
the run.

Cargo-deny is its own program, and not a subcommand that cargo carries, so the
action starts it directly. Cargo answers a different question for the same run:
which workspaces make up the project. Cargo-deny checks one workspace at a time,
so a run asks cargo for the workspace roots and checks each of them.

Cargo-deny weighs everything that it reports with the level that the
configuration of the project gave the check. `deny` marks a shape that must not
appear. `warn` marks one that a maintainer wants to read about. `allow` marks
one that the project does not care about. An error therefore fails a run. A
warning does not. A passing run says how many warnings it read, so the middle
level keeps the meaning that the project gave it.

A finding names the workspace that the error came from and no path. The place
that cargo-deny underlines is a line of a lock file, or of a manifest that lies
in the registry cache of the machine. A path in a finding therefore points to a
file that a reader cannot open.

A run only reports, so the action takes no argument. The action applies to a
project that holds a manifest of cargo, and it skips visibly otherwise.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `rakko-check-dependencies` to
the harness of the project. Then mount the action in the `main` of the harness:

```rust
use rakko_check_dependencies::CheckDependencies;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(CheckDependencies).run()
}
```

The action becomes the `check-dependencies` command of the harness.

## Asynchronous Runtime

A run starts a program and waits for it. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `CheckDependencies`][docs] on docs.rs describes the tools
that the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[cargo-deny]: https://embarkstudios.github.io/cargo-deny/
[docs]: https://docs.rs/rakko-check-dependencies/latest/rakko_check_dependencies/struct.CheckDependencies.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[tokio]: https://tokio.rs
