# ADR-024: Group Runs

## Status

Accepted

## Context

A group is a list of actions that the command line runs as one command, such
as pre-commit. [ADR-019] decided how a group gets its flags: the projection
derives them from the arguments of the members, and it binds every member
before the run of the group starts. Point 1 of ADR-019 gave the rest of the
run to the group, which decides how it runs its members and what it reports.
The projection does not run the members, so the primitives that run a list of
actions stayed a decision of their own, as point 3 of [ADR-014] left them.

So today a group is a trait of the projection with a run of its own. The run
receives the bound members and the context of Clawless, and it writes the
report of each member through Clawless. Pre-commit is the only group in the
fleet. Its run asks each member for its access, as [ADR-017] declares it, and
it runs the members that write before the members that read, one at a time.
The next group has to write this runner again. But since ADR-017, each action
declares whether a run writes, so each copy derives the same order from the
same declarations.

The run also decides what a group answers. Pre-commit writes one report for
each member, so under `--json`, stdout carries one document for each member.
When a member finds problems, pre-commit returns an error, and it exits with
the code of a run that could not answer. Point 6 of [ADR-008] separates a
verdict from a failure for one action, and the run of pre-commit loses this
distinction.

[ADR-020] recorded the plan for the scheduler: it runs the readers of a group
at the same time, and it runs the writers one at a time. In this repository,
on 2026-10-29, the readers of pre-commit took 157 s one at a time and 121 s
all at once, and the writers took 4 s. ADR-020 left open where the scheduler
of a group lives. [ADR-004] puts the machinery that runs mounted actions in
the contract crate, and it keeps the projection thin.

Rakko prepares a public launch, and the launch freezes the public API. A
harness mounts groups, so the trait of a group is part of that API. Its run
takes the context of Clawless, so the frozen trait ties every group to the
types of Clawless. A crate that provides a group, such as pre-commit, depends
on the projection for the trait, and through it on Clawless. The contract
crate defines what a bundle exports, and it cannot refer to a trait of the
projection, so no bundle can export a group.

## Decision

A group is a type with a name and a list of actions, its members. The
scheduler of the projection runs every group, and a group has no run of its
own.

1. **A group is a name and a list of members.** A group implements a trait of
   the contract crate. The trait gives the name of the group and its members,
   and nothing else. The group is a concept of the public API, next to the
   action and the harness. Each group is a type, as each action is, so that a
   later derive can turn the doc comment of that type into the help text of
   the group.

2. **The scheduler of the projection runs every group.** The scheduler runs
   the writers of a group one at a time, in the order of the list. Then it
   runs all the readers of the group in parallel. A write is exclusive under
   point 3 of ADR-017, so a writer runs alone. The readers run after the
   writers, so that they examine the project as the writers left it. The
   access declarations decide which members write and which members read, so
   the order is the same for every group, and no group contains logic for it.
   The scheduler is internal to the projection.

3. **Every member runs to its end.** The scheduler asks every member for its
   access before any member runs. A member that finds problems or stops with
   an error does not stop the other members, because a run that stops at the
   first problem hides the problems after it. Two cases that come from the
   members stop the group before any member runs. The first is a member that
   cannot bind, as point 7 of ADR-019 decides. The second is a member that
   declares a kind of access that the scheduler does not know. The scheduler
   can neither order such a member nor run it safely, so the group stops with
   an error that tells the user to update Rakko.

4. **A run of a group answers like one action.** When the members run, the
   summary of the group is the one artifact of the run, and it holds the
   report of every member. So under `--json`, stdout then carries one document
   for the run. The report of each member also goes out as a message when the
   member finishes. The exit code follows point 6 of ADR-008 for the worst
   outcome of the members. A member that stopped with an error gives the code
   of an action that stopped. Otherwise, a member that found problems gives
   the code of an action with findings. Otherwise, the group exits zero.

5. **Progress never enters the JSON documents.** A view of the progress of a
   group, such as the members that wait, run, or finished, comes after the
   launch. This rule keeps that view an addition: it can reach a terminal with
   no change to the documents that a machine reads.

