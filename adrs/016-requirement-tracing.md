# ADR-016: Requirement Tracing

## Status

Accepted

## Context

[ADR-002] gave each crate a specification, and it used [Tracey] to connect
each requirement to the code that implements it and to the test that verifies
it. The hypothesis was that agents with good specifications work more
autonomously and need less human oversight.

We no longer work that way. We do not write the specification before the code,
and we do not use the coverage of a specification to decide when a task is
complete. The specifications and the references in the code remain, but only
`check-specs` keeps them current, and that check is all that they give us now.

The check has a cost that the practice no longer pays back. The pin of Tracey
names a Git revision of a pre-release, so mise builds it from source. A build
takes about twenty minutes, and it needs a second pin for pnpm, a workflow
that keeps a cache of the binary warm, and steps in CI that keep the build out
of every job that does not need it. The build also fails on the Windows
runner, which is a platform that Rakko supports. Every change to a behavior
also needs a change to a requirement, a new version of it, and an update of
each reference.

## Decision

This repository does not trace requirements.

1. **No specifications.** A crate has no `SPECIFICATION.md`, and its code and
   its tests carry no references to requirements. The documentation of a
   crate describes what it does, and its tests show that it does it.

2. **No check of specifications.** The harness of this repository does not
   mount `check-specs`, so neither pre-commit nor CI runs it, and the
   repository pins no Tracey.

3. **The action stays.** `rakko-check-specs` remains an action that other
   projects can mount. This decision is about the process of this repository,
   not about the actions that it offers.

This supersedes [ADR-002], and the sentence of point 1 of [ADR-012] that each
action keeps its specification beside its source. The rest of [ADR-012]
stands.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Keep the Specifications Without the Check

The specifications could stay as prose, and the references could stay in the
code. But nothing would then report a reference to a requirement that changed
or that no longer exists, so the documents and the code would drift apart.
A specification that might be wrong is worse than no specification, because
a reader cannot tell which parts are still true.

### Keep the Check Outside of Pre-Commit

The check could run only in CI, so that a commit does not wait for it. This
keeps every cost of the build of Tracey, and it moves the failure from the
commit to the pull request. It does not bring the practice back.

### Pin a Released Tracey

A release of Tracey could replace the pin of a Git revision, and with a
prebuilt binary, mise would download Tracey and not build it. But the newest
release has no binary for Windows, and its build script fails there as the
build of the revision does, so Windows would stay blocked. Each change to
a behavior would also still need a change to a requirement, a new version of
it, and an update of each reference. A cheaper tool does not bring back a
practice that we stopped.

## Consequences

- The documentation and the tests of a crate are the only record of its
  behavior. This is the "Documentation Only" alternative of [ADR-002], and it
  has the cost that [ADR-002] gave for it: no tool counts the behaviors, and
  no tool reports when the documentation and the code diverge.
- The issue of a task, not the coverage of a specification, defines when the
  task is complete.
- The installation of the tools no longer builds Tracey, and CI no longer
  needs the cache of the binary or the pin of pnpm.
- This repository no longer runs `rakko-check-specs` against a real Tracey, so
  a change in Tracey that breaks the action shows up first in a project that
  mounts it.
- Tracey was the last tool that kept the tests of this repository off Windows,
  so CI can now add the platform.
- A return to requirement tracing needs a new ADR that supersedes this one.

[adr-002]: 002-specifications.md
[adr-012]: 012-repository-layout.md
[tracey]: https://tracey.bearcove.eu/
