+++
title = "The Harness and Commands"
description = "Why the harness is a separate binary and when to add a custom command."
weight = 20
+++

Your harness is a small binary crate that defines your project's maintenance
command line. Its code selects the actions, groups, and custom commands that
the project runs. Keeping it separate from the project also isolates its
dependencies and Rust version requirements.

## The Role of the Harness

Actions are libraries. The harness calls them through a command line built
by [`rakko-cli`][rakko-cli]. Its `main` function mounts each entry, which
adds it to the command line. Reading `main.rs` shows what the project
selects. Rakko also supplies help and `doctor` in every harness.

A bundle exports a list of actions. The harness can mount the full list,
remove an action, or combine lists. These choices stay in code, where a
reviewer can see them in a pull request.

The harness's `Cargo.lock` records exact action versions. An update bot can
propose new versions in pull requests, so maintenance updates follow the same
review and CI process as other dependencies. [Keep a Harness
Current][keep-current] shows how to set this up.

## A Package Outside the Workspace

The harness has its own Cargo workspace and lock file. In a shared workspace,
Cargo resolves dependencies for all members together. A shared dependency
can then resolve to a version that satisfies both the project and its
maintenance tools.

For example, both the project and an action can depend on `serde`. If the
action needs a newer release, Cargo can select it for the project too. That
release can require a Rust version above the project's declared minimum.
The maintenance dependency can then raise the Rust version needed to use the
library.

Separate dependency resolution prevents this coupling. The project keeps
its dependency versions and minimum supported Rust version.

This has a few costs:

- The harness uses a separate target directory and needs its own build.
- rust-analyzer needs the harness listed as a linked project.
- The update bot must include the harness's `Cargo.lock`.

## The Mise Task

A [mise] task builds and runs the harness. It locates the harness manifest
relative to the project root, so `mise run rakko` works from any project
directory.

The task works on every platform that [mise] supports, including Windows.
Platform-specific shortcuts can be shorter. Rakko documents `mise run rakko`
so that every reader can use the same command.

The task builds before each run. Changes to harness code, actions, or pinned
versions therefore take effect on the next invocation.

Actions also find their external tools through [mise].
[Tools and Mise][tools-and-mise] explains how this works.

## The Command of an Action

Each mounted action gets a command with the same name. Its help comes from
the first paragraph of the action's doc comment. Its flags come from the
action's argument declarations. Action commands also accept shared flags,
such as `--json`, `--quiet`, and `--project-root`.

The exit code of a run tells CI what the action found:

- 0 when the action passed, repaired everything that it found, or skipped
- 1 when the action found problems
- 2 when the run has no answer, for example because the action stopped with
  an error

Preconditions are environment requirements for a correct run. Rakko makes
sure that they hold before running an action, group, or custom command. For
example, tool pins must match the mounted actions. A failed precondition
stops the run, including in CI, and the message explains the repair.

## Custom Commands

An action runs once and returns an outcome. Custom commands can handle
different kinds of work:

- A development server that runs until you stop it.
- A watcher that repeats a check when files change.
- A cleanup or setup step that does not report findings.

A custom command is asynchronous code that returns success or an error. It
receives the project context, including the root directory, and a context
from [Clawless][clawless], the command-line framework. The command uses
[Clawless][clawless] to write its output and controls its own behavior.

Custom commands share the harness's naming rules, argument syntax, and
environment requirements. Their help appears alongside actions. Duplicate
names stop the harness at startup, and `doctor` and `help` are reserved.

A command cannot look up mounted actions. The harness must pass in any
actions the command needs. The command runs them as libraries and handles
their output and results. Its exit code reports success or an error, while
an action's exit code also distinguishes findings.

A command can live in the harness or in a dependency crate.
[Add a Command][add-a-command] shows how to write one.

### A Command or a Group

Use a group when you want to run several actions together. Rakko derives
the flags, schedules the members, and reports their results with one summary
and exit code. A custom command must implement that behavior itself. It
fits work that needs its own arguments or execution order.

[Actions and Groups][actions-and-groups] explains groups, and [Form a
Group][form-a-group] shows how to form one.

[actions-and-groups]: @/explanation/actions-and-groups.md
[add-a-command]: @/how-to/add-a-command.md
[clawless]: https://clawless.rs
[form-a-group]: @/how-to/form-a-group.md
[keep-current]: @/how-to/keep-a-harness-current.md
[mise]: https://mise.jdx.dev
[rakko-cli]: https://docs.rs/rakko-cli
[tools-and-mise]: @/explanation/tools-and-mise.md
