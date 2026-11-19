+++
title = "Tools and Mise"
description = "How actions and projects choose tool versions, and how doctor keeps them consistent."
weight = 30
+++

An action's behavior depends on the tools it runs. Rakko records the
required versions, [mise] installs them, and `doctor` makes sure that the
project's pins match the actions. This keeps contributors, editors, and CI
on the same tool versions.

## Two Kinds of Tools

The project and its actions own different tool versions.

A toolchain is a compiler or runtime for the project's code. Rust, Node.js,
and TypeScript are examples. The project pins these versions in `mise.toml`,
and an action runs on the project's pin. An action can also require a range
of versions, such as a minimum release or a dated Rust nightly. It does this
to protect a result, or to use an option that only a nightly has. Most
actions require nothing. Such an action still runs on the project's pin, and
never on a runtime that it finds on the `PATH`.
[Use a Toolchain From an Action][use-a-toolchain] shows both cases.

For other tools, a pin specifies an exact version. The action owns that pin,
and its tests cover that version. The pin travels with the action, so
projects do not need to copy it by hand.

The pin lives in `mise.rakko.toml` in the action crate or a shared helper
crate. [Mise][mise] reads that file only with the `rakko` environment active.
Renovate can update the pin and test the action against the new tool version
in the same pull request.
[Run a Tool From an Action][run-a-tool] shows how to declare a tool.

## The Generated File

`doctor --fix` collects tool declarations from mounted actions and group
members. It writes `.config/mise/conf.d/rakko.toml` and locks those tools in
`.config/mise/mise.lock`. Commit both files. Use `doctor --fix` to update
them rather than editing them by hand.

[Mise][mise] reads the generated file alongside the project's `mise.toml`.
Contributors, CI, and editors therefore use the versions that the actions
declare. Your `mise.toml` keeps the toolchains and any tools that no action
declares.

Run `mise install` after the pins change. Rakko records requirements but
leaves installation to [mise]. If a required version is not installed, the
action stops with an error that identifies the tool and version.

## One Version for Each Declared Tool

[Mise][mise] lets a project pin override a generated pin. For example, `format-toml`
declares taplo 0.10.0. A project pin of `taplo = "0.9.3"` takes precedence
in [mise] over the generated file.

That override can give editors and manual tool runs a version that the
action's tests do not cover. Rakko rejects it: `doctor` reports the project
pin, and `doctor --fix` removes it.

To keep an older tool version, keep the action release that pins it.

## Tools With Builds for Some Platforms

A platform identifies an operating system and, optionally, an architecture.
If a tool supports only certain platforms, its pin can list them in [mise]
syntax, such as `linux` or `macos/arm64`. Rakko copies the list into the
generated file. [Mise][mise] installs and locks the tool only on those platforms.

On other platforms, Rakko skips the action and names the tool and platform
in the reason. A skip does not fail the run or block other commands. CI on
a supported platform still runs the action.

[Mise][mise] requires trusted configuration for pins with platform lists. Use
`mise trust` to trust the configuration. Until then, `doctor` returns an
error if the generated file contains such a pin.

## The Checks of `doctor`

Every harness has the command `doctor`. It checks three conditions:

- The generated file matches the tools that the mounted actions declare.
- No pin of the project overrides a declared tool.
- The project pins each toolchain that a mounted action requires, at a version
  that the requirement of that action accepts.

The third check reads only the requirements that actions declare. An action
that runs on a toolchain without a requirement does not make `doctor` report
a missing pin. For example, `format-terraform` runs on the Terraform of the
project and requires no release. If the project pins no Terraform, `doctor`
reports nothing, and `format-terraform` stops with an error when it runs.

`doctor` reads project files and asks [mise] about them. It needs no network,
so it can examine the same conditions offline and in CI.

Each problem is a finding, and `doctor` exits with code 1, the code of a run
with findings. `doctor --fix` exits with code 1 too while a problem remains. A toolchain
pin that an action does not accept is such a problem, because `doctor --fix`
does not change it. So a workflow that runs `doctor --fix` fails until you
change the pin.

`doctor --fix` rewrites the generated file, removes overriding pins, and runs
`mise lock`. Locking needs the network and covers all project tools, so it
can also change the lock file beside your `mise.toml`. Toolchain versions
remain your project's choice. If a toolchain pin needs to change, the report
identifies it.

If no action declares a tool, `doctor --fix` removes any leftover generated
file.

## Outdated Tool Pins

Rakko requires the first two conditions before it runs an action, group, or
custom command. If one of them fails, the run stops, including in CI, and
explains the repair. Help, the command list, and `doctor` remain available.

A toolchain requirement is not one of these conditions. If the project pins
no version that an action accepts, only that action stops, with an error that
identifies the pin to change. Other actions and custom commands run. A group
that holds the action runs its other members and fails.

Stopping prevents CI from accepting an inconsistent environment. Repair
requires an explicit `doctor --fix` run because it changes committed files
and needs the network.

An action update can change a tool pin and leave the generated file out of
date. [Keep a Harness Current][keep-current] shows how to run `doctor --fix`
in the update pull request so that CI can test the complete change.

`doctor` sees mounted actions and group members. If an action is passed
only to a custom command, also mount it so that `doctor --fix` includes its
tools.

[keep-current]: @/how-to/keep-a-harness-current.md
[mise]: https://mise.jdx.dev
[run-a-tool]: @/how-to/run-a-tool.md
[use-a-toolchain]: @/how-to/use-a-toolchain.md
