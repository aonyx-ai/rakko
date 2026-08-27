# ADR-005: Composition Is Explicit Data

## Status

Accepted

## Context

[ADR-004][adr-004] settled what an action is: a library crate behind the
`Action` trait, with the command-line interface as a projection that Rakko
generates from mounted actions. It also named the registry view — the erased
action — and said that bundles export lists of them. What it did not settle
is the step in between: how an action travels from a crate that a project
depends on into the registry that the harness builds. This ADR records that
decision.

Two forces shape it. First, the fleet adopts changes through review. The
[vision][vision] makes a bundle release plus Renovate the rollout mechanism,
so most changes to what runs in a project arrive as bot pull requests. A
reviewer — human or agent — must be able to answer "what runs in this
project, and what does this pull request change about it" from the code in
front of them.

Second, the Rust ecosystem pulls the other way. Registration at a distance
is a polished pattern: the [inventory][inventory] and [linkme][linkme]
crates let any crate submit entries to a collection that the binary gathers
at link time, and [Clawless][clawless] itself registers commands this way.
The pattern is attractive precisely because it removes the step this ADR is
about, and without a recorded decision, the mount machinery could drift into
it one convenience at a time.

Bundles sharpen the timing. The MVP mounts actions directly and no bundle
exists yet, but the rollout story rests on bundles, and machinery that grows
up around single actions would have to be reshaped for lists later. The
decision must precede the machinery, so that the MVP is built in the shape
that bundles need.

## Decision

What runs in a project is a value in the code of its harness. The harness
mounts plain lists of erased actions, and nothing reaches the registry any
other way.

1. **The harness mounts lists, in code.** The mount machinery accepts lists
   of erased actions, and the harness passes them explicitly. No linker
   section, no build script, no configuration file, and no environment
   variable adds an action. Reading the harness, and following the names it
   mounts, reproduces exactly what runs — at the versions that Cargo
   resolved.

2. **A dependency alone changes nothing.** Cargo delivers code; the harness
   decides what runs. An action crate that sits in the dependency tree
   without a mount naming it is inert. An adoption is therefore visible
   where it matters: the harness names the new action, and the dependency
   line merely delivers it.

3. **A bundle is a list in a meta-crate.** A bundle exports the same kind of
   list that a harness mounts, and a bundle contains another bundle by
   including that bundle's list in its own. A bundle carries no machinery —
   it is a normal dependency plus the list it exports. Implementation is
   deferred, but the shape binds now: because the mount machinery accepts
   lists from any crate, the first bundle will slot in without a change to
   the contract.

4. **Lists are ordinary values.** A list of erased actions is plain data
   that ordinary Rust manipulates: a harness can concatenate two lists,
   filter one, or inspect what a bundle exports. Leaving one action out of a
   mount is code like any other, so exceptions need no dedicated machinery.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Registration at Link Time

The [inventory][inventory] and [linkme][linkme] crates let each action crate
register itself into a collection that the binary gathers at link time. The
harness would depend on action crates and iterate the collected entries, and
adopting an action would be one line in `Cargo.toml`. This is the
established Rust pattern for plugins without dynamic loading, and it is the
convenience this ADR gives up.

It fails the review force twice. The point of composition disappears: no
code in the project names the action, so what runs is an emergent property
of the link, and the diff of an adoption shows a dependency, not a decision.
And the mechanism's failure mode is silence: whether a registration from a
crate that nothing names survives the link depends on the linker, its flags,
and link-time optimization. A check that silently does not run is the exact
failure Rakko exists to end. Clawless registers commands per module with
inventory, and that use stays internal to one binary; it was never a
mechanism for composing commands across crate boundaries.

### Dynamic Plugin Loading

Actions could be shared libraries that the harness loads at run time, the
way editors load plugins. Rust rules this out on its own: the language has
no stable ABI, so a plugin only loads safely when the same compiler with the
same settings built both sides — a constraint that a fleet of independently
built projects cannot hold. A stable boundary would demand a C ABI or a wire
protocol on day one. Distribution inverts as well: instead of Cargo
delivering source that every project builds, something must deliver built
artifacts per platform. The [vision][vision] already rules out the plugin
platform, and run-time composition falls with it, because what runs would
depend on the filesystem at launch, and reading the harness would answer
nothing.

### Composition in a Configuration File

The harness could stay generic and read a configuration file that names the
mounted actions. This is explicit, and it is data — but the wrong kind of
data. Names in a file are strings that resolve at run time against whatever
happens to be linked, so a typo is a late error, and the file can drift from
the dependency tree in both directions. Composition also splits across two
artifacts: `Cargo.toml` pins the versions while the file picks the names,
and no single place states what runs. In code, the compiler binds the names
to the versions that Cargo resolved, and a dependency that nothing mounts is
at least visible to tooling.

## Consequences

- A bundle release is the fleet rollout, exactly as the [vision][vision]
  describes: publishing a new bundle version turns into one Renovate pull
  request per project, and the CI result of each pull request is the
  per-project rollout gate. This works because composition resolves at
  compile time, inside the pull request that delivers the change.
- Adoption has a history. Mounting or removing an action is a reviewed
  commit, and the harness together with `Cargo.lock` records what ran at
  every point in a project's history.
- The friction of naming everything concentrates instead of disappearing. A
  harness names a bundle once, and the bundle names its actions once for the
  whole fleet. The naming that registration at a distance would hide is
  still written — but in one reviewed place, not in every project.
- Exceptions are expressible from the start, because a mount is list
  manipulation. What a bundle exception should look like, and how visible
  the fleet wants them to be, remains open; this ADR only guarantees that
  expressing one needs no new mechanism.
- Duplicates become possible. Two mounted lists can carry the same action,
  and bundles that contain bundles make the overlap ordinary. The registry
  keys actions by name, so a collision needs one defined, visible meaning.
  That definition belongs to the specification of the registry; this
  decision only rules out that a collision resolves silently.
- Nothing can enumerate the actions that could run. There is no registry to
  query beyond what a harness mounts, so tooling that answers "what is
  available" must read documentation instead. This is the cost of ruling out
  the plugin platform, accepted knowingly.
- The harness is code, and nothing mechanical keeps it small. The same
  expressiveness that makes exceptions cheap lets logic accumulate in the
  one file that every project owns. Review must hold the discipline that a
  harness stays a few lines of naming, as [ADR-004][adr-004] already demands
  of the projection.

[adr-004]: 004-actions-as-libraries.md
[clawless]: https://github.com/aonyx-ai/clawless
[inventory]: https://crates.io/crates/inventory
[linkme]: https://crates.io/crates/linkme
[vision]: ../VISION.md
