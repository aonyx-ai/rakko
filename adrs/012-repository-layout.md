# ADR-012: Repository Layout

## Status

Accepted

## Context

Every crate of Rakko lives in `crates/`. The listing mixes three things: the
contract, the actions that do the work, and the machinery that several actions
share. `rakko-action` sits beside `rakko-lint-toml`, and `rakko-taplo` beside
the two actions that are its only callers. A reader who does not know the
roster cannot tell which of these a project adopts.

The kinds also grow at different rates. An action wraps one tool and carries
its own specification, and every tool that Rakko learns adds one. The
machinery grows only when the actions need something they cannot express. A
flat directory buries the part that a newcomer reads first, and each release
buries it further.

The [vision] adds a third kind. A bundle is a meta-crate that exports a list
of actions. Bundles arrive with the first rollout, and a layout decided after
they exist moves crates twice.

The tempting split is public against internal, and the tree cannot draw it.
Cargo publishes no crate whose dependencies are absent from the registry.
Every helper that an action uses therefore reaches crates.io, whatever
directory holds it. What is left to promise is stability, and a promise has to
reach a consumer. A consumer reads a crate name and never sees this
repository.

## Decision

A directory groups crates by who picks them and why.

1. **`actions/` holds the catalog.** An action is a unit of work that a
   harness mounts, and a project depends on it for what it does. Each action
   keeps its specification beside its source.

2. **`crates/` holds the machinery.** The contract, the projection, and
   everything the actions share live here. Nobody picks these for what they
   do. A harness takes the projection because it is a harness, and an action
   takes the contract and the wrappers because it is an action.

3. **The audience decides, not the topic.** `rakko-taplo` stays with the
   machinery, although the two TOML actions are its only callers. No project
   depends on it for what it does. A crate does not move when its callers
   change in number.

4. **The layout promises nothing.** It is navigation for a reader of this
   repository. It states no publication tier and no stability guarantee,
   because a directory carries neither beyond the tree.

The decision stops at the grouping. Which crates Rakko publishes, and what
each of them promises, stays open. Our actions are opinionated, and a bundle
is more opinionated still, so we do not yet know whether anyone outside Aonyx
adopts either. Use of the tool answers that, and the answer belongs where a
consumer can read it. The home of a bundle stays open for the same reason. A
project picks a bundle the way it picks an action, so this rule alone seats it
in `actions/`. The first bundle can decide whether its kind earns a
directory.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### One Directory

The flat `crates/` needs no rule and gives every crate the same path.

It answers the question a reader asks least. That something is a crate is
already plain. Which crates are the product is not, and only the roster tells
them apart. Names do not carry it either, because `rakko-taplo` and
`rakko-format-toml` both point at a tool. The imbalance also worsens on its
own, because the catalog is the part that grows.

### A Directory per Kind

A finer split, one directory each for the actions, the tool wrappers, the
contract, and the projection, makes every directory state what a crate is.

The kinds are not stable enough to build a tree on. `rakko-worktree` is
neither a wrapper nor part of the contract, and the next piece of machinery
will be a third thing again. Every crate that fits nothing then argues for
another directory. The split also separates crates that nobody looks for
separately, because a reader who wants the machinery wants all of it.

### A Directory per Publication Tier

A `public/` and an `internal/` put the distinction where the question started,
and a reviewer sees a dependency that crosses the line.

The line is not ours to draw in the tree. Publishing an action publishes every
crate beneath it, so the internal directory reaches crates.io as well, and its
names are taken there like any others. The tier then means only that we make
no promise about stability. That claim has to reach a consumer, and a
directory never does.

### Machinery Beside Its Callers

A wrapper nested under the action that needs it stays next to its reason for
existing. A reader of one action then finds everything it uses in one place.

Most of the machinery has more than one caller, and the contract and the tool
layer have nearly twenty each. Nesting therefore picks an owner that the
dependency graph does not support. The crates with a single caller today are
the ones most likely to gain another. A layout that moves a crate whenever
that happens rewrites paths for a fact that changed nothing.

## Consequences

- Listing `actions/` gives the list of what Rakko does, and it stays that list
  as the count grows.
- Every path that names a moved crate changes once: the workspace members, the
  workspace dependency table, the entries in the Tracey configuration, the
  dependencies of the harness, and the sentence in `AGENTS.md` that says where
  a specification lives. Git records the moves as renames, so the history of
  each crate survives.
- The workspace gains a second member glob. A crate that lands in neither
  directory is not a member, and Cargo reports that as a missing dependency
  rather than as a misplaced file.
- Each new crate needs a judgment that a flat directory never asked for, and
  the audience is a softer test than the kind. The wrappers are where it will
  be argued, because tools name them as they name the actions.
- Nothing about the code changes. No crate is renamed, no dependency edge
  moves, and the build produces what it produced before.
- What Rakko publishes, and what it promises, is now visibly unanswered rather
  than half-answered by a directory. Use of the tool decides it, and this ADR
  does not.

[vision]: ../VISION.md
