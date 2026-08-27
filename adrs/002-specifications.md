# ADR-002: Specifications

## Status

Accepted

## Context

[ADR-001][adr-001] records that we use Architecture Decision Records (ADRs) to
document the context and the reasons for a decision, the "why". But when we
implement features, we also need to specify the requirements, the "what". The
requirements need a different kind of document.

A specification is especially important because coding agents implement the
features. The agents get the context from the ADRs, but they need a
specification to guide their implementation and to keep it in scope. A
specification that a tool can compare with the code also shows which
requirements are implemented and which are not. Our hypothesis is that agents
with good specifications work more autonomously and need less human oversight.

## Decision

We write a specification for each crate. We use [Tracey][tracey] to connect
its requirements to the code and to the tests.

1. **One specification per crate.** Each crate has a specification in
   `crates/<crate>/SPECIFICATION.md`. The specification documents what the
   crate does. It does not name types or functions, because the code and its
   documentation do that.

2. **Requirements have identifiers.** A specification is a list of
   requirements. Each requirement states one behavior with the key word MUST,
   as [RFC 2119][rfc-2119] defines it, so that one test can show the behavior.
   Each requirement has an identifier such as `rakko[placeholder.add]`. The
   prefix is the name of the crate without `rakko-`.

3. **The code has references to the requirements.** A comment such as
   `// rakko[impl placeholder.add]` marks the code that implements a
   requirement. A comment such as `// rakko[verify placeholder.add]` marks
   the test for the requirement. Tracey reads the specifications and the
   references. Then it reports the requirements that have no implementation
   or no test. `.config/tracey/config.styx` lists the specifications.

4. **Requirements have versions.** When the text of a requirement changes,
   its version increases, for example to `rakko[placeholder.add+2]`. A
   reference to the old version is stale. It stays stale until the author
   reviews the code and updates the reference. `tracey bump` increases the
   versions of the staged requirements with a changed text.

5. **The check is part of pre-commit and CI.** The `check-specs` recipe
   fails on a broken or stale reference and on a malformed or duplicate
   identifier. It also fails on a staged change to the text of a requirement
   without a new version. Coverage is information, not a gate. The recipe
   shows the requirements that have no implementation or no test, and a
   specification can merge before its implementation.

6. **The specification defines completion.** When each requirement of the
   crate has a reference from the code and a reference from a test, an
   implementation task is complete.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Documentation Only

Rustdoc and README files document the API of a crate. They can describe
requirements in prose, but they have no unit that a tool can count. As a
result, nobody knows which requirements the code implements and which
requirements have a test. The documentation and the code can also diverge over
time, and no tool reports the difference.

### One Specification for the Workspace

One specification for all crates is one document to maintain. But the crates
are independent, and projects can adopt each crate on its own. The
specification of a crate is its contract, so it stays in the directory of the
crate. Tracey also reports coverage per specification, so one specification
per crate gives coverage per crate.

### Coverage as a Merge Gate

The check can also fail for a requirement without an implementation or
without a test. Then `main` always has an implementation for every
requirement. But then a specification cannot merge before its implementation,
and a specification is easier to review on its own. We keep coverage as
information for now. A check that fails only on a decrease in coverage is
possible later.

### Other Formats and Tools

Tracey also reads StrictDoc and Typst files. Markdown is the format that we
already lint and that everybody reads, and a specification can change its
format later. Dedicated tools for requirements management exist for regulated
industries. They are too much process for a small team.

## Consequences

- Every behavior of a crate has a requirement, and a change in behavior starts
  with a change to the requirement. Reviewers read the requirement before the
  code, and a coding agent implements against it.
- The code carries references as comments. Humans and agents must add them and
  keep them current, and `check-specs` fails until they do.
- Requirements without an implementation can exist on `main`. Each check and
  the Tracey dashboard show them, but a removed reference does not fail the
  check. A check that fails only on a decrease in coverage can correct this
  later.
- A change to the text of a requirement forces a review of the code that has
  a reference to it. The reference is stale until the author updates it.
- We depend on a pre-release of Tracey that we build from source at a pinned
  revision. Its configuration format can still change, and a change to the
  revision is a deliberate task.

[adr-001]: 001-adrs.md
[rfc-2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
