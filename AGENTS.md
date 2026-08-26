# Rakko

Rakko turns project maintenance into versioned Rust crates. It is an internal
tool built by Aonyx to replace their collection of Justfiles.

## Engineering

Architecture Decision Records (ADRs) in `adrs/` document why the project is
shaped the way it is. [ADR-001][adr-001] defines the process.

- Before you start a task, read the ADRs that relate to it.
- If your task conflicts with an ADR, stop and report the conflict. Do not work
  around the ADR.
- Do not edit an accepted ADR, except to change its status.
- If a task needs a decision that is architectural, cross-cutting, or hard to
  reverse, draft an ADR before you implement the task. Start from
  `adrs/000-template.md`, give it the next free number, and open it in its own
  pull request. A new ADR can also supersede an existing one.

Each crate has a specification in `crates/<crate>/SPECIFICATION.md` that
documents what the crate does. A specification is a list of requirements with
identifiers, such as `rakko[placeholder.add]`. The prefix is the name of the
crate without `rakko-`. [Tracey] links each requirement to the code that
implements it and to the test that verifies it. `just check-specs` validates
these links, and `.config/tracey/config.styx` lists the specifications.
[ADR-002] records this decision.

- Before you implement a crate, read its specification.
- Put a comment such as `// rakko[impl placeholder.add]` above the code that
  implements a requirement, and `// rakko[verify placeholder.add]` above the
  test that verifies it.
- Do not change the text of a requirement unless the task asks for it. If you
  change it, stage the file and run `tracey bump`, so that the requirement
  gets a new version.
- An implementation task is complete when `tracey query uncovered` and
  `tracey query untested` list no requirements for the crate.
- Use `tracey query rule <id>` to read one requirement, and
  `tracey query status` for an overview of the coverage.

## Language

- Use American English spelling, e.g. "color" not "colour".

## Markdown

- Use title case in headings and titles.
- Always use the Oxford comma.
- Use reference-style Markdown links, not inline links.
- Table cells must be single-line. Markdown does not support multi-line cells;
  each newline starts a new row. Ignore line length limits for table rows.

## Rust

### Dependencies

- All versions managed in root `Cargo.toml`, crates import from workspace.
- Require the lowest version of a dependency that still compiles, so that
  applications keep the widest choice of versions. Verify the floor with
  `just check-minimal-deps`.
- Write dependency entries without comments. Do not describe what a package
  does, and do not explain a version requirement. Reasoning that matters, such
  as why a floor cannot go lower, belongs in the commit message.
- When adding dependencies, run `just check-dependencies` to verify license
  compatibility. If new licenses need allowlisting in `deny.toml`, include
  that in the same commit, again without a comment. Allowlist licenses that
  are OSI- or FSF-approved, ask for any other licenses.

### Derives

- Standard trait order: Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash,
  Debug, Default
- Third-party derives: alphabetical by crate, then by macro.
- First list traits from the standard library, then from external crates.

### Documentation

- Documentation should explain the "why", not just the "what".
  - **Types**: Explain design decisions, invariants, and relationships to other
    types.
  - **Functions**: Document side effects, caller considerations, and non-obvious
    behavior.
  - **Modules**: Explain the module's role in the system and key concepts.
- Write documentation for a reader that has no prior context, and especially no
  knowledge of the conversation that led to the creation of the code.
- Write for a consumer of the published crate. A published crate bundles
  neither the ADRs nor the specifications, so never reference them from a doc
  comment. Internal rationale, such as which library a function hides, stays
  out of the documentation as well. Document what the API does, what it
  requires of the caller, and how it fails.
- Write function/method docs in third-person singular
  ("Returns the..." not "Return the...").
- Do not add a trailing period on the title (i.e. the first line).
- Use reference-style links in doc comments, not inline path links. Paths like
  `super::` and `crate::` should not appear in rendered documentation.
