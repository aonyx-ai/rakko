# ADR-017: Access Declarations

## Status

Accepted

## Context

Some actions change the project and some only read it, and a run that drives
several actions has to know which is which. Pre-commit runs the actions that
write first, so that the actions that read examine what the commit will
contain. Nothing on an action says which kind it is, so every harness that
mounts pre-commit sorts its actions by hand into two lists. Across the ten
harnesses of the fleet, 122 of 181 dependency lines on Rakko name an action
that a mounted bundle already exports, only to put it in one of these lists.
The lists already drift: the action that lints TOML is missing from the list
of readers in four harnesses.

The kind of an action is not fixed. It depends on the arguments of a run: a
formatter writes only when its `fix` argument is set, and it only reads
otherwise. A label that an action carries for all its runs cannot give this
answer, and a harness that sorts by hand cannot give it either.

Earlier decisions expect an answer. The glossary describes access as a
declaration that is a function of the arguments of an action, with exclusive
access to the whole project as the default. [ADR-010] gives the root of the
project as the extent of that default, and [ADR-011] leaves open whether the
discovery of the files that a run touches moves into the machinery, together
with these declarations. No ADR decides what a declaration is, and the
`Action` trait has no place for one.

This is the first step of a longer direction. An action declares interfaces:
its arguments, its tags, and its access. The machinery derives features from
these interfaces, such as group commands, argument merging, scheduling, and
the placement of a run, so that no harness hard-codes what an action can say
about itself. Access comes first, because the order of pre-commit needs it
now. This ADR decides nothing about tags, argument merging, or the scheduler.
Each of them gets work of its own.

A declaration changes the contract crate. [ADR-004] gives that crate a churn
radius of the whole fleet, because every action and every harness depends on
it. A change of this size needs a decision on record before the code.

## Decision

An action declares whether a run reads or writes the project, after its
arguments are bound and before it runs.

1. **A run takes three steps.** First, the argument values of a run are
   bound to the typed arguments of the action, once. Then the bound action
   declares its access. Then the same bound action runs. The declaration and
   the run see the same arguments, so the declaration describes the run that
   follows it.

2. **The declaration does no I/O.** It sees the bound arguments and the
   context of the run, and nothing else. It reads no file and calls no tool.
   When the answer depends on the configuration of the project, the action
   declares the cautious answer, which is a write.

3. **A write is exclusive and a read is shared.** An action that declares a
   write gets exclusive access to what it names: no other action reads or
   writes it during the run. An action that declares a read gets shared
   access: other actions that read can run at the same time. In this first
   version, a declaration names the whole project and nothing narrower,
   except a directory that the tool locks against parallel use itself, such
   as the `target/` directory of cargo. That directory stays unnamed. The
   action uses it, and Rakko gives no guarantee about it.

4. **An action writes only when its run changes files that another action
   could read or change.** Three kinds of change do not count as a write:

   - a change that only the tool itself reads, such as a `.tsbuildinfo` file
     or a `.tracey/` directory,
   - a change inside `.git/`, which Git guards with locks of its own, and
   - a change in a directory that the tool locks itself.

   A declaration covers the action and its tool, and not the code of the
   project that the tool runs. So an action that runs the tests of a project
   reads, although tests, build scripts, and procedural macros can write
   anything, as long as the tool itself changes nothing that counts. A
   lockfile that cargo refreshes during the run counts, because other
   actions read it.

5. **An action that declares nothing writes the whole project.** The contract
   gives every action this default, so an action that declares nothing keeps
   working unchanged. The default never lets two runs change the project at
   the same time, because a writer runs alone. Its cost is that a reader
   that declares nothing runs among the writers. It waits for them, and in
   pre-commit it can examine the files before a formatter changes them.

This decision adds to the `Action` trait, and so to the contract crate of
[ADR-004], under the churn radius that ADR-004 describes. The declaration
cannot live in a smaller crate: a bundle exports erased actions, which the
contract crate defines, so only the contract can carry the answer of an
action through a bundle to the command that runs it. The decision keeps
[ADR-013]: an action still examines nothing before it runs its tool, because
the declaration reads only what the action was given. The declaration is not
the applicability that ADR-013 removed from the design. It says how a run
uses the project, not whether the action applies to it.

The decision records the direction and stops there. The shape of the bound
action, the type of the declaration, the names of the methods, and how the
erased action carries the declaration are implementation choices. Which files
an action reads or writes, in names narrower than the whole project, is a
question of its own. So are access for the commands that a harness writes,
the order and parallelism that a scheduler derives from these declarations,
and where a run takes place.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Two Lists in the Harness

The harness can keep the lists. It names each action that guards a commit
and puts it among the writers or the readers, as it does today.

The lists repeat a fact about the action in every harness that mounts it, and
the copies already drift. And a list cannot follow the arguments: a formatter
is a writer in a run with `fix` and a reader in a run without it, and the
lists hold one answer for both.

### A Static Declaration

The trait can carry a fixed answer for each action, which the machinery
reads without arguments. The declaration then needs no bound arguments, and
it is the smallest change to the trait.

A fixed answer is wrong for every action whose arguments decide it. A
formatter would have to declare a write for each run, so a check without
`fix` would run alone among the writers. The step that binds the arguments
first is the cost of a correct answer, and the first version needs it,
because the formatters are the actions that pre-commit orders.

### A Declaration That Examines the Project

The declaration can read the configuration of the project, or ask the tool,
and give a precise answer where the configuration decides.

[ADR-013] removed every look that an action takes before it runs its tool,
because a look mirrors the discovery of the tool and can disagree with it. A
declaration that reads the project brings that look back. The cautious answer
costs some speed where the configuration decides, and it cannot disagree with
the tool.

### Access in the Outcome

The run can report what it read and wrote in its outcome, and no declaration
comes before the run.

A report that arrives after the run cannot order the run. Pre-commit has to
know that an action writes before it starts the action, and so does any
scheduler that follows.

### Named Files From the Start

A declaration can name the files that a run reads and writes from the first
version. Then two actions that write different files can run at the same
time.

[ADR-013] leaves the discovery of the files that a run touches with the tool,
and [ADR-011] left open whether it moves into the machinery together with
these declarations. To move that discovery, or to mirror it, is a larger
question, and it gets a scoping pass of its own. The order of pre-commit
needs only read or write. A declaration that names the whole project gives
that answer now, and a narrower name can come later without a change of the
three steps.

### No Default

Every action can be made to declare its access, with no default in the
contract. A missing declaration then fails the build instead of a slow run.

The local actions of other projects of the fleet do not declare access, and
they would all break on the release. The default of the glossary keeps them
working. Its cost is speed and order, not two writes at the same time: an
action that forgets to declare runs as a writer.

## Consequences

- Pre-commit can take one list and run the writers before the readers itself,
  so a harness no longer sorts its actions into two lists. A declaration
  does not order two writers against each other. Where the order of two
  writers matters, such as two formatters that write the same file, the
  order of the list still decides it, and this ADR leaves that order open.
- The `Action` trait grows, and every action and every harness takes a
  release of the contract crate. The default makes the change to an action
  optional, but the release still reaches the whole fleet.
- An action author answers one question for each run, from the arguments
  alone. Where the arguments do not settle it, the answer is a write, so some
  runs that only read wait behind the writers.
- Nothing enforces a declaration yet. An action that declares a read and
  writes anyway can change files that another action reads at the same time.
  Until a sandbox enforces the declarations, tests have to check that a
  reader leaves the files of the project unchanged.
- The writes that this ADR does not count stay outside every declaration: the
  state that only a tool reads, the changes inside `.git/`, and the
  directories that a tool locks itself. Two readers that share such state
  can still collide when they run at the same time: the cache of a tool can
  have no lock, and the locks of Git make a second run fail instead of
  wait. A sandbox that syncs back only the named writes would also lose
  these writes. The work on the scheduler has to revisit this rule.
- An action that runs the code of the project declares only what it does
  itself. A test that writes into the project is outside the guarantee, and
  the project is responsible for it.
- Access becomes the first interface that an action declares and the
  machinery consumes. Tags, argument merging, and the scheduler can follow
  the same pattern, but each of them needs a decision of its own.

[adr-004]: 004-actions-as-libraries.md
[adr-010]: 010-project-root.md
[adr-011]: 011-tool-integration.md
[adr-013]: 013-applicability.md
