# ADR-019: Group Arguments

## Status

Accepted

## Context

Some commands run a list of actions. Pre-commit runs the actions that guard
a commit, and the tag commands that Rakko expects later will run the actions
that carry a tag. A user who passes a flag to such a command wants the flag
to reach the actions in the list. The formatters in the list of pre-commit
take a boolean `fix`, and `pre-commit --fix` has to let each of them repair
what it finds.

Today pre-commit defines that flag itself. It declares a `fix` argument of
its own, with a sentence of its own, and it builds the values of its actions
by hand. A run with `fix` passes the value to every action in the list, and
an action that takes no `fix` ignores it. This is a deliberate interim, until
the arguments of such a command have a design. A new argument of an action
does not reach pre-commit until somebody changes the command.

[ADR-014] made a command the place for an activity of a project that does not
fit an action, and pre-commit was its first case. Point 3 of ADR-014 says
that a command knows nothing about actions. The projection hands a command
no view of the actions that the harness mounted, and no primitive to run
them. Point 3 calls the primitives that run a list of actions "a decision of
their own". This ADR makes the part of that decision that concerns
arguments.

Two other decisions frame the answer. [ADR-007] gives every naming and syntax
decision to the projection. So `fix` is a boolean in the description of an
action, and `--fix` is a choice of the projection. The rules that the
projection enforces at mount, such as the stop on a name that collides,
already live there. [ADR-017] gives every run three steps: the arguments are
bound, the bound action declares its access, and the same bound action runs.
So a command can bind all its actions before the first one runs.

This is the second step of a longer direction. An action declares
interfaces, such as its arguments, its tags, and its access, and the
machinery derives features from them. Group commands, argument merging,
scheduling, and the placement of a run are such features. Access came first,
in ADR-017 and [ADR-018], and arguments come second. The tag commands are the
next commands that run a list of actions, but this ADR decides nothing about
tags.

## Decision

A group is a command that gives the projection the actions that it runs. The
projection derives the flags of the group from the arguments of these
actions.

1. **A group gives the projection its members.** The members of a group are
   the actions that it runs. The group gives their list to the projection,
   so the projection knows the members when the harness mounts the group.
   The group keeps the decision how it runs its members and what it reports,
   within the guarantees of points 4 and 7.

2. **The flags of a group are the union of the arguments of its members.**
   The projection derives them at mount, from the descriptions of ADR-007.
   When a member declares a boolean `fix`, the group gets a flag for `fix`.
   The projection gives that flag its syntax, as it does for an action.
   Nobody defines a flag on a group by hand.

3. **One name with two shapes stops the harness at mount.** When two members
   declare an argument with the same name and different shapes, the harness
   stops where it mounts the group. It stops as it does for a name that
   collides or for a reserved argument. The message gives both members, the
   argument, and both shapes. Support for this case waits until a real need
   comes up.

4. **Each member receives only the values of its own arguments.** A flag
   reaches every member that declares the argument, and no other member.
   Every action already ignores the values that it does not know. The
   routing makes "no other member" a property of the machinery, and not of
   each action.

5. **The help of a merged flag shows the sentence of each member.** For each
   flag, the help shows one line for each member that declares the argument,
   in the order of the members. Each line starts with the name of the
   member, such as `format-json: Rewrite the files that prettier can format`.
   The names are necessary, because three members share the sentence of
   prettier, and without them the help shows one line three times. This rule
   stays until a design for documentation across actions replaces it.

6. **A group has no arguments of its own.** Beside the shared flags that
   every command carries, its flags come from its members only, until a group
   needs an argument of its own. A command with arguments of its own, such as
   the command that sets the MSRV of a project, stays a plain command. It
   calls its actions as libraries, as ADR-014 describes.

7. **A member that cannot bind stops the group before any member runs.** The
   machinery, and not each group, binds every member to the values of the run
   before the run of the group starts. When a member cannot bind, the group
   does not run, and the error gives the member and the argument. A failed
   bind is an error in the input, not a finding of an action, so no group
   runs half of its members. The projection cannot know before the bind
   whether a member binds. Each action checks its own values when they
   are bound, such as a missing value or a value that does not convert. The
   vocabulary has no `required` argument, and `required` stays out of it
   until an action needs it.

This decision partly supersedes point 3 of ADR-014. Point 3 says that a
command knows nothing about actions. A group is a command that does: it gives
its members to the projection, and the projection derives their flags and
routes their values. ADR-014 stays Accepted, and its status refers to this
ADR.