- Use the `/simple-english::simple-english` skill to adhere to the ASD-STE100
  standard for Simplified Technical English.

### Modules

- One public type per module, use submodules for related types.
- Use `mod.rs` for modules that contain submodules.
- Prefer `pub` over `pub(crate)`. Visibility should come from module
  structure, not access modifiers. If a type needs restricted visibility,
  that is usually a signal to restructure the modules.

### Tests

- Use blank lines to separate Arrange/Act/Assert phases.
- Test functions ordered alphabetically within modules.
- Name tests descriptively: `function_name_<condition>_<result>`, e.g.
  `greet_with_name_returns_greeting`.
- Do not test compiler-derived traits (Eq, Ord, Hash, Clone, etc.). Only test
  auto traits (Send, Sync, Unpin) and custom behavior like builder round-trips.
- Each test should have exactly one assertion.

### Errors

- Define one error enum per fallible action, named after the action and its
  object (e.g. `LoadWorkflowError`, `DiscoverProjectError`), never after the
  component that raises it.
- Use struct variants. The underlying cause is a field named `source`, and
  context the message needs is carried in named fields. A variant only
  carries context its own layer knows.
- Variants name the failure condition together with its object (e.g.
  `UnresolvedOperation`, `HostUnavailable`, `MissingRun`), never the step
  that failed. Name the condition at the certainty you have: a failed spawn
  is `DenoUnavailable`, not `DenoNotInstalled`.
- A variant with a `source` reads "failed to ..." in its message; a variant
  that is itself the diagnosis states its condition declaratively.
- A failure an operation reports is a value (`Failure`), not an error. It
  travels in a regular field, never as a `source`.
- Start `.context()` and similar error messages with a lowercase letter
  (e.g., `"failed to read Cargo.toml"`). Error messages may be embedded in
  larger error chains, and Rust convention is lowercase for these fragments.

### Type System

- Primitives (`i64`, `String`, `bool`) are only allowed at system boundaries.
  Owned structs must always define a newtype (e.g. with `typed-fields`).
- Use enums with meaningful variants instead of bool parameters.
- Fields must never be `pub`. Implement getters instead, e.g. with `getset`.

## Version Control

- Never commit directly to `main`, always create a branch or worktree.
- Every commit should be a logical unit of change.
- Every commit must build and pass all checks. Use `just` recipes for
  verification (e.g. `just pre-commit`).
- Fixes and refactoring should be in separate commits from features.
- Each pull request should have one primary commit with a well-crafted
  message — this is what lands in the Git history since we squash merge.
  Follow-up fixups within the same PR can use simple one-liner messages
  since they get squashed into the primary commit on merge.
- Reuse the commit message as the pull request description, but reflow each
  paragraph onto one line, because GitHub renders every newline as a line
  break. Do not use `gh pr create --fill`.

### Commit Messages

- We use Git as our Version Control System and GitHub to host the code.
- We use pre-commit hooks to verify the changes before committing them.
- We follow this [style guide][git-style-guide] for commit messages:
  - Capitalized, short (50 characters or less) summary in imperative mode
    ("Fix bug", not "Fixed bug")
  - Blank line between summary and body
  - Focus on the "why" — motivation and reasoning — not what changed
  - Minimal formatting or bullet points, plain prose is preferred
  - Full sentences with simple past and present tense
  - Wrap the body at 72 characters
- Write commit messages for a reader that has no prior context and no access to
  the session history.
- Keep commit messages concise. Aim for two or three paragraphs, not more.
- Don't use backticks in commit message titles, but do use them in bodies.
- **Never** write conventional commit messages.
- **Never** add yourself as a co-author.

[adr-001]: adrs/001-adrs.md
[adr-002]: adrs/002-specifications.md
[git-style-guide]: https://tbaggery.com/2008/04/19/a-note-about-git-commit-messages.html
[tracey]: https://tracey.bearcove.eu/
