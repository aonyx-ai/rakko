# ADR-010: Project Root

## Status

Accepted

## Context

The context that an action receives names the root of its project, and that
one directory decides what the action may touch, where it writes, and what
its output says: the default access declaration covers the whole project, the
layout derives the configuration and cache directories from the root, and a
finding reports its location relative to it.

The command line hands over the directory that the user ran the command from,
a placeholder from the first day that the specification of `rakko-cli`
records in prose. A run from a subdirectory therefore reads the wrong files,
writes its cache into the middle of the tree, and reports paths that no other
tool in the project agrees with — and nothing fails while it happens. A
replacement has to say what marks a root, and what a run does when nothing
marks one.

A marker has to be an entry that a walk tests without reading it. A rule that
looks inside a file depends on the format of that file and on every later
version of it, and it misses a project that keeps the same information in a
place the rule does not know. A path either exists or it does not, the test
is the same in every project, and an error message can state it in one
sentence.

The two markers that projects already have both fail. Git fails by
vocabulary: the [glossary][glossary] says a project is usually a Git
repository and Rakko does not require one, and "repository" left the
vocabulary for exactly that reason. Mise ([ADR-003][adr-003],
[ADR-009][adr-009]) fails by its own layout: among the paths mise reads is
`.config/mise/config.toml`, which is also where a user keeps their global
configuration, so a walk over mise paths finds a project in nearly every home
directory — and mise configuration nests by design, so a subtree that pins a
tool version would capture a run.

What is left is a marker that Rakko owns and that means nothing to anything
else. The place for it exists: the layout already defaults the configuration
directory to `.config`, and `.config/<application>.toml` is where our
applications keep the file that belongs to them.

## Decision

The project root is the directory that holds `.config/rakko.toml`, and a run
that cannot find one stops.

1. **One file marks the root.** Rakko never reads the file; its presence is
   the whole test. A project writes it when it adopts Rakko, next to the
   harness and the mise task of [ADR-009][adr-009].

2. **A run searches upward from the working directory** and takes the first
   directory that holds the marker. The root is a property of the tree, not
   of where the user stood. A project inside a project resolves to the inner
   one, because the inner one carries its own marker.

3. **A stated root wins over the search.** A caller that names the root skips
   the walk. A test, a checkout that is laid out in a way Rakko does not
   expect, and a caller that already knows the answer stay supported, which
   is what makes the next point affordable.

4. **A run that finds no marker stops with an error.** It does not fall back
   to the working directory: a wrong root is quietly wrong everywhere, while
   an error that names the missing file costs one message.

The decision stops at the rule. Whether the file ever holds content, whether
the marker later moves to the directory form of the convention, how a caller
states a root, whether a run remembers what it found, and which library
performs the walk are all open.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### The Working Directory

The placeholder needs no rule, no walk, and no error path, and it is correct
whenever the user stands in the root.

The output of a run then depends on where the user stood, and the failure is
silent: an action that finds no configuration reports a clean project, and
relative paths point at a directory that no other tool uses. A tool that
answers differently from a subdirectory is not one that a CI job and a
developer can both trust.

### The Mise Configuration of the Project

A walk that stops at a mise configuration file gives every adopter a root
with no new file, and that root equals the one mise resolves.

The marker set has to cover every path mise reads, one of which is the global
`.config/mise/config.toml` of a user, so a home directory becomes a project
root — and an action would declare access to it and write a cache into it.
Nested mise files that only pin tool versions would capture runs as well.
Both failures are the placeholder in a new form.

### The Task Declaration Inside the Mise Configuration

Taking only a directory that declares the task of [ADR-009][adr-009] avoids
both traps and means nearly the right thing: this directory knows how to run
maintenance for this tree.

It has to parse another tool's configuration, which the context rules out.
Mise accepts several file names, takes tasks from a directory of scripts as
well as from a table, and merges what it finds across a walk of its own, so
the rule has to reproduce enough of mise to stay right, and keep reproducing
it. The marker also vanishes for a project that adopts Rakko without the
canonical task, which ADR-009 allows.

### The Git Repository

The nearest `.git` finds the root of nearly every Aonyx project with no
adoption step, and matches what a developer believes their root is.

A project is usually a repository and Rakko does not require one, so
answering with `.git` makes Git a requirement after all. The marker is also
imprecise: a worktree carries `.git` as a file, a submodule or a vendored
checkout puts a second marker below the first, and one repository can hold
several trees that Rakko maintains separately. Even as a second marker behind
Rakko's own it costs more than it saves, because every repository then yields
a root and the error that says a project is not set up never appears.

### A List of Ecosystem Markers

A walk over familiar files — a workspace `Cargo.toml`, a `package.json`, a
`pyproject.toml`, a `.git` — gives a tree a root before it adopts anything.

Every entry is another way to be wrong, and the ways compound: a polyglot
project holds several of these files at several depths, and the rule resolves
to whichever sits lowest. A root that a user cannot predict, from a rule that
an error message cannot state in a sentence, is the problem in a new form.

### The Answer of Mise Itself

Mise computes a configuration root for its own purposes and can pass it to a
task through the environment, so Rakko could reuse the answer instead of
walking, and the two roots could never disagree.

That makes the harness runnable only through mise. [ADR-009][adr-009]
deliberately left it an ordinary Cargo package that `cargo run` starts with
no environment, and the variable is absent exactly there — and in a debugger,
an editor, and a copied binary. Asking mise at run time through a subprocess
trades the missing variable for a process on every run and a hard dependency
on a tool being on the path.

### The Location of the Harness

A harness knows the directory of its own manifest at compile time, so it
could report a root a fixed number of levels above it, with no walk and no
marker.

That bakes the path of one machine into a binary, which is wrong exactly
where being right matters most: a cached binary in CI, a build from another
checkout, a moved build directory. ADR-009 also left the directory of the
harness open, so the number of levels is not a constant Rakko may assume.

## Consequences

- Adoption gains a file, `.config/rakko.toml`, and this repository writes one
  as well. That is the price; every alternative that avoids it pays with a
  root that is sometimes wrong.
- The marker means one thing. It cannot appear in a home directory or arrive
  from a parent that belongs to somebody else, so a run either has the root
  that its project named or it has none.
- Rakko owes nothing to the format of another tool. The walk reads no file,
  so a change in how mise names its configuration, or a decision that
  replaces mise, leaves the rule standing.
- The file is empty for now, and a file that exists only to be found invites
  deletion. The documentation and the error message have to say what it is
  for, and it earns its place only when Rakko has something to keep in it.
- The rule — an upward walk over path markers, an error when none match — is
  what a project discovery library provides, so the projection can adopt one
  instead of writing the walk. Which library is an implementation choice.
- The missing-marker error is the first thing many new adopters see, so it
  has to name the file and the fix, not the walk that failed.
- The specification of `rakko-cli` loses the placeholder paragraph and gains
  requirements for the search, the stated root, and the failure. The context
  type in the contract crate does not change, because the root was always a
  value that a caller supplies.

[adr-003]: 003-mise.md
[adr-009]: 009-harness-entry-point.md
[glossary]: ../GLOSSARY.md
