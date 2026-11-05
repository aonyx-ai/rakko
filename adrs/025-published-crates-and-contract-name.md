# ADR-025: Published Crates and Contract Name

## Status

Accepted

Corrected on 2026-11-05: `rakko-test-utils` goes to crates.io. The first
version of this ADR kept the crate off crates.io. That version did not weigh
that Cargo then removes the crate from the manifest of each published crate
that tests with it. The tests in those packages then cannot build. That
version also did not weigh that the crate helps the authors of actions
outside Aonyx.

## Context

No crate of Rakko is on crates.io today. A harness is the small binary crate
in a project that mounts the actions of Rakko. The harness of each other
project of Aonyx depends on Rakko through a Git revision, and the harness of
this repository uses paths. Rakko prepares a public launch, and a project
outside Aonyx adopts Rakko through Cargo. So Rakko must decide which crates go
to crates.io.

[ADR-012] grouped the crates into directories, and it left this question open.
It noted one fact that limits the answer. Cargo publishes no crate whose
dependencies are absent from crates.io. So when Rakko publishes an action,
every helper crate that the action uses goes to crates.io too.

A development dependency, which only the tests of a crate use, can stay off
crates.io when the manifest gives it no version. But Cargo then removes it
from the manifest of the package that it publishes, and the tests in that
package cannot build. Some users of a published package build those tests.
Examples are crater, which tests the Rust compiler against the crates on
crates.io, the packagers of distributions, and builds that keep a copy of
each dependency in their own source tree.

The workspace of this repository holds 42 crates. A harness can depend on
four kinds of them:

- The contract crate holds the `Action` trait, the trait of a group, and the
  types that they share. A group is a list of actions that runs as one
  command, such as pre-commit. Every action and every harness depends on the
  contract crate.
- The projection, `rakko-cli`, turns the actions that a harness mounts into a
  command line.
- A helper crate holds machinery that many actions use. An example is
  `rakko-tool`, which finds an external tool through mise and runs it.
- The catalog is the actions, the bundles, and the commands. A bundle is a
  crate that exports a list of actions, such as `rakko-baseline`. A command
  is a crate that a harness mounts beside its actions, such as
  `rakko-set-msrv`. A project picks the crates of the catalog for what they
  do.

A harness does not depend on two more crates. The first is
`rakko-test-utils` in the workspace, which holds helpers for the tests of an
action. The actions and some helper crates of this repository use it as a
development dependency. The helper `run_checking_access` of
`rakko-test-utils` runs an action and fails the test when a run that declares
a read changes the project. It is the only check that an action keeps its
declaration of access, so the author of an action outside Aonyx can use it
too.

The contract crate does not use `rakko-test-utils`, because
`rakko-test-utils` depends on the contract crate. A development dependency in
the other direction makes a cycle, and Cargo then cannot publish either crate
first.

The second crate is the harness of this repository, outside the workspace in
`tools/rakko`. A harness is a binary, and no crate depends on it.

The contract crate is `rakko-action` today. The `rakko` crate is a placeholder
that holds only an example function. [ADR-004] calls the contract crate
`rakko`, so the code and the ADR do not agree. Since [ADR-024], the contract
crate also holds the trait of a group. So the name `rakko-action` describes
only a part of what the crate holds.

A name on crates.io is almost permanent. crates.io deletes a crate only in a
small number of cases, so a crate that Rakko publishes stays there. On
2026-11-05, all 42 names of the workspace were free on crates.io.

## Decision

Rakko publishes to crates.io every crate that a harness can depend on, and
`rakko-test-utils`, the helpers for the tests of an action. The contract crate
is `rakko`.

1. **Every crate that a harness can depend on goes to crates.io.** These are
   the contract crate, the projection, the helper crates, the actions, the
   bundles, and the commands.

2. **Only the harnesses stay off crates.io.** A harness is a binary of one
   project, and no other crate depends on it. `rakko-action` does not go to
   crates.io either, but point 4 removes that crate before the first
   release. `rakko-test-utils` goes to crates.io with the other crates, and
   it becomes one of the crates whose API the launch freezes. Each published
   crate that tests with it then keeps it in its manifest, so the tests in
   the published package build. The author of an action outside Aonyx can
   also use it to check that the action keeps its declaration of access.

3. **Publication is not a promise of support.** A crate on crates.io can look
   like a product that Aonyx supports. Aonyx commits to Rakko as a tool, but
   not to the support of each action forever. The catalog goes to crates.io
   to protect its names and so that a project can adopt it. What Rakko
   promises about the changes between two versions of a crate is a separate
   decision, the [stability promise]. It covers breaks, and not how long
   Aonyx maintains an action. This ADR does not record it.

