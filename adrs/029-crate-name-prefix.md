# ADR-029: Crate Name Prefix

## Status

Accepted

## Context

Rakko prepares a public launch, which is its first release on crates.io.
[ADR-025] decided that every crate that a harness can depend on goes to
crates.io. The workspace holds 42 crates. One crate is `rakko`, and the name
of each other crate starts with `rakko-`.

A name on crates.io is almost permanent, and crates.io has no namespaces. The
first author who publishes a name owns it. So the launch is the last moment
at which a new name costs only work in this repository and in the fleet.

The crates do not all serve the same reader. The section Support of the
README records the decision of the ticket [What Aonyx Promises to Maintain at
the Launch][support ticket]:

- Aonyx supports four crates for the authors of other actions. These crates
  are `rakko`, `rakko-cli`, `rakko-tool`, and `rakko-test-utils`.
- The catalog is the actions, the bundles, and the commands. Aonyx built the
  catalog for its own workflows, and keeps it focused on them. The README
  encourages an author with other needs to publish an action of their own.
- Five helper crates hold code that Aonyx built for its own actions. These
  crates are `rakko-cargo`, `rakko-nextest`, `rakko-prettier`, `rakko-taplo`,
  and `rakko-worktree`. The README of each one says so.

The names do not show this difference. `rakko-lint-rust` and `rakko-tool`
look like two parts of one product. The name of a bundle misleads more.
`rakko-rust` reads as the Rust support of Rakko, but it is the set of Rust
actions that Aonyx uses. [ADR-012] observed that a consumer reads a crate
name and never sees this repository. The README of a crate states whom the
crate serves. But a reader sees the name first, in a manifest or in a search
result.

The short names also take the obvious name for each task. An author who lints
Rust in a different way cannot publish `rakko-lint-rust`. Rakko also has no
answer for two publishers who want the same name.

The name of a crate is not the name of a command. An action declares the name
of its command, such as `lint-rust`, in its code. A harness writes the name
of a crate only in its manifest and in its imports.

On 2026-11-12, no crate on crates.io had a name that starts with
`aonyx-rakko`. Aonyx owns the crates `aonyx` and `aonyx-cli` there. An
unrelated project, Aonyx Agent, publishes crates whose names also start with
`aonyx-`, such as `aonyx-core` and `aonyx-agent`.

## Decision

A crate that Aonyx built for its own workflows carries the prefix
`aonyx-rakko-`. A crate that Aonyx supports for other authors keeps the name
`rakko` or the prefix `rakko-`.

1. **The prefix follows the support.** The four crates that Aonyx supports
   for other authors keep their names. `rakko-macros` keeps its name too. It
   holds the derive macros of the contract, and `rakko` exports them again.
   So it is a part of the contract crate.

2. **The catalog takes the prefix.** Each action, each bundle, and each
   command becomes `aonyx-rakko-<name>`. For example, `rakko-lint-rust`
   becomes `aonyx-rakko-lint-rust`, and `rakko-rust` becomes
   `aonyx-rakko-rust`.

3. **The five helper crates of Aonyx take the prefix too.** They hold the
   tool choices of Aonyx, and Aonyx does not support them for other authors.
   A name with only `rakko-` puts them beside the supported crates, and the
   README of each one then says the opposite.

4. **Only the name of the crate changes.** The names of the commands, of
   their flags, and of the types stay. A project that runs `rakko lint-rust`
   sees no difference.

5. **The old names never go to crates.io.** The new names land before the
   first release.

6. **A new crate gets its name by the same rule.** It takes `rakko-` alone
   only when Aonyx supports it for other authors.

ADR-025 does not change. Every crate that a harness can depend on still goes
to crates.io, and its first release still claims its name. [ADR-026] does not
change either. Its promise about breaks covers a crate with either prefix.
The prefix shows whose workflows a crate serves, and not how its versions
change. ADR-012 does not change. Its directories still group the crates by
who picks them.

The decision records the direction, and it stops there. The names of the
directories in this repository stay open. So do the steps of the change, and
how the harnesses of the fleet move to the new names. This ADR also sets no
rule for the names that other authors give to their crates.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### The Names Stay

Every crate keeps its name. The README of the repository and the README of
each crate already state whom the crate serves. No file changes, and the
short names stay claimed on crates.io.

But a README reaches a reader later than a name does, and the names of the
bundles mislead. After the launch, a name cannot change. A new name is then a
new crate, and the old crate stays on crates.io.

### The Prefix for the Catalog Only

The catalog takes the prefix, and the five helper crates keep `rakko-`. A
project picks no helper crate for what it does, so the name of a helper
crate reaches fewer readers.

Jan David chose the prefix for the helper crates too. They hold the tool
choices of Aonyx, and they are not among the crates that Aonyx supports for
other authors. Without the prefix on them, `rakko-` has no clear meaning.

## Consequences

- A reader on crates.io learns from the name whom a crate serves. A crate of
  Aonyx with only `rakko` in its name is a crate that Aonyx supports for
  other authors.
- 37 crates get a new name: the 32 crates of the catalog and the five helper
  crates. Five crates keep their names. The change reaches each manifest,
  each import, each README, each link to docs.rs, and the examples of the
  website.
- Each harness of the fleet changes its manifest and its imports once.
- The lines in a harness get longer, for example
  `aonyx_rakko_lint_github_actions::LintGitHubActions`. A harness that mounts
  a bundle writes few of these lines.
- The short names stay free on crates.io. Anyone can publish
  `rakko-lint-rust`, and that crate can look more official than the crate of
  Aonyx. ADR-025 accepted the same for `rakko-action`. A user who copies a
  name from the documents of Rakko gets the crate of Aonyx. A user who
  guesses a name does not.
- The prefix does not prove that Aonyx published a crate, because an
  unrelated project also uses `aonyx-`. The owner on crates.io and the link
  to the repository prove it.
- An author outside Aonyx can publish an action with a short name. That
  author can also copy the pattern of a publisher, then `rakko`, then the
  name.
- The prefix does not change with the support. When Aonyx starts to support
  a helper crate for other authors, the crate keeps its prefix. A name with
  only `rakko-` then needs a new crate.
- The documents of the launch must explain the prefix. A user reads those
  documents and not this ADR.

[adr-012]: 012-repository-layout.md
[adr-025]: 025-published-crates-and-contract-name.md
[adr-026]: 026-harness-api-stability.md
[support ticket]: https://github.com/aonyx-ai/rakko/issues/536
