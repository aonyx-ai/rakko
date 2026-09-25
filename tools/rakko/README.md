# Harness

This package is the harness of the Rakko repository: the one place that states
which maintenance actions run here. It mounts the bundles and the actions that
this repository uses, and the command line that it builds turns each of them
into a command. A bundle carries a set of actions that projects adopt
together, so the harness names the bundle instead of each action in it.

It also mounts commands, for a maintenance activity that no single action
describes. A command comes from a crate, as an action does, and `src/main.rs`
names it. When a command runs actions, `src/main.rs` names those actions as
well and gives them to the command.

## Usage

Run the harness from any directory of the repository:

```console
mise run rakko
```

Where mise supplies its environment, `rakko` is a shortcut for the same
command.

## Commands

### `pre-commit`

The command comes from [`rakko-pre-commit`][rakko-pre-commit]. It runs the
actions that guard a commit: the formatters first, in the order in which they
rewrite the tree, and then the checks that read what they wrote. `src/main.rs`
names both lists, and the README of the crate describes what a run does and
when it fails.

```console
mise run rakko -- pre-commit --fix
```

The hook that Git runs before a commit passes `--fix`, so that the formatters
rewrite the files that they can format. A run without the flag reports what a
commit has to repair, and it changes nothing.

### `set-msrv`

The command comes from [`rakko-set-msrv`][rakko-set-msrv]. It sets the
minimum supported Rust version of the repository in the three places that
must agree: the root `Cargo.toml`, the Rust pin in `mise.toml`, and
`mise.lock`. The reason replaces the comment above `rust-version`.

```console
mise run rakko -- set-msrv --msrv 1.89.0 --reason "bon 3.11 requires Rust 1.89"
```

The run then installs the new toolchain through mise and runs check-msrv on
it, and it fails when the check does not pass. The files keep the new version
either way.

## Layout

The package sits outside the workspace of the repository, so it resolves its
dependencies on its own and carries its own `Cargo.lock`. The binary is named
`rakko`, and the package is named `harness`, because a package that depends on
the `rakko` crate cannot carry that name as well.

[rakko-pre-commit]: ../../commands/rakko-pre-commit/README.md
[rakko-set-msrv]: ../../commands/rakko-set-msrv/README.md
