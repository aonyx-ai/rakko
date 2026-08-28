# ADR-007: The Contract Owns the Argument Vocabulary

## Status

Accepted

## Context

[ADR-004][adr-004] made an action a library crate behind the `Action` trait
and kept a command-line framework out of the contract crate: an action
depends on the contract crate and on nothing else, "not on Clawless, and not
on clap." It also named the erased action, the object-safe view that bundles
export and the registry holds, and placed it in the contract crate.
[ADR-006][adr-006] then made composition explicit data: a bundle exports a
list of erased actions, and a harness mounts those lists in code.

Defining the erased action turns a question that ADR-004 left implicit into a
blocking one. The `Action` trait leaves the type of the arguments to the
action, as `Args`, and a run reads a value of that type. Something has to
build that value from what the user asked for, and erasure is where the
difficulty appears: the erased view is all that the machinery ever sees. The
concrete type of an action is gone by the time a bundle exports it, so the
erased view has to carry the construction of its arguments with it.

Rust decides where that construction can be written. Code that builds a
struct from input is generated from the fields of that struct, and a derive
macro runs only where the type is defined. No other crate can supply it. An
implementation of a foreign trait for a foreign type is an orphan, and a
harness that is generic over actions it did not write has no fields to work
from. The annotation therefore lives in the crate that defines the arguments,
which is the action crate, and no arrangement of the other crates moves it.

What stays open is the vocabulary that the annotation speaks. If it speaks
the vocabulary of a command-line parser, then every action carries that
parser, which ADR-004 and the [vision][vision] both rule out by name. This
ADR records the answer.

## Decision

An action describes its arguments as data, in a vocabulary that the contract
crate owns.

1. **The contract crate defines the vocabulary.** It carries an `Args` trait
   and the types that describe an argument set as data: the fields that an
   action reads, their shape, and their documentation. A derive macro that
   the contract crate provides fills that description in from the type. The
   vocabulary takes its cues from [clap][clap] and stays small. It grows when
   an action needs something that it cannot say, under the same pressure that
   ADR-004 puts on every addition to the contract crate.

2. **A description states shape, not syntax.** It says that an action reads a
   boolean named `fix`. It does not say `--fix`, it does not say `-f`, and it
   does not say where the argument sits in a command tree. Every naming and
   syntax decision belongs to the projection, which reaches all mounted
   actions at once.

3. **The arguments of every action implement the trait.** The bound sits on
   the associated type, and the contract crate implements the trait for the
   empty argument set, so that an action that reads nothing states nothing.
   Erasure is then total: every action has an erased view, and an action that
   breaks the bound fails at its own `impl` block instead of inside the
   machinery.

4. **The erased action stays in the contract crate.** Nothing in erasure
   names a type of a command-line parser, so nothing pushes the erased view
   out of the crate where ADR-004 put it. An action depends on the contract
   crate, a bundle depends on the contract crate, and neither of them carries
   a command-line framework.

The decision stops at the vocabulary. Which parser the machinery renders that
vocabulary into is an implementation choice, and this ADR does not make it.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### The Derive of a Command-Line Parser

An action can derive `clap::Args` on its argument type, and the erased view
can require that trait. This is the shortest path by a wide margin. The
derive is mature, it already expresses everything an action could want to
say, and Rakko writes no vocabulary and no macro at all.

It puts a command-line parser in the dependency tree of every action, which
ADR-004 and the vision rule out by name. Their reason is churn: whatever
every action depends on releases into the whole fleet. It is also not a
dependency that a later refactor can quietly undo, because the annotation is
written in the source of every action, so the cost of reversing this grows
with each action that exists.

The subtler cost is what such an annotation says. Defaults and validation
belong to the attributes and run during parsing, so a caller that builds the
argument value directly gets neither. The scheduler, a CI runner, and a test
are all such callers, and each of them would either go through a command line
that it has no reason to have, or silently skip the rules that the action
believes are in force.

### Erasure Outside the Contract Crate

