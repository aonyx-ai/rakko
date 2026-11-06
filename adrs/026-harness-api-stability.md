# ADR-026: Harness API Stability

## Status

Accepted

## Context

A harness is the small binary crate in a project that mounts the actions of
Rakko. Rakko prepares a public launch, which is its first release on
crates.io. After the launch, a project updates Rakko through its harness,
usually with a pull request from Renovate. When a release changes an item that
the harness uses, each update needs a fix by hand. For example, a change to
how a harness mounts pre-commit broke 10 of the 10 open Rakko updates in the
fleet.

[ADR-025] decided that every crate that a harness can depend on goes to
crates.io. These crates are the contract crate, the projection, the helper
crates, and the catalog. The catalog is the actions, the bundles, and the
commands that a project picks for what they do.

[ADR-012] grouped the crates into directories, and it stated no stability
guarantee on purpose. Point 3 of ADR-025 left the promise about the changes
between two versions to a separate decision. This ADR records that decision.
The decision ticket [Which Parts of the Harness API the Launch
Freezes][freeze ticket] settled it on 2026-11-02.

Semantic versioning (semver) is the rule that Cargo applies to the version of
a crate. A release that removes or changes an item that users can depend on
is a break, and it needs a new major version. Before 1.0, Cargo treats a
change of the first number after the zero, such as from 0.1 to 0.2, as a
break. No crate of Rakko is on crates.io yet.

A proposal on record made the `rakko` crate a facade. A facade is a crate
that re-exports other crates, so that a harness depends on one crate only.
Two facts work against it:

- The projection, `rakko-cli`, depends on the contract crate, `rakko`. If
  the contract crate re-exports the projection, Cargo finds a cycle between
  the two packages. Cargo rejects this cycle even when the feature that
  enables the re-export is off.
- A facade over the catalog must release when any crate under it releases.
  So a break in one action reaches every project that depends on the facade.

The fleet used Rakko for half a year before the launch. Most of the work that
Aonyx plans after the launch adds items and changes none. But Jan David
remembers a planned feature that breaks the API, and he cannot yet say which
one.

## Decision

The harness API is everything that the code of a harness and the workflows of
a project depend on. The launch ships it as 0.1. After the launch, the
harness API takes only additions, and a replaced item stays until a breaking
release removes it. Every crate goes to 1.0 at the same time, after four
weeks without a break.

1. **The harness API has four parts.** These are the parts:

   - The builder of the projection: the function that creates it, the
     methods that mount, and the method that runs.
   - The items of the catalog that a harness uses. These are the function of
     each bundle that gives its actions, the type of each action, and the
     constructor of each command. The part also includes the types that these
     items expose. Examples are the argument and error types of an action,
     and the types that the constructor of a command takes.
   - The crates for a harness that writes its own actions or groups. These
     are the contract crate, the helper crates, and `rakko-test-utils`.
     ADR-025 adds `rakko-test-utils` to the crates whose API the launch
     freezes.
   - The command line that the workflows of a project call: the names of the
     commands, their flags, and their exit codes.

   The document that a command writes with `--json` is not part of the
   harness API. [ADR-008] gives its schema a version of its own, and each
   document says `"schema": "unstable"`. This version promises nothing. A
   stable schema later is an addition.

   The Renovate preset of Rakko and the GitHub Action that fixes the pull
   request of an update are not among the four parts. [ADR-023] records what
   they promise, and this ADR does not change that promise.

2. **The launch ships 0.1, and 1.0 comes after four weeks without a break.**
   Every harness of the fleet runs the launch release for four weeks. If no
   change in that time needs a break of the harness API, the gate for 1.0
   opens. A change that needs a break ships as the next 0.x release, and the
   four weeks start again. An addition does not stop the four weeks. A
   deprecation does not stop them either, because the deprecated item stays
   and the harness still builds.

   The contract crate, the projection, the catalog, the helper crates, and
   `rakko-test-utils` go to 1.0 at the same time, so a user learns one rule.
   The 1.0 releases remove every item that 0.x deprecated, because Cargo
   treats the step from 0.x to 1.0 as a break anyway.

3. **After the launch, the harness API takes only additions.** When a new
   item replaces an old one, the harness API keeps the old item and marks it
   as deprecated. The old item stays until a breaking release removes it. The
   notes of that release give the steps of the migration. This rule applies
   to the command line as well as to the Rust items.

