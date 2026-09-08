# ADR-014: Commands

## Status

Accepted

## Context

[ADR-004] made the action the unit of maintenance work: a library crate
behind the `Action` trait, with the command line as a projection that
derives one command from each action a harness mounts. That command runs
the action once, the action answers with one outcome, and the renderers of
[ADR-008] turn the outcome into output and an exit code. Every activity
that Rakko runs today has this shape.

Some maintenance activities of a project do not. A pre-commit run drives
many actions in one shot, the formatters in sequence and then the checks
in parallel, and reports on all of them. A watch command would rerun them
each time a file changes, and never return. A development command would
start a server. In this repository these activities live in the justfile,
as the shell scripts that the [vision] names as the part bash is worst at,
and the justfile is what Rakko exists to replace. The command line of a
harness cannot be the one interface of a project while these activities
sit outside it.

The shape that these activities need is the one that ADR-004 rejected. It
turned down actions as Clawless commands, because registration in Clawless
"does not compose across crate boundaries, so a harness cannot mount
commands that a dependency exported," and because a command-shaped action
couples every action to the framework, so that "each Clawless release
would cascade through the fleet." Both forces speak of actions, which
travel between crates and stand underneath the whole fleet. An activity of
one project travels nowhere. It is written in the harness of that project,
the one crate that already depends on the projection and on Clawless, and
no crate depends on it in turn. A reader who finds a Clawless command in
Rakko needs the record that says why this is not what ADR-004 ruled out,
and where such a command fits the architecture that ADR-004, [ADR-006],
and [ADR-007] describe.

## Decision

A command is an ordinary Clawless command that a harness writes, for a
maintenance activity of its project that does not fit an action.

1. **A command is the escape hatch of a project.** Any maintenance activity
   that does not fit the shape of an action, one run that returns one
   outcome, is a command. It stays scoped to the project, and it follows
   the conventions of an action: it runs in the project root of [ADR-010],
   it carries the shared flags, it documents itself in the help, and it
   sits in the same command tree. The command line of a harness is the one
   interface that a project documents, from the checks that CI runs to the
   environment that a contributor develops in.

2. **A command is arbitrary code.** A harness writes it as an asynchronous
   function that receives the context of the project and the context of
   Clawless, writes its output through Clawless, and returns what a
   Clawless command returns: success or an error, and no value. Nobody can
   foresee how a project fills it, so nothing in Rakko shapes what a
   command does or what it reports.

3. **A command knows nothing about actions.** The projection hands a
   command no view of the actions that the harness mounted and no
   primitive to run them. A command that runs actions calls them as the
   libraries that ADR-004 made them. Primitives that run a list of actions
   are a decision of their own, and a command can call them when they
   exist.

4. **The trait lives in the projection.** A command receives the Clawless
   context, and ADR-004 and ADR-007 keep Clawless and every command-line
   parser out of the contract crate, so the trait cannot live there. It
   mirrors the action trait: a name, documentation, arguments in the
   vocabulary of ADR-007, and an asynchronous run, as [ADR-005] made the
   run of an action. Only the mount differs. The contract crate does not
   change, and a harness, which already depends on the projection and
   takes the releases of Clawless with it, absorbs nothing new.

5. **Commands and actions share one flat namespace.** The harness mounts
   commands as a second list beside its actions, in code, as ADR-006 asks
   of everything that runs. A user of the command line sees one flat list,
   and the rules that the mount enforces on the names of actions and of
   their arguments reach a command unchanged: a name that collides, within
   a kind or across kinds, stops the harness where it mounts.

6. **The code of a command lives in the harness package.** By convention
   it is not written in `main.rs`, so that the file stays the list of names
   that ADR-006 wants a reviewer to read. Beyond that, the structure is the
   project's own.

The decision stops at the concept. The signature of the trait, the name of
the mount, the exit code of a command that fails, and whether a closure
form follows the trait belong to the interface. Whether a bundle can ship a
command waits for a bundle that has one worth shipping: it needs the trait
in the contract crate and an output surface there, and no command needs
either yet. Which activities are commands is learned from practice. The
action trait is meant for any maintenance activity of a project, not only
for the checks of CI, and the first publish or site build to arrive decides
with a real case.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### A Command Built Into the Projection

