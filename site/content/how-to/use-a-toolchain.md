+++
title = "Use a Toolchain From an Action"
description = "Run your action on the compiler or runtime that the project pins. Require a release of it only when an older release gives a wrong result."
weight = 25
+++

Run your action on a toolchain of the project, such as Node.js or Rust. The
project pins its toolchains, and your action runs on that pin. Your action can
also require some releases of a toolchain, when another release gives a
wrong result.

This guide builds `test-node`, an action that runs the test runner of Node.js
on a project. You need to know how to [build an action][build-an-action]. The
action uses [`rakko-tool`][rakko-tool], as in [Run a Tool From an
Action][run-a-tool].

## The Pin of a Toolchain

A toolchain is a compiler or runtime that the code of the project builds or
runs with. The project pins it in its own `mise.toml`:

```toml,name=mise.toml
[tools]
node = "24.11.0"
```

Your action does not pin a toolchain, and `doctor --fix` never writes or
changes the pin of a toolchain. A project that updates Node.js changes its
own pin, and your action then runs on the new release.

A tool is different. Your action pins each tool that it wraps, such as taplo,
in its `mise.rakko.toml`, and its tests cover that version. [Tools and
Mise][tools-and-mise] explains why the two have different owners.

## Resolve the Toolchain

Resolve the toolchain with `Tool::resolve_toolchain`. Give it the program to
start:

{{ <example crate="test-node" file="src/lib.rs" region="run" /> }}

The lookup asks [mise] for the program at the version that the project pins.
It never searches the `PATH` of the environment. So your action runs on the
same release as the editors, the terminals, and the CI of the project. A
project that pins no Node.js stops the action with an error that says so.

Without a requirement, the lookup takes the first pin of the toolchain. With
one, it takes the first pin that the requirement accepts. A project can pin several
releases of one toolchain, as it does for a stable and a nightly release of
Rust. An action that needs another pin selects it in the run, such as the
release that the `rust-version` of a manifest gives. `accepting` adds such a
range for one lookup.

Some tools start their toolchain by name, such as a package for Node.js that
starts `node`. Resolve the tool and the toolchain. Then give the toolchain to
the tool with `Tool::with_toolchain`. The tool then runs on the toolchain of
the project, and not on the one that comes first on the `PATH`.

## Require a Release

Most actions require nothing of a toolchain, and they run on whatever release
the project pins. An action without a requirement does not run on any runtime
that it finds. It still runs on the pin of the project.

Declare a requirement only for a need of the action itself:

- An older release gives a wrong result, such as a pass without a test.
- The error of the toolchain does not say that its pin is the cause, such as
  an unknown option that only a Rust nightly has.

That a tool runs on a toolchain is no such need. A package for Node.js can
state the releases of Node.js that it supports. Do not copy them into a
requirement, because the tool reports an unsupported release with its own
error.

`test-node` has such a need. Node.js runs test files in TypeScript only from
22.18 and 23.6 on. An older release skips these files, finds no test, and
passes. So the action returns a `ToolchainRequirement` from `toolchains`:

{{ <example crate="test-node" file="src/lib.rs" region="toolchains" /> }}

The requirement holds the toolchain and a `VersionRange`. A range uses the
syntax of a version requirement of Cargo, with two additions that Cargo does
not have. A range joins alternatives with `||`, and the range `nightly-*`
accepts every dated Rust nightly. A requirement adds no pin, and
the project still chooses the release.

You write no check of your own. `Tool::resolve_toolchain` reads the
requirements of the action and takes the first pin that they all accept.

## An Unmet Requirement

If the project pins no release that the requirement accepts, the lookup
returns an error. The action stops with that error, and it does not skip. The
error gives the requirement and the pin, and it says which pin to change.

Only the action with the requirement stops. Other actions and the custom
commands of the harness still run. A group that holds the action runs its
other members and fails. A run that skips because a tool has no build for the
platform checks no toolchain.

`doctor` reports an unmet requirement of each mounted action, so that you
find it before a run. It reports only the requirements that actions declare.
An action that runs on a toolchain without a requirement, such as `lint-rust`
on Rust, does not make `doctor` report a missing pin. That action stops with
an error when it runs.

## Test the Requirement

A test can read the requirement without a toolchain:

{{ <example crate="test-node" file="tests/test_node.rs" region="toolchains" /> }}

A test that runs the action needs [mise], because the lookup asks it about
the pins of the project. A project that pins an older release shows that the
action stops. Mise does not have to install the release, because the lookup
compares the pin before it asks for the program:

{{ <example crate="test-node" file="tests/test_node.rs" region="unmet" /> }}

`Project` is a helper of the test file. It writes the `mise.toml` of a
temporary project and trusts it with `mise trust`, because [mise] ignores a
configuration that nobody trusts.

## Next Steps

[Test an Action][test-an-action] covers tests that run tools.
[Migrate From 0.1][migrate] shows how an action of 0.1 changes to declare
its toolchains in this way.

[build-an-action]: @/how-to/build-an-action.md
[migrate]: @/how-to/migrate-from-0-1.md
[mise]: https://mise.jdx.dev
[rakko-tool]: https://docs.rs/rakko-tool
[run-a-tool]: @/how-to/run-a-tool.md
[test-an-action]: @/how-to/test-an-action.md
[tools-and-mise]: @/explanation/tools-and-mise.md
