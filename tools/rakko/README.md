# Harness

This package is the harness of the Rakko repository: the one place that states
which maintenance actions run here. It mounts the actions that this repository
uses, and the command line that it builds turns each of them into a command.

It also writes the commands of this repository, for a maintenance activity
that no single action describes. `src/main.rs` names them, and each of them
lives in a module of its own.

## Usage

Run the harness from any directory of the repository:

```console
mise run rakko
```

Where mise supplies its environment, `rakko` is a shortcut for the same
command.

## Commands

### `pre-commit`

The command runs the actions that guard a commit: the formatters first, in the
order in which they rewrite the tree, and then the checks that read what they
wrote. It reports each action as a run of that action alone would, and an
action that found problems does not end it, so one run reports every problem
of the tree.

```console
mise run rakko -- pre-commit --fix
```

The `--fix` flag lets the formatters rewrite the files that they can format.
The hook that Git runs before a commit passes it. A run without the flag
reports what a commit would have to repair, and it changes nothing.

The command fails when an action found problems or stopped, so the commit
waits. A run in which the formatters repaired everything that they found
succeeds, because the repair is what the flag asked for. The commit still
waits in that case: pre-commit compares the tree with what the hook received,
and it stops a commit whose files a hook rewrote.

## Layout

The package sits outside the workspace of the repository, so it resolves its
dependencies on its own and carries its own `Cargo.lock`. The binary is named
`rakko`, and the package is named `harness`, because a package that depends on
the `rakko` crate cannot carry that name as well.
