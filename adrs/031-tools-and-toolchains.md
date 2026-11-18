# ADR-031: Tools and Toolchains

## Status

Accepted

## Context

[ADR-021] gives each external program that an action runs one of two owners.
A tool, such as taplo, belongs to the action. The crate that wraps the tool
pins its exact version in a mise file that the crate embeds. A toolchain,
such as Rust or Node.js, belongs to the project. The project pins it, and
the action states the range of versions that it accepts. [ADR-028] lets the
pin of a tool list the platforms where the tool has a build.

The contract has one type for both, the tool requirement. An exact version
marks a tool, and a range marks a toolchain. Each requirement also holds the
program that the action starts. The `Action` trait has one method that
returns the requirements of the action. On 2026-11-18, 29 of the 30 actions
in this repository implement that method.

For a tool, the method repeats the mise file. The file already holds the
identifier, the exact version, and the platforms. Code reads these values
when the crate compiles and builds a requirement from them. Only the program
comes from the code, and nothing reads the program before a run. The
generated file of the project and the platform skip use the other three
values. The program matters only when the action asks mise for it. That
lookup already stops the action when mise finds no such program. So the
author of an action learns a builder to write what the file already says.

For a toolchain, 20 of the 29 actions declare a range: 11 for Rust, 8 for
Node.js, and 1 for Terraform. Some of these ranges protect a result.
`test-typescript` needs a release of Node.js that strips types by default.
An older release leaves out every test in TypeScript and says nothing, so
the action passes. Three Rust actions need a nightly toolchain for unstable
options ([ADR-015]).

Other ranges repeat a fact about a tool. For 7 of the 8 actions that declare
Node.js, the range is a copy of the `engines` field of an npm package. These
ranges arrived together with the pins of ADR-021. No failure on an
unsupported release of Node.js led to them. Renovate moves the pin of such a
tool, and nothing moves the copied range with it.

Rakko checks a range in two places. When an action resolves a toolchain, the
run stops if the pin of the project is outside the range (point 6 of
ADR-021). [ADR-022] also makes every range of every mounted action a
condition of the precondition on the tools. A failed precondition stops
every action, group, and written command. So a pin of Node.js outside the
range of `check-renovate-config` also stops `format-toml`, which never
starts Node.js.

ADR-022 stops the run because a warning lets a broken environment pass a
pull request without a sign. A toolchain outside a range cannot pass without
a sign, because the action that needs it stops with an error. Point 3 of
ADR-028 adds that the stop exists for what `doctor --fix` can repair.
`doctor --fix` does not change the pin of a toolchain.

The contract crate is part of the harness API, which takes only additions
after the launch ([ADR-026]).

## Decision

The mise file that a crate embeds declares the tools of its actions. A
toolchain requirement is a separate and optional part of the contract. An
unmet requirement stops only the action that declares it.

1. **The mise file of the crate declares the tools.** Every tool that the
   file pins is a declared tool, with its identifier, its exact version, and
   its platforms. The machinery takes the declarations from the file. The
   author of an action builds no requirement by hand and writes no method
   that lists the tools one by one.

   An action states only where its tools come from: the file of its own
   crate, a helper crate, or both. A helper crate that wraps a tool keeps
   the pin (point 3 of ADR-021). It offers the declaration of that tool to
   the actions that use it. An action that runs no tool states nothing.

   The crate still embeds the file when it compiles. So a published crate
   carries its declarations, and no run reads a file of the crate. An error
   in the file fails the build of the crate, as point 2 of ADR-028 decided
   for a platform.

   The machinery still has every declaration before a run starts. It
   generates the file of the project and checks the precondition. It also
   skips an action outside the platforms of its tool. ADR-021, ADR-022, and
   ADR-028 still decide these three steps.

2. **The program belongs to the run.** A declaration holds no program. The
   action, or its helper, gives the program when it asks for a tool, such as
   `renovate-config-validator` from the tool `npm:renovate`. One pin serves
   every program of a tool.

   The lookup still asks mise for the program of the declared tool at its
   exact version (point 6 of ADR-021). It takes no other version, and it
   does not search the `PATH` of the environment. An action can ask only for
   a tool that it declares. When the tool has no such program, the action
   stops with an error that gives the tool, the version, and the program.

