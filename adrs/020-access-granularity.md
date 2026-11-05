# ADR-020: Access Granularity

## Status

Accepted

## Context

[ADR-017] lets an action declare whether a run reads or writes the project.
In its first version, a declaration covers the whole project. [ADR-018] made
the declaration finer. An action also declares glob patterns for the files
that a run reads and for the files that it writes. The patterns serve only
the scheduling of runs: a scheduler could run a writer at the same time as
another run when their files cannot overlap, such as the formatter for
Markdown and the formatter for YAML.

That kind of scheduler is no longer planned. The scheduler that Rakko plans
is simpler. It runs the readers of a group at the same time, and it runs the
writers one at a time, so it does not need to know which files a run
touches. Rakko reads far more than it writes: an action writes only when a
run passes `fix`, and in this repository only the pre-commit hook passes
it. Its five writers take about 4 seconds together, so a scheduler that
runs a writer at the same time as another run would save at most these 4
seconds.

Nothing outside the tests reads the patterns. The contract crate holds the
types and the method for them, and the helper crates and eleven actions
declare patterns, each with tests. But no harness, no command, and no
scheduler uses them, in this repository or elsewhere in the fleet.

Rakko prepares a public launch, and the launch freezes the public API of the
contract crate. Every action and every harness depends on that crate
([ADR-004]). After the launch, a method on the `Action` trait cannot go away
without a breaking change. A new method with a default can still come in
without one. So the patterns cost less to remove now, and to add again if a
consumer needs them, than to keep through the launch.

## Decision

An action declares only whether a run reads or writes the project. A
declaration covers the whole project, as ADR-017 decided, and it holds no
patterns of files.

1. **ADR-017 holds as it was accepted.** A write is exclusive access to the
   whole project, and a read is shared access to it, except a directory that
   the tool locks itself, as point 3 of ADR-017 says. When the answer
   depends on the configuration of the project, the action declares the
   cautious answer, which is a write. ADR-018 replaced this rule of point 2
   for the files that a declaration covers. Without patterns, that
   replacement has nothing left to apply to.

2. **Each tool keeps its own discovery.** Rakko gives no list of files to a
   tool. The tool selects its files from its own configuration and from any
   pattern that the action gives it, as [ADR-011] and [ADR-013] require.
   ADR-011 left open whether the discovery of the files that a run touches
   moves out of the actions and into the machinery. ADR-018 answered that it
   stays with the tool, and this decision keeps that answer.

This decision supersedes ADR-018 as a whole. [ADR-019] mentions ADR-018 as
a step of the direction on access, and it stays as it was accepted.

A declaration that covers less than the whole project can return later. It
needs a consumer, such as a scheduler that runs a writer at the same time
as another run, and a decision of its own.

The decision records the direction and stops there. How the declarations of
files leave the code is an implementation choice. The decision says nothing
about where the scheduler of a group lives, about the isolation of a run, or
about a sandbox. Each of them gets work of its own.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Keep the Patterns for Later

The patterns can stay on the trait without a consumer, ready for a scheduler
that may come later. Nothing has to come out of the code.

A declaration without a consumer has no way to show that it is wrong, except
its own tests. Each new action that wraps a tool that selects its files by
pattern would still have to write and test its patterns. The launch would
also freeze the method, so it could not go away without a breaking change,
even if no scheduler ever comes. A method with a default can come in after
the launch without a breaking change, so the patterns lose nothing if they
wait for a consumer.

### Build a Scheduler for the Patterns

Rakko can build the scheduler that ADR-018 expected, so that the patterns
get a consumer. A writer then runs at the same time as every other run whose
files cannot overlap with its own.

Rakko reads far more than it writes, so a faster run of the writers has
little to gain. Of the 25 actions in this repository, 7 can write, and each
of them writes only when a run passes `fix`. Only the pre-commit hook passes
it, and every run in CI only reads. In pre-commit, the five writers take
about 4 seconds together. The readers take about 157 seconds one at a time
and about 121 seconds at the same time, because they cannot finish before
the slowest of them, the tests of the Rust code. The simpler scheduler gets
the gain of the readers. A writer beside other runs would save at most the
4 seconds of the writers, at the cost of a finer scheduler, its tests, and
patterns in every action whose tool selects its files by pattern.

### Deprecate ADR-018

[ADR-001] marks an ADR Deprecated when it no longer applies and no newer ADR
replaces it. ADR-018 could become Deprecated, and ADR-017 would describe
access alone.

ADR-018 does not stop applying on its own. This ADR gives a different answer
to the same question, how much of the project a declaration covers, and it
keeps the answer that ADR-018 gave to the question of ADR-011. That is a
newer ADR that replaces an older one, which ADR-001 calls Superseded.

## Consequences

- The `Action` trait goes into the launch with read or write access only.
  The contract crate, the helper crates, and eleven actions lose their
  declarations of files and the tests of these declarations.
- A writer always runs alone, as points 3 and 5 of ADR-017 promise, even
  when its files cannot overlap with those of another run. The formatter for
  Markdown waits for the formatter for YAML. In this repository, this costs
  at most about 4 seconds of a pre-commit run.
- The scheduler needs no lists of files to keep that promise. So the gap
  that ADR-018 described, where a writer creates a file that is in no list,
  goes away.
- ADR-017 again describes access on its own. A reader no longer has to read
  its points 2 and 3 together with another ADR.
- A tool still finds its own files. Work on the isolation of a run, on an
  ignore list for the whole project, or on runs over the staged files cannot
  give a list of files to a tool without a new ADR.
- The table of ADR-018 stays as a record of how each tool treats a file in
  an explicit list. Work that gives files to a tool can start from it.
- A finer declaration can return as a new method with a default that covers
  the whole project. That is an addition to the contract, and it breaks no
  action. It needs a consumer and a decision of its own.
- The [vision] expects a scheduler that runs two formatters at the same time
  because their declarations cannot overlap. No work in Rakko leads there
  now, and a return to that goal needs the finer declaration above.
- ADR-019 still refers to ADR-018 as the step on access. A reader follows
  the status of ADR-018 to this ADR.

[adr-001]: 001-adrs.md
[adr-004]: 004-actions-as-libraries.md
[adr-011]: 011-tool-integration.md
[adr-013]: 013-applicability.md
[adr-017]: 017-access-declarations.md
[adr-018]: 018-file-access.md
[adr-019]: 019-group-arguments.md
[vision]: ../VISION.md