The trait of a group can live in the contract crate, because it carries no
output. Point 4 of ADR-014 keeps the trait of a command in the projection. A
command receives the context of Clawless and writes its output through it,
and ADR-004 and [ADR-007] keep Clawless out of the contract crate. A group
writes no output, because the scheduler reports for it. So the trait of a
group uses only types of the contract crate, and no type of Clawless. The
trait moves there before the launch, because the launch freezes that crate,
and an author of an action already depends on it. A crate that provides a
group then depends on the contract crate alone. The trait is an addition to
the contract crate, under the churn radius that ADR-004 describes.

The first alternative of ADR-014 turned down a pre-commit command that the
projection builds in, because the order of pre-commit was logic of this
project. Since ADR-017, every action declares whether a run writes, so the
projection can derive the order of every group from these declarations, by
the rule of point 2. The harness still decides which actions a group
contains, and the projection decides only how a list of actions runs.

This supersedes four parts of ADR-019. The first is the sentence that opens
its decision, where a group is a command that gives the projection the
actions that it runs. A group still gives the projection its members, but it
is a concept of its own, and the projection derives its command, as it
derives the command of an action. The second is point 1, where a group
decides how it runs its members and what it reports. The third is the
paragraph that partly supersedes point 3 of ADR-014, because a group is a
command that knows about actions. A group is no longer a command, so point 3
of ADR-014 holds again for every command that a harness writes. The fourth is
the paragraph that says that the projection does not run the members of a
group. That paragraph left the primitives that run a list of actions to a
decision of their own, and it kept point 2 of ADR-014, where a command is
arbitrary code, for a group. A group now has no code that runs.

ADR-019 stays Accepted, and its status refers to this ADR. Its other points
stand: the flags of a group come from its members, one name with two shapes
stops the harness at mount, each member receives only the values of its own
arguments, the help shows the sentence of each member, a group has no
arguments of its own, and a member that cannot bind stops the group before
any member runs. The command that the projection derives from a group shares
the flat namespace of point 5 of ADR-014, as the command of an action does,
so the rules that the projection enforces at mount still reach it. ADR-014
stays Accepted as well, and its status refers to
this ADR for point 3. A command that a harness writes knows nothing about
actions, and the primitives that it can call to run a list of actions stay a
decision of their own, because the scheduler is internal.

This also supersedes points 2 and 3 of ADR-004, for the run of a group only.
Point 2 puts the machinery that runs mounted actions in the contract crate.
Point 3 gives the projection no logic of its own beyond what it needs to
define its commands. The scheduler is such machinery, but it reports to the
user while it runs. The user sees the report of each member when the member
finishes, and a later view shows the progress of the group while it runs.
Clawless carries this output as a stream of events, and ADR-004 and ADR-007
keep Clawless out of the contract crate. A scheduler in the projection writes
into that stream directly. A scheduler in the contract crate needs a second
stream of its own, at the cost that the alternative "The Scheduler in the
Contract Crate" describes. ADR-004 stays Accepted, and its status refers to
this ADR. This ADR changes nothing else in ADR-004.

The decision records the direction and stops there. The names of the trait
and its methods, how a harness mounts a group, the shape of the summary, and
what the summary shows when a flag asks for less output are implementation
choices. The decision says nothing about groups that a bundle exports. The
trait in the contract crate keeps them possible, but no case for them exists
yet. The decision also says nothing about groups formed from tags, about a
limit on the readers that run at once, about writers that run in parallel, or
about a view of the progress of a group. A command that needs the scheduler
later, such as watch, can get it as an addition. Each of these gets work of
its own.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### A Run of Its Own

This is the trait of today. The trait keeps its run in the projection, and
each group decides how it runs its members and what it reports. A group can
then use an order or a report of its own.

But the run receives the context of Clawless, so the frozen trait ties every
group to the types of Clawless. The trait stays in the projection, so no
bundle can export a group. And each group writes the same runner again, which
derives the same order from the same declarations of access.

### A Run Without Clawless

The trait can keep its run and move into the contract crate. The run then
receives an output handle of the contract crate in place of the context of
Clawless. A bundle can then export a group, and each group still decides how
it runs.

This handle is the mirror of output that ADR-014 turned down in its
alternative "The Trait in the Contract Crate". It is a second output surface,
in the crate whose churn radius is the whole fleet, and it has to follow
Clawless release by release. Each group also still writes its own runner.

### A Trait Without a Run in the Projection

