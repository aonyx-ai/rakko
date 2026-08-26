# ADR-001: Architecture Decision Records

## Status

Accepted

## Context

Rakko is a long-term project to turn the maintenance of our projects into
versioned Rust crates. It is a set of crates. Projects can adopt each action
on its own or in bundles together with others. During the development of
these crates, we make many decisions about architecture, infrastructure, and
design.

We want to record these decisions for two reasons. First, a written record
helps the team to discuss a decision and to find the best possible design.
Second, the record gives context to humans and to agents. We think that this
context becomes more important as coding agents do more of the work.

Decisions can change over time, and we must record these changes too.
Sometimes we take one path, learn important lessons, and then reverse the
decision. We want to record these lessons so that we do not make the same
mistake twice.

## Decision

We adopt [Architecture Decision Records][adr] (ADRs) to document decisions in
this project. An ADR is a lightweight way to record the context of a decision
and the decision itself. The team discusses each ADR in a normal pull request.

1. **One decision per ADR.** Each ADR records one decision. We divide a large
   direction into independent ADRs instead of one document that settles
   everything at once. The team can review, accept, and implement each ADR on
   its own.

2. **The structure is fixed.** Every ADR has five sections: Status, Context,
   Decision, Alternatives, and Consequences. Context describes the situation
   that makes the decision necessary. Decision states the choice plainly.
   Alternatives lists the other options and why we rejected them. Consequences
   lists both the benefits and the costs of the decision. Every new ADR starts
   from `000-template.md`.

3. **ADRs need founder consensus.** Both founders review each ADR. Both must
   agree with the direction of an ADR before they accept it.

4. **The lifecycle is explicit.** An ADR has one of four states: Accepted,
   Rejected, Superseded, or Deprecated. There is no "Proposed" state. The
   author opens a pull request with the ADR already marked Accepted. The pull
   request is the proposal, and the merge is the decision. When a newer ADR
   replaces an Accepted ADR, the old ADR becomes Superseded. When an ADR no
   longer applies and no newer ADR replaces it, the ADR becomes Deprecated.

   If the founders decide against an ADR, the author changes its status to
   Rejected and merges it anyway. We keep rejected ADRs and do not delete them.
   The reasons against a path are as valuable as the reasons for one. Without
   this record, someone can reopen the same question later, and nobody knows
   why we closed it.

5. **Accepted ADRs are immutable.** When a decision changes, we do not edit
   the ADR in place. We write a new ADR that supersedes the old one, and we
   mark the old one "Superseded by ADR-NNN." This chain preserves how our
   thinking evolved.

6. **Numbers are permanent.** ADRs have sequential numbers, and we never reuse
   a number. They live in `adrs/` as `NNN-kebab-case-title.md`. When the
   author opens the pull request, they claim the number. If two open pull
   requests claim the same number, the second one to merge gets a new number
   before the merge. A number stays with its ADR even after a newer ADR
   supersedes it.

Not every choice needs an ADR. The bar is significance. When a decision is
architectural, cross-cutting, hard to reverse, or otherwise expensive to get
wrong, it needs an ADR. Routine implementation choices do not need one. ADRs
for routine choices only dilute the ADRs that matter.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Spec-Driven Development

In spec-driven development, a team writes specifications that describe the
target state of a system. Specifications document _what_ must happen, but not
_why_. We can still adopt specifications to define requirements. However, they
are not a good tool to document architecture and infrastructure decisions and
how these decisions evolve.

### Request for Comments

Requests for Comments (RFCs) are similar to ADRs, but they usually include a
formal comment period. A comment period is too much ceremony for a small team
that wants to move fast. When the team grows and coordination becomes more
difficult, we can reconsider RFCs. At that point, a comment period gives each
team time to discuss the decisions that affect it.

### Wiki or Code Documentation

We can also document the architecture and infrastructure in a wiki or in the
code. But both places overwrite previous decisions. Then it is more difficult
to see a linear history of how our thinking evolved. Both places also spread
important decisions over a wider surface, and it is easier to miss them.

## Consequences

- The collection of ADRs becomes the official record of our reasoning. It is
  the first thing that a new contributor reads to understand what Rakko is
  and why it has this shape.
- An ADR adds friction, and this friction is deliberate. When we state a
  decision and its alternatives in prose, disagreement and half-formed
  reasoning become visible before they become code.
- Immutability produces chains of superseded ADRs instead of one current
  truth. A reader learns from this history why we took a path and later left
  it.
- Rejected ADRs accumulate next to accepted ones. This accumulation is
  intended, because the record of the paths that we did not take is part of
  the asset.
- Consensus becomes explicit and reviewable. The cost is that the founders
  must resolve disagreement about direction before they accept an ADR. They
  cannot defer it.
- The decision about what is "significant" enough to record is a judgment
  call, and it will sometimes be wrong. A missed ADR is better than meaningful
  ADRs buried under routine ones.

[adr]: https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions
