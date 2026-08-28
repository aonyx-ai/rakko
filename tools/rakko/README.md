# Harness

This package is the harness of the Rakko repository: the one place that states
which maintenance actions run here. It mounts the actions that this repository
uses, and the command line that it builds turns each of them into a command.

## Usage

Run the harness from any directory of the repository:

```console
mise run rakko
```

Where mise supplies its environment, `rakko` is a shortcut for the same
command.

## Layout

The package sits outside the workspace in `crates/`, so it resolves its
dependencies on its own and carries its own `Cargo.lock`. The binary is named
`rakko`, and the package is named `harness`, because a package that depends on
the `rakko` crate cannot carry that name as well.
