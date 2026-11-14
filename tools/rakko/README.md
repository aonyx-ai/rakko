# Harness

This package is the harness of the Rakko repository: the one place that states
which maintenance actions run here. It mounts the actions of the repository,
and the command line that it builds turns each of them into a command. A
bundle carries a set of actions that projects adopt together, so the harness
mounts the bundle instead of each action in it.

The harness also mounts actions that find nothing to do here, such as the
actions for TypeScript and the action for shell scripts. No bundle exports the
action for shell scripts, so the harness mounts it on its own. The only shell
scripts here are the stubs that mise generates in `bin/`, and `.editorconfig`
keeps shfmt away from them. Each action pins the tools that it runs, and
`doctor --fix` writes the pins of the mounted actions into
`.config/mise/conf.d/rakko.toml`. This repository holds the tests of every
action, and those tests run the tools at their pinned versions. So the mounts
make mise install every declared tool that a test needs. The root `mise.toml`
supplies the toolchains. Each mounted action is also a job of CI. So the job
of a TypeScript action installs the tools, and then the action skips.

It also mounts commands, for a maintenance activity that no single action
describes. A command comes from a crate, as an action does, and `src/main.rs`
names it. When a command runs actions, `src/main.rs` names those actions as
well and gives them to the command.

It also mounts groups, such as pre-commit. A group is a name and a list of
actions that the command line runs as one command. `src/main.rs` gives each
group its list. The command line takes the flags of the group from those
actions, and it runs them.

## Usage

Run the harness from any directory of the repository:

```console
mise run rakko
```

Where mise supplies its environment, `rakko` is a shortcut for the same
command.

## The Action of This Package

### `build-site`

The action lives in this package, in `src/build_site/`, because it serves only
this repository. It builds the website in `site/` into `site/public` with Zola.
Zola is a toolchain of the repository: the root `mise.toml` pins it, and the
action accepts 0.23.6 and later releases before 0.24. The build fails on a
broken `@/` link or anchor between two pages. It also fails on an example that
a page shows from a file or a region that does not exist. A failed build gives
one finding with what Zola reported. So the build is the check of the website
as well, and CI runs it on every pull request, as it runs every action.

```console
mise run rakko -- build-site
```

## Commands

### `pre-commit`

The command comes from [`aonyx-rakko-pre-commit`][aonyx-rakko-pre-commit]. It
runs the actions that guard a commit. `src/main.rs` gives them in one list, and
the command runs the actions that write before the actions that read. The
command takes its flags from these actions, so `--fix` reaches the formatters,
which declare it, and no other action. With `--fix`, the formatters write, so
they run first, in the order of the list, and the checks then read what they
wrote. The README of the crate describes what a run does and when it fails.

```console
mise run rakko -- pre-commit --fix
```

The hook that Git runs before a commit passes `--fix`, so that the formatters
rewrite the files that they can format. A run without the flag reports what a
commit has to repair, and it changes nothing.

### `set-msrv`

The command comes from [`aonyx-rakko-set-msrv`][aonyx-rakko-set-msrv]. It sets
the minimum supported Rust version of the repository in three places. These
places must agree: the root `Cargo.toml`, the Rust pin in `mise.toml`, and
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

[aonyx-rakko-pre-commit]: ../../commands/aonyx-rakko-pre-commit/README.md
[aonyx-rakko-set-msrv]: ../../commands/aonyx-rakko-set-msrv/README.md
