# ADR-027: Specification Check

## Status

Accepted

## Context

[ADR-016] stopped requirement tracing in this repository. The harness no
longer mounts `check-specs`, and the repository pins no Tracey. But the action
`rakko-check-specs` stayed in the catalog, so that other projects could mount
it.

Kawauso was the only project that mounted the action. On 2026-11-10, it
removed its specifications, the references in its code, and its pin of
Tracey, for the reasons that ADR-016 gives. Clawless still holds a
configuration of Tracey and references in its code, but its harness does not
mount the action, so nothing checks them. No project mounts the action now.

The action still has a cost. A change to the contract or to the shared
machinery also changes the action. Seven pull requests changed it after
ADR-016, with about 1,300 lines in total. Nothing in this repository runs the
action against a real Tracey, so its tests cannot show that it still works
after such a change. This harness cannot mount the action either. Tracey is
a pre-release that mise builds from a Git revision, and the pin of the action
cannot carry the option that the cargo backend of mise needs to build it.

[ADR-025] publishes every action to crates.io at the launch, and [ADR-026]
freezes the part of each action that a harness uses. Without a change, the
launch publishes and freezes an action that no project runs and that no test
runs against its tool.

## Decision

Rakko does not trace requirements, and its catalog has no action that checks
them.

1. **No specifications.** A crate has no `SPECIFICATION.md`, and its code and
   its tests carry no references to requirements. The documentation of a
   crate describes what it does, and its tests show that it does it.

2. **No check of specifications here.** The harness of this repository does
   not mount an action that checks specifications, so neither pre-commit nor
   CI runs one, and the repository pins no Tracey.

3. **No action for specifications.** Rakko removes `rakko-check-specs` before
   the first release. The catalog offers no action that wraps Tracey or
   another tool that traces requirements. A project that traces requirements
   runs its tool outside of Rakko, or writes an action in its own harness.

This supersedes [ADR-016] as a whole. Points 1 and 2 restate its decision for
this repository, and point 3 replaces its point 3. ADR-016 superseded the
sentence of point 1 of [ADR-012] that each action keeps its specification
beside its source. That sentence stays superseded, and the rest of ADR-012
stands.

## Alternatives

We considered these alternatives and rejected them for the reasons below. The
alternatives that ADR-016 rejected for this repository stay rejected for the
reasons that it gives.

### Keep the Action Without a User

This was the decision of ADR-016. Point 3 of ADR-025 says that publication is
not a promise of support. But every change to the contract still has to keep
the action building and its tests passing, and those tests do not run Tracey.
By the time a project mounts the action, it is likely to fail, and nobody can
see that before then.

### Publish the Action, Then Deprecate It

The first release could publish the action and mark it as deprecated. This
reserves its name on crates.io, which is the reason that ADR-025 gives for
the publication of every action. But the action would still have to follow
every change to the contract until the release. A new crate that is
deprecated in its first release also offers nothing to a user. After the
removal, no documentation of Rakko refers to the name, so few users would look
for a crate with that name.

## Consequences

- The documentation and the tests of a crate are the only record of its
  behavior. No tool counts the behaviors, and no tool reports when the
  documentation and the code diverge. ADR-016 gave this cost, and it stays.
- The issue of a task, not the coverage of a specification, defines when the
  task is complete.
- A project cannot check its specifications with an action of Rakko. The
  project runs its tool outside the harness, or it writes an action of its
  own against the contract.
- A change to the contract or to the shared machinery no longer has to update
  the action, and the catalog has one action fewer to publish, freeze, and
  maintain.
- The name `rakko-check-specs` stays free on crates.io, so someone else can
  publish a crate with that name. The risk is small, because no project
  mounts the action and nothing refers to the name.
- A return to requirement tracing, in this repository or in the catalog, needs
  a new ADR that supersedes this one. The Git history holds the action as a
  starting point.

[adr-012]: 012-repository-layout.md
[adr-016]: 016-requirement-tracing.md
[adr-025]: 025-published-crates-and-contract-name.md
[adr-026]: 026-harness-api-stability.md