The projection can ship the activities itself: a `pre-commit` command that
runs the actions the harness names as its steps, and a `watch` command
beside it. A spike built the first, and every project would get both for
free.

The projection then decides what pre-commit means for every project, and
the order that this repository needs, the rewriters in sequence and then
the readers in parallel, is logic of this project. The list of activities
is open-ended as well. Each one the projection did not foresee becomes a
release of the crate that every harness depends on, and a project waits
for it. An escape hatch that has to be built in is not an escape hatch.

### The Trait in the Contract Crate

The trait can live beside the action trait, with an output handle that
mirrors the events of Clawless, so that a bundle can ship a command the
way it ships an action, and the fleet can roll it out.

The mirror is a second output surface, in the crate whose churn radius is
the whole fleet, that has to track Clawless release by release, and no
command needs it yet. The one command that will want output of its own is
watch, and watch points at a terminal user interface that no mirror of
events would carry. The question comes back with the first bundle that
has a command worth shipping, and a real case shapes the answer.

### A Declarative Command

A command can return a plan, the actions to run and how, and the
projection can run the plan. The projection keeps control of every run,
the output stays uniform, and a scheduler can take the plan when it
exists.

A plan cannot loop and cannot listen. It describes pre-commit and nothing
after it: a watch command waits for files, a development command holds a
server open, and neither ends in a list. What a plan is good for, running
a list of actions, is the primitive that point three defers, and a command
can call that primitive when it exists.

### A Command With a Parser of Its Own

A harness depends on Clawless anyway, so a command can define its command
line directly, with the derive of a parser, and skip the vocabulary.

ADR-007 gives every naming and syntax decision to the projection, so that
the command line is uniform across the fleet. A command that names its own
flags takes that decision back, and its flags follow a rule that the flags
of the action beside it do not. The vocabulary exists, and the mechanics
that turn it into flags serve a command unchanged.

### A Local Action

A harness can write pre-commit as an action of its own, with no new trait
at all, and mount it beside the others.

An action returns one outcome, and one outcome is what these activities
cannot fit. Pre-commit produces one per action that it ran, and a reader
wants each of them, while a watch command never returns at all. An action
that folds or drops what it learned bends the contract that the renderers
and the exit codes of ADR-008 rest on.

## Consequences

- The command line of a harness can carry everything a project does to
  maintain itself. In this repository, pre-commit moves out of the
  justfile and into the harness, and it is the first test of the trait: if
  the trait feels heavy in the writing, a closure form follows, and if
  pre-commit needs primitives, they get a decision of their own.
- The harness stops being a list of names. It becomes a package with code
  of its own, and nothing mechanical keeps that code small. ADR-006 asked
  review to hold a harness to a few lines of naming; the line now runs
  through `main.rs` rather than around the package.
- A command is not portable. It lives in one harness, and no bundle rolls
  it out, so the next project writes its pre-commit again. This is the
  duplication that the vision exists to end, accepted for now, because
  what a shared command should look like is unknown until several exist.
- The uniform output of ADR-008 does not reach a command by construction.
  A command writes through Clawless, and Rakko cannot make it report an
  outcome, so what a command says about the actions it ran is its own
  work until the primitives exist. One binary carries two output paths.
- The exit code of a command says success or failure, where the exit code
  of an action separates a verdict from a failure. CI reads both as pass or
  fail, and a command that runs actions folds their verdicts into its
  result by hand.
- Clawless reaches a harness as an interface, not only as a dependency. A
  Clawless release could break the projection before; now it can break a
  command that a project wrote, and the project repairs it.
- The vocabulary of ADR-007 gains a second consumer, and pressure on it
  now comes from what a project wants to say to a command, such as a port
  or a path, not only from what an action reads.
- The help lists derived and written commands as one, and a reader cannot
  tell them apart. The documentation of a command has to say what it does,
  because nothing else does.
- The word "command" changes meaning. The glossary said the projection
  derives one per action; now a harness writes the others, and the
  distinction that matters is written or derived, not two nouns.

[adr-004]: 004-actions-as-libraries.md
[adr-005]: 005-asynchronous-actions.md
[adr-006]: 006-composition-is-explicit-data.md
[adr-007]: 007-argument-vocabulary.md
[adr-008]: 008-renderers.md
[adr-010]: 010-project-root.md
[vision]: ../VISION.md
