# ADR-004: Actions as Libraries

## Status

Accepted

## Context

Rakko turns maintenance tasks into versioned Rust crates that every project
composes into its own small command-line tool. The [vision][vision] places
Rakko in the middle of a three-layer architecture: mise provisions the
external tools, Rakko provides the actions and the machinery to run them,
and [Clawless][clawless] provides the command-line chassis. The unit of
work is the action, and this ADR settles what an action is before the first
one exists.

The obvious shape is no shape at all: write each action directly as a
Clawless command, the way a normal CLI grows. Two forces rule this out, and
they shape the decision below.

First, actions must compose across crate boundaries. A project adopts
actions as Cargo dependencies, directly or through bundles, and Renovate
rolls new releases across the fleet as pull requests. The harness of a
project must be able to mount a list of actions that another crate
exported, at a version that Cargo resolved.

Second, whatever every action depends on has a churn radius of the whole
fleet. All actions and all harnesses must share types — the trait, the
outcome, the findings — so one crate sits underneath everything, and each
release of it cascades into a Renovate pull request for every action and
every project. A command-line framework has its own release cadence, and
that cadence must not set the pace for the fleet.

The [glossary][glossary] already names the parts of this design. This ADR
records the decision behind those names and its reasons.

## Decision

An action is a library crate that implements the `Action` trait. The
command-line interface is a projection that Rakko generates from mounted
actions. Two crates carry this split: `rakko`, the contract crate, and
`rakko-cli`, the projection.

1. **Actions are libraries.** An action implements the `Action` trait from
   the contract crate and depends on nothing else from Rakko — not on
   Clawless, and not on clap. The trait describes the action as data: its
   name, its documentation, and its arguments. An action never defines a
   command and never parses arguments; it reads what the trait handed it
   and does its work.

2. **The contract crate stays minimal.** `rakko` contains the `Action`
   trait, the shared types, and the machinery that registers and runs
   mounted actions. It contains nothing that depends on a command-line
   framework. Every action and every harness depends on this crate, so its
   churn radius is the whole fleet: every addition must justify itself
   against that radius, and everything that can live elsewhere does.

3. **The CLI is a thin projection.** `rakko-cli` turns mounted actions into
   Clawless commands: the command tree, the help text from each action's
   documentation, and the shared output flags. It implements no logic of
   its own beyond what it needs to define its commands; everything else is
   a call into the contract crate. Only harnesses depend on `rakko-cli`, so
   Clawless never enters the dependency tree of an action.

4. **The registry holds erased actions.** The `Action` trait is written for
   action authors. The registry needs one uniform view, so the contract
   crate defines the erased action: the object-safe type that wraps any
   action behind one interface. Bundles export lists of erased actions, and
   the registry that the harness builds holds them. Action authors never
   see the type; erasure happens when an action is exported or mounted.

5. **Actions detect their own applicability.** An action examines the
   project and decides whether it applies. When it does not, it skips, and
   the skip is visible in the output. Applicability lives in the trait, not
   in the harness, so a bundle can stay broad: mounting an action that a
   project does not need is safe and honest. "Skipped" is an outcome, not
   an error.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Actions as Clawless Commands

Each action can be a Clawless command, written against the chassis with no
layer in between. This is the shortest path to a working CLI, but it fails
both forces from the context. Clawless registers commands per module with
the [inventory][inventory] crate, and this registration does not compose
across crate boundaries, so a harness cannot mount commands that a
dependency exported. And a command-shaped action couples every action to
the CLI framework: each Clawless release would cascade through the fleet,
and any future embedding of actions without a terminal — a scheduler, a CI
runner, a test — would drag the CLI along.

### One Crate With a CLI Feature

The contract and the projection can share one crate, with Clawless behind a
`cli` feature that only harnesses enable. This saves a crate, and the
feature keeps Clawless out of an action's build. But a feature hides the
dependency, not the churn. One crate has one version, so every
Clawless-driven change still releases a new `rakko`, and Renovate still
delivers it to every action and every project. A breaking change behind the
feature still forces a major version bump of the crate the fleet depends
on. Two crates turn the boundary into a compile-time fact that no feature
flag can blur.

### Actions as Standalone Binaries

Each action can be its own executable, discovered on the PATH the way cargo
discovers subcommands. This decouples actions from the CLI framework
completely, but it makes Rakko a plugin platform, which the vision rules
out. Composition stops being explicit data: reading the harness no longer
tells you what runs, because the PATH decides. Findings must cross a
process boundary, which demands a stable wire protocol on day one. And the
shared behavior — help text, output flags, JSON rendering — is no longer
generated in one place but reimplemented in every binary.

## Consequences

- An action is testable as a library. Tests call the trait, no process is
  spawned, and no output is scraped.
- The CLI stays uniform. Help text, output flags, and rendering exist once
  in the projection, and every mounted action gets them without writing CLI
  code.
- Clawless can move without moving the fleet. A chassis release touches
  `rakko-cli` and the harnesses; the actions do not notice.
- Bundles can stay broad. An action that does not apply skips visibly
  instead of failing, so the recommended bundle can cover every stack
  without per-project configuration.
- The contract crate becomes the bottleneck for expressiveness. An action
  can only declare what the shared types can say, and when an action needs
  more, the contract crate grows — a fleet-wide release. This pressure is
  deliberate, but it is a standing cost.
- A major version bump of `rakko` is a fleet-wide migration, because Cargo
  unifies the crate only within one major version. The contract evolves
  conservatively, and this caution slows some changes down.
- The thinness of `rakko-cli` is a discipline that review must hold.
  Nothing mechanical stops logic from growing in the projection because it
  was convenient there; a reviewer must send it down into the contract
  crate.
- Every action carries applicability detection, and detection can be wrong.
  A false "does not apply" hides a real check behind a skip message, so the
  message must say enough for a reader to question it.

[clawless]: https://github.com/aonyx-ai/clawless
[glossary]: ../GLOSSARY.md
[inventory]: https://crates.io/crates/inventory
[vision]: ../VISION.md
