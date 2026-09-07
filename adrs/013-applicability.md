# ADR-013: Applicability

## Status

Accepted

## Context

[ADR-004] gave every action the judgment of whether it applies to a project,
and [ADR-011] left that judgment with the action once actions began to wrap
tools. Both decisions are older than the actions that carry them, and the
implementation settled a question that neither one asked: how an action
reaches the judgment. It reaches it with a cheap look of its own, a walk of
the project that mirrors the discovery of the tool that the action wraps,
and the walk runs before the action resolves that tool.

The mirror is a second discovery of the same files, and every specification
that describes one concedes that the two can disagree at the margins. A
disagreement that skips hides a real check behind a message, which is the
failure that [ADR-004] named in its own consequences. The tools answer the
same question, and they answer it from the configuration that the project
wrote: yamllint lists the files it selected, markdownlint reports a
selection that is empty, taplo counts what it found, and prettier refuses a
pattern that matched nothing. The action that lints YAML already asks twice.
It walks the project, resolves the tool, and then asks the tool which files
it examines, and the accurate answer is the one that arrives second.

The look also buys less than its shape suggests. It never reached the trait
that [ADR-004] placed it in, so it sits inside the run, and nothing can ask
whether an action applies without running it. A listing cannot filter with
it and a scheduler cannot plan with it. What it returns today is one message
and one process that did not start.

What it costs is plain. Roughly a fifth of the requirements of each action
describe the walk and the skip, and five wrappers carry a walker with a
suite of its own. Two actions cannot skip at all: [ADR-010] marks a project
root with a TOML file, and the TOML tool reads that file, so their skip is
verified only against fixtures that are not projects.

## Decision

Applicability is what a tool reports, not what an action looks for.

1. **An action examines nothing before it runs its tool.** The look goes,
   and the requirements that describe it go with it. An action resolves its
   tool and runs it.

2. **A skip comes out of the run.** A tool that reports it had nothing to
   examine produces a skipped outcome, with the reason that the tool gave.
   One discovery then answers both what the run examined and what it found,
   so the two cannot disagree.

3. **Applicability stays out of the trait.** [ADR-004] put it there and the
   code never did. Nothing needs the answer before a run today, and a
   decision that eighteen actions have not implemented should not be
   implemented now in order to be deleted later.

This supersedes point 5 of [ADR-004] and the consequence of [ADR-011] that
follows from it. The rest of both stands, and a tool that a project has not
provisioned still stops the action. Whether such a project should skip
instead is a real question, and it belongs to bundles: only an action that a
project did not choose for itself can be mounted for a tool that the project
does not have. Bundles do not exist yet, and the first one can answer it.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Applicability in the Trait

The shape that [ADR-004] described can be built instead of removed. A trait
method would let a listing filter, a scheduler plan, and a reviewer see the
judgment as part of the interface rather than buried in a run.

The method has to answer before the tool runs, so it is the same mirrored
discovery with a signature in front of it, and the drift it must avoid is
the drift it cannot see. Nothing consumes the answer early today either. The
scheduler is unbuilt, and the question a scheduler asks is which files an
action touches, not whether it applies, so building the method now would
guess at a consumer twice.

### A Pass Instead of a Skip

The skipped outcome can go as well. An action with nothing to examine passes
with a summary that says so, and the exit code, the report schema, and the
renderers each lose a case.

A pass and a skip answer different questions. A contributor who mounted an
action wants to know that it examined nothing because there was nothing, not
that everything it examined was fine. The state costs one variant once the
discovery behind it is gone, and it is the variant that keeps a green run
honest.

### The Look for the One Tool That Needs It

Prettier is the one tool that treats a pattern matching no file as an error,
so the walk could stay for the three actions that wrap it and go everywhere
else.

Prettier names that condition in its own output, like every other answer a
wrapper reads. Keeping the walk for it would trade one rule for two, and it
would keep the mirrored discovery alive in the wrapper with the most callers.

## Consequences

- Each action loses roughly a fifth of its requirements, along with the tests
  behind them, and five wrappers lose a walker each.
- A run does one discovery instead of two, and the answer comes from the tool
  that the editor and the contributor also run. A skip can no longer
  contradict the check it replaced.
- A skip now costs a process start. The walk was cheaper, and it was also
  wrong in a direction that hides problems.
- A project that mounts an action for a tool it does not provision stops
  where it used to skip. No project does this today, because a harness names
  what it mounts and pins what it names.
- Applicability stops being something an action author implements. It
  becomes a shape that a wrapper reads out of its tool, so the glossary
  describes a property of a run rather than a duty of an action.
- The trait stays as it is, and the drift between it and [ADR-004] is closed
  by the record rather than by code.

[adr-004]: 004-actions-as-libraries.md
[adr-010]: 010-project-root.md
[adr-011]: 011-tool-integration.md