3. **A toolchain requirement is a part of the contract on its own.** The
   contract gets a type for what an action requires of a toolchain of the
   project. A requirement gives the toolchain and what the action accepts,
   such as a range of versions or a nightly of Rust. An exact version or a
   range no longer says whether a program is a tool or a toolchain.

   The `Action` trait gets a method that returns these requirements, with a
   default of none. The declaration depends on neither the arguments nor the
   context of a run, so the machinery can read it before a run. A helper
   crate can offer a requirement that several actions share.

   The project still pins its toolchains (point 1 of ADR-021). A requirement
   adds no pin to the generated file, and the pin of a nightly still has a
   date (ADR-015).

4. **An action declares a requirement only for a need of its own.** That a
   tool runs on a toolchain is not such a need. A requirement protects a
   result of the action, or it gives a diagnosis that the tool does not
   give.

   `test-typescript` keeps its range of Node.js, because an older release
   gives a pass that is wrong. An action that needs unstable options keeps
   its requirement of a nightly. A range that only repeats the `engines`
   field of an npm package goes. The tool then reports a runtime that it
   does not support in its own words. If such a report misleads its reader,
   that failure is the reason to declare the requirement again.

5. **The selection of a toolchain belongs to the run.** An action without a
   requirement still runs on the toolchain that the project pins. The helper
   that starts the tool asks mise for that pin. It runs the tool on that
   toolchain, and not on the one that the environment has first on `PATH`.
   A toolchain that the project does not pin still stops the action (point 6
   of ADR-021).

   A project can pin several toolchains of one kind, as it does for Rust.
   The helper then selects one of them. It can select by a fact that only a
   run knows, such as the `rust-version` of a manifest. No declaration can
   hold such a fact, and the selection needs none.

6. **An unmet requirement stops the action that declares it, and nothing
   else.** When the project pins no toolchain that a requirement accepts,
   the action stops with an error. It does not skip, and it reports no
   finding. The error gives the requirement and the pin of the project, and
   it says which pin to change.

   The rule is the same when the action runs alone, as a member of a group,
   and in a command that a harness writes. The other members of a group
   still run ([ADR-024]). The author of an action writes no check, because
   the machinery compares the pin with the requirement. A run that skips
   for its platform (ADR-028) checks no toolchain.

   A toolchain requirement is no longer a condition of the precondition. So
   it stops no other action, group, or command. `doctor` still reports an
   unmet requirement of a mounted action, so that a person can find it
   before a run. `doctor --fix` still leaves the pin of a toolchain to the
   project.

This supersedes parts of two earlier ADRs:

- Points 1, 2, and 6 of ADR-021 give every toolchain of an action a range.
  An action now states a range only as a requirement under point 4. The
  stop of point 6 for a pin outside a range applies only to such a
  requirement. The rest of ADR-021 stands, and so does its rule that a
  declaration takes neither arguments nor context.
- Point 5 of ADR-022 makes every required toolchain a condition of the
  precondition. That condition goes. The generated file and the pins that
  override a declared tool stay conditions. Point 2 of ADR-022 still stops
  every command when one of them fails.

ADR-021 and ADR-022 stay Accepted, and their status lines refer to this ADR.
ADR-028 does not change. Its platforms are part of the pin, and this
decision reads them from there.

This decision changes the `Action` trait and the tool requirement, which are
items of the harness API. ADR-026 decides how such a change reaches a
harness and the authors of actions outside this repository. The change can
arrive as additions next to deprecated items, or as a breaking 0.x release.

The decision records the direction and stops there. The names of the types
and the method are implementation choices. So is the form in which an action
states where its tools come from, such as a derive or an attribute. Whether
the contract keeps a method for tools that no author writes stays open too.
The machinery can compare a requirement with the pin before it starts the
action, or when the helper resolves the toolchain. How `doctor` shows an
unmet requirement is an implementation choice. Point 4 decides for each
range of today whether it stays, one action at a time.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### One Requirement for Tools and Toolchains

