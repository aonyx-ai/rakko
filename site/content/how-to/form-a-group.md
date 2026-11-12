+++
title = "Form a Group"
description = "Combine several actions into one command, such as pre-commit."
weight = 40
+++

Run several actions with one command. A group names a list of actions, its
members, and reports one summary and one exit code.

This guide builds `check-project`, a group of checks for a team's projects.
You need a working harness and actions to include. If you need a harness,
start with [Get Started][get-started]. To write an action, follow
[Build an Action][build-an-action].

## Write the Group

A group implements `Group` and supplies its name and members. Rakko runs
the members for you. Write the group in your harness or in a separate crate
that depends on `rakko`:

{{ <example crate="check-project" file="src/lib.rs" region="group" /> }}

The code has these parts:

- The `Documented` derive uses the first paragraph of the doc comment as
  help text. Write one sentence without Markdown or links.
- The group name is also its command name. It follows the action naming rules
  and must be unique among all mounted actions, groups, and commands.
- `members` returns a `Bundle` of actions. Return the same list each time,
  because Rakko reads it both at mount time and before each run. Each member
  must have a unique name within the group.

The constructor accepts the members so that each harness can choose its own
actions without changing the group type.

## Mount the Group

Form the group in the `main` of your harness from a list of actions, and mount
it:

{{ <example
  crate="check-project"
  file="examples/check_project_harness.rs"
  region="main"
  name="src/main.rs" /> }}

`Bundle::default().with(...)` adds each action to the list as it is. The
command line now has the command `check-project`:

```sh
mise run rakko -- check-project
```

Members do not need separate mounts. To expose a member as its own command,
mount it as well, as this example does for `check-license`.
`mise run rakko -- doctor --fix` includes the tools of every group member.

## Pass Flags to the Members

A group gets its flags from its members. Each flag reaches only the members
that declare that argument. The members of `check-project` declare no
arguments, so they add no flags.

A group with formatters, such as `pre-commit`, can expose their `--fix` flag.
Other members run without that argument:

```sh
mise run rakko -- pre-commit --fix
```

The help of each flag shows one line for each member that declares the
argument, with the name of the member and its help line.

If two members declare the same argument, they must use the same shape.
Otherwise, the harness stops at mount time and identifies both members and
their conflicting declarations.

## The Order of a Run

Rakko reads each member's access declaration first. It runs writers one at a
time, in list order, then runs all readers together. Readers therefore see
the project after all writers finish, regardless of their positions in the
list.

A member that fails or returns an error does not stop the others. The group
reports every member's result and exits with the code of the worst outcome.

The group stops before any member runs if argument conversion fails or a
member declares an unknown kind of access. For unknown access, the error
asks you to update Rakko.

[Actions and Groups][actions-and-groups] explains the reasons for this order.

## A Group or a Command

A group supplies the members, and Rakko controls their execution. If you need
custom arguments or a different execution order, write a command instead.
[Add a Command][add-a-command] shows how.

[actions-and-groups]: @/explanation/actions-and-groups.md
[add-a-command]: @/how-to/add-a-command.md
[build-an-action]: @/how-to/build-an-action.md
[get-started]: @/tutorial/get-started.md
