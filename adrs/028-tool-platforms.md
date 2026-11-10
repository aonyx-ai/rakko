# ADR-028: Tool Platforms

## Status

Accepted

## Context

[ADR-021] makes an action pin the exact version of each tool that it runs.
Rakko writes these pins into a mise file that the project commits, and mise
installs each tool from that file. A toolchain, such as Rust, is different.
The project pins it, and the action declares only the range of versions that
it accepts. [ADR-022] adds a precondition, which the projection checks before
it runs anything. The generated file must match the declarations of the
mounted actions, and the project must pin each toolchain in its range. The
command `doctor` reports the precondition, and `doctor --fix` repairs it.

A platform is an operating system, or an operating system with an
architecture, such as Linux, or Windows on x64. Some tools have no build for
some platforms. Stiles has no build for Windows or for macOS on Intel.
Zizmor has no build for Windows on ARM. A pin cannot say this.

On a platform without a build, mise cannot install the tool. `mise lock`
gives a warning for that platform, so the action `check-mise-lock` stops
with an error. The action that runs the tool stops, because the tool is absent
(point 2 of [ADR-011] and point 6 of ADR-021). The project cannot repair a
missing build. `doctor --fix` writes the same pin again, and mise still has
nothing to install.

This repository has a workaround for stiles. The action `lint-stiles`
declares stiles as a range, as for a toolchain. So the root `mise.toml` can
pin stiles with a platform limit, and the generated file does not hold it.
On Windows, mise then reports no version of stiles. So `doctor` reports that
the project pins no version in the range, and the precondition stops every
command of the project. Thus on Windows, the harness does not mount
`lint-stiles`.

So a contributor on Windows loses either one linter or every command. Each
project that mounts the action must also find this workaround again.

Mise can limit a tool to some platforms with its `os` option. The value is a
list of platforms, such as `["linux", "macos/arm64"]`. Mise then installs and
locks the tool only on those platforms. We checked mise 2026.10.21 on
2026-11-10 on an Apple Silicon Mac:

- The option works in a file in `.config/mise/conf.d`, as it does in
  `mise.toml`.
- `macos/arm64` and `macos/aarch64` both match that Mac, and `macos/x64` does
  not.
- With the option set, `mise lock` locks only the listed platforms, and it
  gives no warning about the others.
- On a platform outside the list, mise reports no current version of the
  tool.
- A value that mise does not know, such as `macos-arm64`, matches no
  platform, and mise gives no warning.

## Decision

An action can declare the platforms where its tool exists. On every other
platform, `doctor` and the precondition do not require the tool. The
machinery then skips the action with a reason that the user sees.

1. **The author of the action declares the platforms.** The declaration is
   part of the pin of the tool, next to its exact version. The author pins
   the tool and sees its release files, so every project gets the same limit
   and copies nothing. A tool without a list of platforms exists on every
   platform, as before. A toolchain gets no list, because the project pins
   its toolchains ([ADR-021], point 1).

2. **A platform uses the form of mise.** A platform is an operating system
   with an optional architecture, in the form of the `os` option of mise,
   such as `linux`, `macos/arm64`, or `windows/x64`. Rakko takes the
   platform as a value of its own type. So a misspelled platform, such as
   `macos-arm64`, fails the build of the action. The error never reaches
   mise, which accepts such a value and gives no warning.

   Rakko writes the list into the generated file without a change. So mise
   installs and locks the tool only on its platforms, and the lockfile has
   no entry for the others.

3. **Outside its platforms, `doctor` and the precondition do not require
   the tool.** No part of the precondition requires that mise installs or
   resolves such a tool on that platform. So `doctor` gives no finding for
   the absent tool, and the precondition does not stop the run because of
   it. The stop exists for pins that `doctor --fix` can repair, and nothing
   can repair a missing build.

   The generated file is the same on every platform. So on every platform,
   the precondition still compares the pin and its list of platforms with
   the declarations. There, `doctor --fix` still repairs a stale pin.

4. **The machinery skips the action.** On a platform outside the list, the
   machinery does not start the action. The run ends as skipped, and the
   reason gives the tool and the platform. A skip does not fail the run, but
   it shows that the check did not run. The machinery decides from the
   declaration, so no action changes its code.

   The skip applies to a run of the action alone, as a member of a group,
   and in a command that a harness writes. A command that a harness writes
   calls its actions as libraries (point 3 of [ADR-014]). So the skip must
   be on the path that such a call takes.

This supersedes two parts of earlier ADRs, each only for a tool outside its
declared platforms:

- In point 2 of ADR-011, a tool that is absent stops the action. Such a tool
  skips the action instead. For every other tool, point 2 stands as ADR-021
  left it.