The bound on the arguments is what pulls a parser toward the contract crate,
so the erased action can move to where such a dependency is welcome: into the
machinery crate, or into a new crate between the contract and the machinery
that depends on the parser but not on Clawless. ADR-004 placed the erased
action before this problem was visible, and moving it looks like the smaller
correction.

It does not work, because it misreads where the dependency enters. The
annotation is written on the argument type in the action crate whatever crate
the erased trait lives in, so the parser reaches every action either way and
the move buys nothing. It also costs. A bundle erases the actions that it
exports, so the crate of the erased trait is a dependency of every bundle. A
bundle is the unit of rollout across the fleet, and a major version of the
parser underneath it would cascade into every project, which is the churn
that ADR-004 exists to prevent.

### The Derive of a Serialization Format

The erased view can require `Deserialize` from [serde][serde] and build the
argument value through a deserializer over the parsed input. The annotation
is then a description of data rather than of a command line, which is what
this ADR is after, and serde sits in most dependency trees already, so the
churn argument is weaker than it is for a parser.

Deserialization constructs, but it does not describe. A command line has to
exist before any value does, because the projection needs the field names,
their shapes, and their documentation to build `--fix` in the first place,
and a `Deserialize` implementation cannot be asked which fields it accepts.
The description would need a second derive beside the first, and that
description is the vocabulary that this ADR adopts. The result is the same
vocabulary, two annotations on every argument type, and a dependency that the
contract crate did not need.

### Raw Arguments

The machinery can hand an action what the user typed and let the action make
sense of it. There is no vocabulary to design, no macro to maintain, and an
action can express anything at all.

ADR-004 already rejected this shape: an action "never defines a command and
never parses arguments; it reads what the trait handed it." The projection
could generate no help, because nothing would describe what an action
accepts, so every action would document its arguments in prose and drift from
what it parses. Parsing itself would return once per action, which is the
duplication that the vision leaves behind with the shell recipes.

## Consequences

- The contract crate carries a vocabulary and a derive macro, and it trails
  the expressiveness of a mature parser permanently. Value enumerations,
  argument groups, conflicts, and fallbacks to the environment cannot be said
  at first, and each one that an action genuinely needs arrives as a release
  of the crate that the whole fleet depends on. ADR-004 named this bottleneck
  and accepted it. This decision is where the fleet starts to pay for it.
- An action and a bundle depend on the contract crate and on nothing else.
  Which command-line framework the machinery uses, and whether that changes,
  is invisible to both of them. A harness takes such a change as an ordinary
  dependency update.
- The command line is uniform across the fleet by construction, because no
  action can name its own flags and one projection decides how every argument
  appears. The cost is that an action cannot express a convention that would
  read better in its own command, even when its author is right.
- An action is not unaware of its arguments, and it never was: it declares
  their type and reads their fields. The annotation adds a description of
  input that the action already knows. What an action still does not know is
  that a command line exists.
- The boundary between shape and syntax is clear in the middle and not at the
  edges. A positional argument, or a field whose type is an enumeration of
  the action's own, forces a judgment about whether the description or the
  projection decides. Each such case pushes on the vocabulary, and review has
  to hold the line, as ADR-004 already asks elsewhere.
- The bound on the argument type is a breaking change to the `Action` trait.
  It is free today and expensive once actions exist outside this repository,
  so it lands before the first one does.
- A derive macro in the crate that every action depends on is the hardest
  kind of code to change compatibly, and it costs compile time in every
  project of the fleet.
- Something still has to turn the vocabulary into a real command line, and
  that mapping lives in the machinery. The vocabulary is what keeps the
  choice of parser an implementation detail, and what keeps a later change of
  that choice invisible to actions and bundles.
- The trait, and its implementation for the empty argument set, are enough to
  define the erased action and to build the machinery. The vocabulary and the
  derive can follow the first action that reads an argument, so that a real
  case shapes them instead of speculation.

[adr-004]: 004-actions-as-libraries.md
[adr-006]: 006-composition-is-explicit-data.md
[clap]: https://crates.io/crates/clap
[serde]: https://crates.io/crates/serde
[vision]: ../VISION.md