4. **Semver covers the Rust items and the command line.** New findings alone
   are not a break. For example, an action that finds a problem that it
   missed before does not break the harness API. A new version of a tool can
   be a break, for example when a project must change its configuration to
   keep the behavior that it had. This ADR sets no rule for these cases. Each
   case gets a decision when it occurs.

5. **Rakko has no facade.** The machinery is two crates, and neither crate
   re-exports the other. The contract crate serves the authors of actions and
   groups. The projection turns what a harness mounts into a command line. A
   later projection, such as a terminal user interface, gets a crate of its
   own.

   The two facts in the Context section are the reasons against a facade.
   The contract crate cannot re-export the projection, and a facade over the
   catalog spreads a break in one action to every project. Once the
   projection erases what a harness mounts, a harness that only mounts needs
   no contract crate. So a separate facade over the contract crate and the
   projection saves nothing.

ADR-008 and ADR-012 do not change. ADR-012 left the promise open, and
ADR-008 already keeps the schema of `--json` apart from the harness API. All
of their points still apply.

The decision records the direction, and it stops there. The names and the
signatures of the methods stay open, and so does the shape of the mount. How
a command or a flag shows that it is deprecated also stays open. The list of
the items that each crate exports, the automation of the releases, and the
text of the release notes are implementation choices too.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### A Launch at 1.0

The launch ships 1.0, and semver then marks each break with a new major
version. Jan David argued for this option. The fleet used Rakko for half a
year, and the work that Aonyx plans after the launch adds items. A version
below 1.0 can also make a user think that the tool is untested.

He chose 0.1, and the launch documents state the conditions for 1.0. He
remembers a planned feature that breaks the API, and 0.1 leaves room for
that break. The launch documents must say that 0.1 marks a young API, and
not a young tool.

### A Facade Crate

The `rakko` crate, or a separate crate, re-exports the contract crate, the
projection, the catalog, or some of them. A harness then depends on that one
crate. A user has one version to read and one crate to update.

The contract crate cannot re-export the projection, because Cargo rejects the
cycle between the two packages. A facade over the catalog releases whenever
an action under it releases. So a break in one action reaches every project,
also a project that does not mount that action. A separate facade over the
contract crate and the projection has neither problem, but it saves nothing.
Once the projection erases what a harness mounts, a harness that only mounts
needs no contract crate.

### A New Tool Version Is Never a Break

Semver covers only the Rust items and the command line, and a new version of
a tool never counts as a break. The rule is easy to apply.

Jan David rejected it. When a project must change its configuration to keep
the behavior that it had, the update breaks that project, whatever the Rust
items do.

## Consequences

- A project can take a 0.x release that only adds items with no fix by hand.
  A replaced Rust item shows a warning of the compiler, and the release notes
  give the migration, before a breaking release removes the item.
- In a project that mounts lint-rust, each warning of the compiler fails the
  run. So when an item becomes deprecated, the update pull request of that
  project fails. The harness still builds, so every command of the harness
  keeps working.
- Rakko keeps each deprecated item until the next breaking release. This
  costs code and tests that no new harness needs.
- Each break before 1.0 starts the four weeks again, so nobody knows the date
  of 1.0 in advance. A crate that did not change also goes to 1.0, because
  all crates go to 1.0 at the same time.
- Each item that a crate of the catalog exports becomes part of the harness
  API. So each crate of the catalog must export only what a harness, or its
  own implementation of a trait, needs. Any other item freezes with no user.
- Today a harness writes erased types when it mounts. Once the projection
  erases what a harness mounts, a harness that only mounts depends on the
  projection and on the catalog. A harness that writes its own actions or
  groups also depends on the contract crate, as every author of an action or
  a group does.
- The document of `--json` can change in any release. A program that reads it
  can break on an update until a stable schema arrives.
- A new version of a tool has no rule. A user cannot know in advance whether
  an update of a tool counts as a break, until Rakko decides that case.
- The README and the announcement of the launch must state the gate for 1.0,
  and that 0.1 marks a young API, and not a young tool.

[adr-008]: 008-renderers.md
[adr-012]: 012-repository-layout.md
[adr-023]: 023-rakko-updates.md
[adr-025]: 025-published-crates-and-contract-name.md
[freeze ticket]: https://github.com/aonyx-ai/rakko/issues/453
