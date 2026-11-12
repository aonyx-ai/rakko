+++
title = "Actions and Groups"
description = "How actions, groups, and bundles organize maintenance work and report results."
weight = 10
+++

An action does one maintenance task. A group runs several actions under one
command. A bundle supplies a set of actions for a project to adopt.

The harness is your project's maintenance binary. It mounts the actions and
groups you choose, which adds them to its command line. These concepts
separate what work a project adopts from how it runs that work.

## Actions

An action is a library crate that implements `Action` from `rakko`, the
contract crate. The contract defines the interfaces that actions and
harnesses share. An action can format TOML, lint Markdown, or inspect
dependency licenses. It depends on the contract and any helpers it needs,
rather than on a command-line framework.

An action declares its name, help text, arguments, and tools as data. The
command line can read these declarations without running the action.
Arguments describe their types, such as a boolean `fix`. The command line
turns that declaration into `--fix`, so actions share the same flag syntax.

An action that only examines a project is often called a check. Rakko uses
the same `Action` trait for checks and actions that repair problems.

### Access

Access declares whether a run reads or writes the project. It can depend on
arguments, so `--fix` can turn a read into a write. The declaration does no
I/O because Rakko needs it before any action runs. Writers receive exclusive
access to the whole project. Readers can share it.

The default is write access, which keeps the run separate from other
actions. An action that only reads can declare read access to run alongside
other readers.

The command line trusts this declaration. In tests, use
`run_checking_access` from `rakko-test-utils` to detect project file changes
during a read-only run.

### Tools of an Action

An action pins external tools, such as linters, to exact versions. [Mise][mise]
installs those versions, so projects use the tools covered by the action's
tests. The project pins its toolchains, such as Rust or Node.js, and the
action declares an accepted version range. [Tools and Mise][tools-and-mise]
explains who owns each pin.

## The Outcome of a Run

An outcome is the result of one action run. It reports whether the action:

- Passed.
- Changed the project and repaired every problem it found.
- Failed with findings.
- Skipped.
- Stopped with an error.

A finding describes one problem and its location. The location can identify
the project, a directory, a file, or a position or range within a file.
A repair describes a problem that the action fixed. It has the same
structure as a finding. If some problems remain, the action reports failure
with both the remaining findings and the completed repairs.

An action skips when it has nothing to examine or a required tool has no
build for the current platform. A skip makes the missing examination visible
without failing the run.

Errors are outcomes too, so the command line handles every action result
through the same interface.

### Output and Exit Codes

The action returns data, and the command line formats the output. At a
terminal, it shows findings and a summary such as `check-readme: 1 finding`.
With `--json`, it writes a document for programs to read. With `--quiet`, it
omits informational messages. This separation lets Rakko add output formats
without changing actions.

The exit code follows the outcome:

| Outcome                     | Exit code |
| --------------------------- | --------- |
| Passed, changed, or skipped | 0         |
| Failed with findings        | 1         |
| Stopped with an error       | 2         |

A run that repairs every problem exits with 0. An error takes precedence
over findings because the action cannot complete its examination.

## Groups

A group names a list of actions, its members. For example, `pre-commit` can
run the actions that examine a change before a commit. The group supplies
the list, and Rakko decides how to run it.

A group's flags come from its members. Each flag reaches only the members
that declare the corresponding argument. For example, `pre-commit --fix`
passes `fix` to members that support it and leaves other members' arguments
unchanged.

### The Scheduler

The scheduler controls when group members run. It reads every member's
access declaration, runs writers one at a time in list order, then starts
all readers together. A linter therefore sees the files after a formatter
changes them, regardless of their positions in the list.

Every member runs to completion, even if another member fails or returns an
error. One group run therefore reports all the results instead of stopping
at the first problem.

### The Result of a Group

The group reports one summary and exits with the code of its worst member
outcome. Errors take precedence over findings. With `--json`, it writes one
document for the whole group. Scripts and CI can therefore handle a group
like a single action.

## Bundles

A bundle is a crate that exports a list of actions. It lets a harness adopt
a set with one mount. Bundles can include other bundles, and the harness can
filter or combine their lists. A bundle controls which actions the harness
adopts. A group controls which actions run together under one command.

## Commands

Rakko creates a command for each mounted action and group. For work that
needs different behavior, such as a development server, the harness can
also mount custom commands. [The Harness and Commands][harness-and-commands]
explains how they fit.

[harness-and-commands]: @/explanation/harness-and-commands.md
[mise]: https://mise.jdx.dev
[tools-and-mise]: @/explanation/tools-and-mise.md