The rest of point 3 stands. A command that is not a group still knows nothing
about actions. The projection still hands no command a view of the actions
that the harness mounted. It does not run the members of a group either, so
primitives that run a list of actions stay a decision of their own. Point 2
of ADR-014 stands as well, because the stop on a failed bind happens before
the run of the group starts.

The decision relies on ADR-007. No group and no member chooses the syntax of
a flag. The projection decides it for a merged flag as it does for the flag
of one action. The stop on two shapes joins the rules that the projection
enforces at mount, which reach a command under point 5 of ADR-014. The stop
on a failed bind relies on the bind step of ADR-017.

The decision records the direction and stops there. The implementation
chooses how a group gives its members to the projection and what the run of a
group receives. It also chooses the types and the names of the methods. The
decision says nothing about tags, about documentation across actions, or
about a sentence that a group writes for a merged flag. It also says nothing
about the derive macro of ADR-007 or about growth of the vocabulary, such as
`required` and defaults. Each of them gets work of its own.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### A Helper That Each Command Calls

Rakko can offer a function that a command calls itself. The function takes a
list of actions, merges their arguments into flags, and routes the values of
a run. The projection then stays as it is, point 3 of ADR-014 holds as
written, and each command keeps full control of its list.

The tag commands that the longer direction expects will ask much more of the
projection than one flag. A helper solves the need of pre-commit now, and the
design then grows around the helper instead of around the need of the tag
commands. ADR-007 already gives every naming and syntax decision to the
projection, and the rules that run at mount already live there. A helper puts
part of these decisions into the code of each command.

### Predefined Arguments

The contract crate can define arguments that many actions share, such as one
`fix` with one sentence. Each action that repairs then takes this argument.
A merged flag has one meaning and one sentence by construction, and the help
needs no line for each member.

A predefined argument moves the description of an argument, its shape and
its sentence, out of the action and into the contract crate. In this design,
the arguments of a group come from its members only, and each member
describes its own arguments. That is the intent of the design. The main
benefit of a predefined argument is one sentence for a flag that many members
share. How documentation works across actions is a problem of its own, and it
is no reason to break from the design.

### A Flag for Each Member

When two members declare one name with two shapes, the projection can give
each of them a flag of its own, such as `--format-rust-jobs`. The harness
then mounts, and each member gets the shape that it declares.

In this design, one flag stands for one argument, and it reaches every member
that declares that argument. A flag for each member breaks this rule. One
argument then gets several flags, and the flag of a member depends on the
other members of the group. No group needs two shapes yet, so support for
them waits until a real need comes up.

### The First Member Wins

The projection can give a flag the shape of the first member that declares
it. The harness then mounts, and the help shows one shape.

This hides a defect. The other member declares a shape that the flag does not
have, and nothing shows the conflict at mount. The defect appears only in a
run that gives that member a value that it cannot bind.

## Consequences

- A harness defines no flag on a group. `pre-commit --fix` reaches every
  member that takes `fix` and no other member, and the argument that
  pre-commit defines by hand today goes away.
- A new argument of a member changes the flags and the help of every group
  that contains the member. The group itself does not change.
- The members of a group share one set of argument names. Two actions that
  declare one name with two shapes cannot be in the same group, even when
  their authors wrote them apart. Today every argument of an action in the
  fleet is a boolean `fix`, so no harness stops yet. The first conflict waits
  for a design, or for a new name in one of the two actions.
- A stop at mount stops the whole harness, and not only the group, so no
  command of the project runs until the conflict goes away. Each action has
  releases of its own, so a conflict can arrive with the update of one
  action. The project then removes one of the two actions from the group, or
  waits for a release that gives the argument a new name.
- The help of a group grows with its members. In this repository, the help
  of `--fix` on pre-commit shows five lines, and three of them repeat the
  sentence of prettier. In aonyx.ai, it shows eight lines, and six of them
  repeat that sentence. Only the names of the members tell these lines apart.
- A value that one member cannot bind stops the whole group. Today
  pre-commit reports such a value as an error of that action and runs the
  other actions. After this decision, no member runs, so no member writes the
  project in a run whose input was wrong.
- The projection grows. It knows the members of each group and the arguments
  of each member, and the tag commands will ask more of it. Point 3 of
  ADR-014 is now true only of a command that is not a group. A reader has to
  read the two ADRs together.
- A group cannot carry an argument of its own. A command that needs one stays
  a plain command, and it gets no merged flags from the actions that it calls.
- The vocabulary still has no `required` argument. A member that needs a
  value fails when its values are bound at the start of a run, and not at
  mount.

[adr-007]: 007-argument-vocabulary.md
[adr-014]: 014-commands.md
[adr-017]: 017-access-declarations.md
[adr-018]: 018-file-access.md