The contract can keep one type and one method for both. Nothing breaks, and
no action migrates. But each author still builds, by hand, values that the
mise file of the crate already holds. The kind of the version also still
carries two facts that it does not state: who owns the pin, and how a run
resolves it.

### The Program in the Declaration

A declaration can keep the program, so that the machinery can check before a
run that the tool has it. But no check reads the program today. The
generated file and the platform skip do not use it, and the lookup at the
run reports a missing program anyway. The author would write a second source
of truth, and only the lookup would read it.

### Toolchain Requirements in the Mise File

The mise file of the crate can hold the ranges of the toolchains next to the
pins of the tools. An action then has one place for everything that it
needs. But an entry of a mise file is a pin: mise installs it, and Renovate
moves it. A range is neither. It needs a syntax of its own that mise does
not read, and Rakko has to keep it out of the generated file. The reason for
a requirement is also a fact about the code of the action, and the code is
where a reader looks for it.

### No Toolchain Requirements

The contract can drop every range and let each tool fail on a toolchain that
it does not support. The contract is then as small as it can be. This works
when the tool reports the problem. It does not work for `test-typescript`,
where an older release of Node.js gives a pass. An action that needs a
nightly would also fail with an error about an unknown option, which does
not say what the project must pin.

### A Requirement Only in the Helper

A helper can hold a range and check it when it resolves the toolchain, with
no method on the trait. The contract then gets no new item, and the check
sits next to the code that needs it. But nothing can read such a requirement
before a run. `doctor` cannot report it, and the documentation of an action
cannot show what the action needs from the project. The project pins its
toolchains, so the project must be able to see what its actions require of
them.

### Toolchains in the Precondition

The precondition can keep the toolchains, as ADR-022 decided. A project then
learns about every unmet requirement before any command runs. But one action
that the project mounts can then stop every other command, and `doctor
--fix` cannot repair the cause. The action that needs the toolchain fails
with an error either way, and a group that holds it fails too. So a broken
toolchain still cannot pass a pull request without a sign.

## Consequences

- The author of an action that wraps a tool pins the tool in the mise file
  and says that the action uses the file. The program appears only where the
  action starts it. A usual action is then its name, its arguments, its
  access, and its run.
- The mise file of a crate becomes a declaration. Every tool in it reaches
  the generated file of each project that mounts an action of the crate. So
  the file must hold only the tools that those actions run.
- Toolchains become a concept of the public API on their own. The
  documentation can describe what a project pins and what an action requires
  apart from the tools that Rakko manages.
- The 29 actions that implement the method migrate, and so do the helper
  crates. For each of the 20 actions with a range, someone has to decide
  under point 4 whether the range stays.
- Without a copied range, a runtime that a tool does not support shows as a
  failure of the tool. That report can be less clear than the error of a
  range. A requirement returns only when such a report misled someone.
- A project with an unmet requirement can still run its other commands. The
  action with the requirement fails, and so does each group that holds it,
  such as pre-commit.
- A project learns about an unmet requirement later than today. It learns
  when the action runs or when someone runs `doctor`, and no longer before
  every command.
- `doctor` now also reports a condition that is not a precondition. A
  finding of `doctor` therefore no longer means that every command stops.
- The change reaches the harness API after the launch. Under ADR-026, it
  keeps deprecated items until the next breaking release, or it is that
  breaking release. A breaking release starts the four weeks before 1.0
  again.
- The glossary entries for the tool requirement, the toolchain, and `doctor`
  change together with the implementation.
- A reader of ADR-021 or ADR-022 must read this ADR as well.

[adr-015]: 015-nightly-toolchain.md
[adr-021]: 021-tool-declarations.md
[adr-022]: 022-preconditions.md
[adr-024]: 024-group-runs.md
[adr-026]: 026-harness-api-stability.md
[adr-028]: 028-tool-platforms.md