The trait can lose its run and stay in the projection. The contract crate
then does not grow, and only a harness and a crate that provides a group see
the trait.

But an author learns the action from the contract crate, and the group is a
concept next to the action. A crate that provides a group would still depend
on the projection, and through it on Clawless, for a trait that uses no type
of Clawless. The launch also freezes the place of the trait, so a later move
into the contract crate needs a bridge in the projection.

### The Scheduler in the Contract Crate

The scheduler can live in the contract crate, where point 2 of ADR-004 puts
the machinery that runs mounted actions. The scheduler then returns the
outcome of each member, and the projection only binds the members, calls the
scheduler, and renders what it returns. The scheduler is then in the crate
that defines the kinds of access, so it knows every kind, and no member can
stop the group with a kind that is unknown.

But the scheduler cannot write output there, so it cannot show the report of
a member when the member finishes. It needs a stream of events of its own,
and the projection has to forward that stream into the stream of Clawless. A
callback that receives each finished member is enough for now. The view of
the progress of a group also needs events for the members that wait and run,
so the callback grows into that stream. This is the mirror of output that
ADR-014 turned down, and the launch freezes it in the crate whose churn
radius is the whole fleet.

### A Group as a Value

A group can be a plain value, a name and a list of actions that the harness
gives to the projection, with no trait. This is the smallest API, and nobody
writes a type for a group.

But a value has no doc comment. A group that is a type has one, and a later
derive can turn it into the help text of the group, as it can for an action.
A value needs a second way to document the group.

## Consequences

- A harness writes no runner for a group. Every group gets its order, the
  readers in parallel, the summary, and the exit code from the projection.
  Pre-commit keeps its crate and its list of actions, and it loses its run.
- No group can run its members or report in a way of its own. An activity
  that needs such a way is a command that a harness writes, and that command
  calls its actions as libraries, as ADR-014 describes.
- The readers of a group run at the same time. In this repository, a run of
  pre-commit then takes about as long as its writers and its slowest reader
  together. The report of each reader appears when the reader finishes, so
  the reports of the readers do not follow the order of the list.
- Two readers that share state that point 4 of ADR-017 does not count as a
  write, such as a cache of a tool without a lock, can now collide. The
  consequences of ADR-017 expected this, and they gave the question to the
  work on the scheduler. This decision does not change that rule. On
  2026-10-29, all the readers of pre-commit in this repository ran at once,
  and the readers that build with cargo share a `target/` directory that
  cargo locks itself. A harness whose readers collide takes one of them out
  of the group, because a group cannot run its readers one at a time.
- Under `--json`, a run of a group writes one document, and not one document
  for each member. A reader that parses the output of pre-commit today has to
  change. CI can now tell a group with findings from a group that could not
  answer, from the exit code alone.
- The reports of the members are messages, and the flags that ask for less
  output hide messages. Under `--quiet`, a reader at a terminal sees only the
  summary of a group. Today pre-commit sends each report as an artifact, so
  `--quiet` still shows it.
- The change breaks every crate that provides a group and every harness that
  mounts one, and the contract crate grows by the trait of a group. The break
  lands before the public launch.
- The projection grows by the scheduler. Point 3 of ADR-004 kept the
  projection thin, and its consequences ask a reviewer to send logic from
  the projection down into the contract crate. That rule no longer covers
  the run of a group.
- A new kind of access can stop a group that ran before. A new kind comes in
  a compatible release of the contract crate, so Renovate can update an
  action, and with it the contract crate, before the harness takes a newer
  projection. Every group that contains that action then stops before any
  member runs, as point 3 decides, until the harness updates the projection.
  A scheduler in the contract crate would know every kind.
- The scheduler is internal. A command that a harness writes cannot run a
  list of actions through it. Such a command still calls its actions as
  libraries, and it gets no summary of a group.
- ADR-019 now covers only the arguments of a group. A reader reads ADR-019
  and this ADR together.

[adr-004]: 004-actions-as-libraries.md
[adr-007]: 007-argument-vocabulary.md
[adr-008]: 008-renderers.md
[adr-014]: 014-commands.md
[adr-017]: 017-access-declarations.md
[adr-019]: 019-group-arguments.md
[adr-020]: 020-access-granularity.md