- Point 6 of ADR-021 stops a run when mise did not install the declared
  version. Mise never installs such a tool, and the run skips instead. For
  every other tool, point 6 stands.

ADR-011 and ADR-021 stay Accepted, and their status lines refer to this ADR.

ADR-022 needs no change. Its point 2 stops every command when a condition
of the precondition fails, and its point 5 gives the conditions for the
tools. No condition today depends on an install of a tool with an exact
version. The generated file, the pins that override a declared tool, and the
toolchains get the same checks on every platform. Point 3 above keeps this
true for a later condition: no condition requires a tool outside its
platforms.

[ADR-013] still holds. Its point 1 says that an action examines nothing
before it runs its tool. Here the machinery skips the action from the
declaration, before the action starts, and the action examines nothing. The
skip examines no part of the project. It answers only whether the tool
exists on the platform of the run, from a fact about the releases of the
tool. So the skip is not a judgment of whether the action applies to the
project, which ADR-013 calls applicability.

Point 2 of ADR-013 still decides the skip of a tool that ran and had nothing
to examine. Point 3 of ADR-013 still keeps applicability out of the trait.

ADR-013 also leaves open whether an action skips in a project that does not
provision its tool. An example is a project that mounts `lint-yaml` and does
not let mise install yamllint. That question stays open. A missing build is
not a choice of the project, so this decision does not answer it.

The decision records the direction and stops there. The names of the types
and the format of the platforms in the pin are implementation choices. How
the skip learns the platform of the run is an implementation choice too. The
skip can compare the platform of the run with the list, or it can ask mise.
The decision covers only the `os` option of mise. Other options of mise on a
pin, such as `version_prefix`, wait for a case that needs them. The C library
of a Linux build, such as gnu or musl, is outside this decision, because the
`os` option cannot express it.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### A Failure Outside the Platforms

The precondition can let every command run, while the action fails on a
platform without a build. The failure is then visible, and nobody can miss
that the check did not run. But the project cannot repair a missing build,
so the failure stays on every run on that platform. People then learn to
ignore it, and they also miss the failures that they can repair. A skip
shows that the check did not run, and it does not fail the run.

### Platforms in the Configuration of the Project

Each project can limit the tool in its own configuration, as this repository
does today for stiles. But every project that mounts the action then copies
the same limit, and it must learn which platforms have a build. The author
of the action already knows, because the author pins the release. [ADR-021]
moved the pins from the project to the action for the same reason.

### A Platform Check in Each Action

Each action can check the platform when it runs, and skip on its own. But
then every action that wraps such a tool repeats the same code. `doctor`
also needs the list in the declaration, because `doctor --fix` writes the
list into the generated file, and the precondition compares the file with
the declarations. The machinery reads the declaration once for every
harness, and the action examines nothing, as ADR-013 says.

### Target Triples

A platform can use the form of a Rust target triple, such as
`aarch64-apple-darwin`. Rust developers know this form. But mise does not
read it, so Rakko must translate each triple into the form of mise. A triple
that mise cannot express has no translation. The form of mise needs no
translation, and the generated file holds the same text as the pin.

### Every Option of Mise

The pin can carry any option of mise, such as `version_prefix` or the
`crate` option of the cargo backend. This covers more tools. But only the
platform limit has a case today. [ADR-027] removed `check-specs`, the only
action that needed `crate`. The declaration can grow when a case needs
another option.

## Consequences

- A project on any platform can mount an action whose tool has no build
  there. Its other commands still run, and the run of that action skips
  with a reason.
- On a platform outside the list, the check of the action does not run. A
  contributor on that platform depends on CI or on another contributor for
  that check.
- The author of an action must keep the list current. A new build of a tool
  for a new platform reaches projects only when the author adds the platform
  to the list. An update of the tool can also remove a build, and the author
  must then remove that platform from the list.
- The lockfile of the generated file holds no entry for a platform outside
  the list, so `check-mise-lock` gives no warning for those platforms.
- This repository can mount `lint-stiles` on every platform. The generated
  file can then hold the pin of stiles, which the root `mise.toml` holds
  today.
- The machinery and the contract grow by a list of platforms on each pin of
  a tool. The list is an addition with a default of every platform, so an
  action that declares no list does not change.
- A platform that is not in the form of mise, such as `macos-arm64`, fails
  the build of the action. A valid platform that is wrong still passes, such
  as `macos` for a tool without a build for macOS on Intel. Only the author
  of the action can find that error.
- A reader of ADR-011 or ADR-021 must read this ADR as well, for a tool
  outside its platforms.

[adr-011]: 011-tool-integration.md
[adr-013]: 013-applicability.md
[adr-014]: 014-commands.md
[adr-021]: 021-tool-declarations.md
[adr-022]: 022-preconditions.md
[adr-027]: 027-specification-check.md
