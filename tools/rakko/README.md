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

The `pre-commit` command runs the actions that guard a commit, in the order
that the harness names them, and reports each of them on its own. The hook
runs it with `--fix`, so that the formatters rewrite the tree before the
checks read it:

```console
mise run rakko -- pre-commit --fix
```

## Layout

The package sits outside the workspace of the repository, so it resolves its
dependencies on its own and carries its own `Cargo.lock`. The binary is named
`rakko`, and the package is named `harness`, because a package that depends on
the `rakko` crate cannot carry that name as well.