4. **The contract crate is `rakko`.** The crate holds the traits of actions
   and of groups, so the name `rakko-action` does not show all that the crate
   holds. The traits and their types move into `rakko`, and `rakko-action`
   never goes to crates.io. ADR-004 already calls the contract crate `rakko`,
   so ADR-004 does not change, and the code then agrees with it. A search for
   Rakko on crates.io finds the contract crate first, but the author of a
   harness needs the projection. So the documentation of `rakko` links that
   author to `rakko-cli`.

The first reason for this decision is the supply chain. If an action is on
GitHub but not on crates.io, anyone can publish a crate with the name of that
action. That crate can hold malicious code. A user who adds it to a harness
runs that code when the harness builds and when it runs. The first release on
crates.io claims the name of each crate that a harness can depend on.

The second reason is adoption. The launch aims for a project that adopts Rakko
from crates.io, and the catalog is what a project adopts. The third reason is
the cost of the alternative that kept the catalog in Git. It needed a fixed
release order for the contract crate and a versioning rule in Renovate, as the
first alternative describes.

ADR-012 does not change. It left open which crates Rakko publishes, and all of
its points still apply.

The decision records the direction, and it stops there. The steps that move
the traits from `rakko-action` into `rakko`, and the settings in the
manifests, are implementation choices. So are the automation of the releases
and the metadata of each crate, such as its description.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### The Catalog in Git

The crates in `crates/` go to crates.io, and the catalog stays in Git. A
project then depends on an action through a Git tag, such as
`rakko-format-json@0.1.0`. Jan David leaned toward this split on 2026-10-11.
A crate on crates.io can look official, and the split showed that Aonyx does
not support each action forever.

But the names of the actions stay free on crates.io, so anyone can publish
malicious code under them. A project adopts the catalog, so the catalog is
what must be on crates.io.

Two pins at different Git tags make Cargo build two copies of a crate that
they share, such as the contract crate. So each action in this repository must
take the contract crate from crates.io, and not from its path. A change to the
contract crate then must be on crates.io before an action that uses it gets a
tag. Renovate also reads the versions on crates.io with no setup, while a Git
tag such as `rakko-format-json@0.1.0` needs a versioning rule that nobody
tested. Point 3 of the decision states that publication is not a promise of
support, and the split had to show that.

### `rakko-action` as the Name

The contract crate keeps its current name. No crate moves, and no action or
harness breaks.

But the crate holds the traits of actions and of groups, so the name describes
only a part of it. The code also keeps its disagreement with ADR-004. And the
`rakko` crate goes to crates.io as a placeholder, under the name that a search
for Rakko finds first.

### `rakko-core` as the Name

The glossary retired the term core crate, because four names were in use for
the contract crate. A crate with this name brings the retired term back.

## Consequences

- The first release on crates.io claims the name of every crate that a
  harness can depend on, and the name `rakko-test-utils`. Nobody else can
  publish a crate under the name of an action that Rakko published. A crate
  that Rakko adds later claims its name with its own first release.
- The name `rakko-action` stays free on crates.io. This repository and the
  harnesses of Aonyx use it today. So the first reason of the decision does
  not protect this name.
- A project adopts all of Rakko from crates.io. Renovate updates each crate
  that the manifest of the harness lists, as it updates any other
  dependency. A crate that a bundle brings in is a separate question. For
  example, `rakko-baseline` depends on `rakko-format-json`. A harness that
  mounts `rakko-baseline` gets a new release of `rakko-format-json` only
  when `rakko-baseline` releases, or when Renovate updates the lock file.
  This case is an open question of its own, [bundle releases].
- A crate of the catalog stays on crates.io, also when Aonyx no longer
  maintains it. Point 3 says that publication is no promise of support,
  but a user reads the documents of the launch and not this ADR. Those
  documents must say it.
- The move from `rakko-action` to `rakko` breaks every action, and every
  harness that imports the contract crate. An example is a harness that
  mounts a group, such as the harness of this repository. The move lands
  before the first release on crates.io, so that `rakko-action` never goes
  there.
- The crate that a search for Rakko finds first is the contract crate, and
  not the crate that the author of a harness needs. Its documentation must
  link that author to `rakko-cli`.
- Every helper crate and `rakko-test-utils` become public crates. Each
  change to one of them is then a change to a crate that others can depend
  on.

[adr-004]: 004-actions-as-libraries.md
[adr-012]: 012-repository-layout.md
[adr-024]: 024-group-runs.md
[bundle releases]: https://github.com/aonyx-ai/rakko/issues/508
[stability promise]: https://github.com/aonyx-ai/rakko/issues/492
