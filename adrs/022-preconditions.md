# ADR-022: Preconditions

## Status

Accepted

## Context

[ADR-021] makes the actions pin their tools. Rakko generates a mise file
from the declarations of the mounted actions, and the project commits it. A
run is then correct only if the environment of the project agrees with the
declarations. The generated file must match them, no pin of the project can
override a declared tool, and each toolchain must be in the range that an
action accepts. Something has to report a project that breaks one of these
conditions, and something has to repair it.

Every harness needs this, and not only the harnesses that mount a given
bundle. A project that has no way to write the file gets errors from each
action that resolves its tool, and no repair. The check also needs the
declarations of every action that the harness mounts. An action sees only
its own context. Point 3 of [ADR-014] gives a command no view of the mounted
actions at all.

[ADR-006] and ADR-014 let only the harness decide what runs. The harness
mounts the actions, it writes the commands, and it mounts both as lists in
code. So a reader of the harness sees everything that runs. A check that
every harness gets, with no code in the harness, is outside that model.

Rakko prepares a public launch. Commands and actions share one flat
namespace, and a name that collides stops the harness where it mounts (point
5 of ADR-014). A command that the machinery adds after the launch can take a
name that a harness already uses, and that harness then stops working.

## Decision

Before the projection runs anything, it checks the preconditions of Rakko. A
precondition is a condition about the environment of the project that a
correct run needs. The command `doctor` reports the preconditions, and
`doctor --fix` repairs them.

1. **Preconditions are machinery.** The projection checks them in every
   harness, and a harness writes no code for them. Only Rakko defines a
   precondition. No public API lets an action or a harness add one. A
   harness that wants checks of its own runs them before it builds the
   command line.

2. **A failed precondition stops the run.** Every action, group, and written
   command stops, also in CI, and the message says how to repair the cause.
   Only the help and `doctor` still run. A warning instead of a stop lets a
   broken environment pass a pull request without a sign.

3. **Preconditions need no network.** A precondition reads the project and
   asks mise about it, so a run offline checks the same conditions as a run
   in CI. A check that needs the network, such as the comparison of the
   lockfiles of mise, stays an action. A network error then stops one action
   and not every command.

4. **`doctor` is a command of the machinery.** The projection adds it to
   every harness, and its name is reserved: a harness cannot mount a command
   or an action with that name. `doctor` reports each precondition. With
   `--fix`, the flag that an action uses for a repair, it repairs each
   precondition that it can repair.

5. **The first precondition covers the tools.** The generated file of
   ADR-021 matches the declarations of the mounted actions. No pin of the
   project overrides a declared tool. Each toolchain that a mounted action
   requires is pinned and in its range. A project whose actions declare no
   tool needs no file. `doctor --fix` writes the file, removes each pin that
   overrides a declared tool, and locks the file with mise.

ADR-006 still holds for actions: an action runs only when a harness mounts
it. This supersedes the part of the decision of ADR-006 that the code of the
harness shows everything that runs. The preconditions and `doctor` run in
every harness, and the harness does not list them. ADR-014 still describes
every command that a harness writes. For `doctor`, this supersedes the part
of point 5 of ADR-014 that the harness mounts every command. Point 3 of
ADR-014 keeps the mounted actions away from a written command. `doctor` is
part of the projection, which holds them anyway.

The decision records the direction and stops there. How the projection runs
the checks, the words of a message, and the output of `doctor` are
implementation choices. Other preconditions can come later, each with a
decision of its own.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### An Action in a Bundle

An action in the baseline bundle can check the generated file and write it
with `fix`. It runs in a group, such as pre-commit, and in CI like every
other check. But it helps only the projects that mount the bundle. The
context of an action also has to carry the declarations of every other
mounted action, which no action needs otherwise.

### A Command That the Harness Mounts

A crate can export the check as a command, as `set-msrv` is one, and each
harness mounts it. The harness then shows what runs, as ADR-006 asks. But
point 3 of ADR-014 gives a command no view of the mounted actions. So the
harness has to give the same list a second time. A harness that leaves the
command out gets errors from its actions and no repair.

### A Repair Before Every Run

The projection can write the generated file on its own before each run, so
that the file is never stale. But a run in CI then changes a committed file
without a sign, and the stale file merges. Locking the file also needs the
network.

### A Public API for Preconditions

The contract can let an action or a harness add preconditions of its own. No
action needs one today, and the launch freezes every public item. A harness
can already run its own checks in its `main` before it builds the command
line. So an API adds nothing that a harness lacks.

### The Lockfile Check as a Precondition

The comparison of the lockfiles of mise can run before every command instead
of as an action. But it needs the network, so every command stops offline or
at a network error. An install of mise in locked mode already stops a job of
CI when the lockfile lacks its platform. The check of the other platforms is
a property of the project, and not a condition for a correct run.

### A Flag Instead of a Command

The projection can offer the repair as a flag on the command line of the
harness, and not as a command. A flag is less likely to collide with a
command of a harness. But a flag is harder to find in the help, and it is a
second way to start a run, next to the commands.

## Consequences

- Every harness gets the check of the tools and the command `doctor` with no
  code of its own. A project that adopts Rakko runs `doctor --fix` once, and
  each project of the fleet does the same when it moves to ADR-021.
- A stale file or an override stops every command, also in CI. A Rakko
  update that changes a tool therefore fails in each project until the file
  is written again. How that update writes the file is still open in
  ADR-021.
- Every run pays for the check before it starts. The check reads the mise
  files of the project and asks mise about the toolchains. It needs no
  network.
- A harness cannot use the name `doctor` for a command of its own. The name
  is reserved before the public launch, so that no harness loses a name
  later.
- The code of a harness no longer shows everything that runs. The
  preconditions and `doctor` run in every harness, as the help and the
  output flags already do.
- A new precondition reaches every project with the next update of Rakko,
  and it can stop runs that passed before. So a new precondition needs a
  repair through `doctor --fix`, or a message that says what to change.
- A harness that needs checks of its own writes them in its `main`. Rakko
  gives it no hook for them.

[adr-006]: 006-composition-is-explicit-data.md
[adr-014]: 014-commands.md
[adr-021]: 021-tool-declarations.md
