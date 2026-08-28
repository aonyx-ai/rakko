# ADR-008: Output Renderers

## Status

Accepted

## Context

A run of an action produces an outcome, and today that outcome goes nowhere.
The projection drives the action, drops what it returned, and exits zero
whatever happened. The specification of the projection says so plainly: "A
run shows nothing." Wiring that outcome to a reader, and to an exit code, is
the next step, and it is the step that fixes what every reader of Rakko sees
from here on.

Three readers wait at the other end of that wire, and the [vision][vision]
names all three: a developer at a terminal, a machine reading a pipe, and the
pull request of a CI run. They want different bytes for the same outcome. The
developer wants the offending line with a caret under it, the way a compiler
answers. The machine wants a schema it can depend on across releases. The
pull request wants [workflow commands][annotations] that put a comment on the
right line of the diff.

[Clawless][clawless] gives a command two slots for this. A value that a
command hands to the chassis renders through `Display` for text and through
`Serialize` for JSON, and the `--json` flag picks between them. Two slots fit
two readers. The third has nowhere to sit, and the two that fit come with a
condition: the rendering and the schema are both properties of the type that
the machinery hands over, so whatever type that is becomes the published
output of Rakko.

The contract crate has already drawn one line here. A finding "says nothing
about how it looks, because the shape of the output belongs to the
machinery," and the machinery is what maps each state of an outcome to output
and to an exit code. What sits on the machinery side of that line is
unwritten, and the first code to sit there decides the shape for everything
after it. Compiler-style output sharpens the timing: a source excerpt under a
finding needs data that a finding does not carry, and the obvious repair —
put the excerpt in the finding — is a change to the crate whose churn radius
is the whole fleet ([ADR-004][adr-004]).

The exit code carries the same weight from the other side. It is the only
signal a CI job reads without parsing anything, so it is a contract with
every workflow in the fleet, and a contract that cannot change quietly.

## Decision

Renderers own the shape of the output. A run selects one renderer, and every
renderer reads the outcome of that run and nothing else.

1. **A renderer turns an outcome into output.** It reads the outcome and the
   context that the run already has, and it produces the bytes that a reader
   gets. Everything that a reader sees is a decision of a renderer: the
   source excerpt, the caret, the color, the order, the grouping, the counts,
   and the summary. One renderer serves the terminal, one serves a machine,
   and one serves a pull request. Adding a reader means adding a renderer.
   How much of an outcome a reader gets is a renderer's decision as well, so
   the flags that ask for less and for more reach the renderer, and no action
   reads them.

2. **An action reports, and it never renders.** An action returns an outcome
   and writes to no stream. It cannot ask whether a terminal is attached, it
   cannot color a word, and it cannot learn which renderer is active. What an
   action produces is therefore a value that a test asserts on, and the
   uniform output that the vision promises holds by construction rather than
   by convention.

3. **A finding states what and where.** Everything a reader sees beyond that
   comes from a renderer, including the source text that a compiler-style
   block shows. A renderer reads the project; an action does not read files
   on a renderer's behalf. A new reader therefore costs a renderer, and it
   costs no change to any action and no release of the contract crate.

4. **The run selects the renderer, and CI selects it without a flag.** The
   vision asks CI to run the command that a developer runs and get the same
   answers, so a workflow must not have to pass an output flag to get
   annotations. The environment therefore supplies the default renderer, and
   an explicit flag overrides it. This is detection, and the vision rules
   detection out for what an action reads and writes, where a wrong guess
   corrupts a project. A wrong guess about a reader shows the wrong bytes to
   someone who can see that they are wrong, and who holds the flag that
   corrects it.

5. **The machine-readable output is a contract of its own.** The schema
   belongs to the renderer that emits it, it carries its own version, and it
   does not fall out of the types that actions produce. A serialization
   derived from the contract types would tie the schema to the fields of
   those types, so every refactor of the contract would break every consumer,
   and the schema would inherit a version number that means something else.

6. **The exit code is derived from the outcome, and it separates a verdict
   from a failure.** An action that passed exits zero, and an action that
   does not apply exits zero as well, because a skip is an answer. An action
   that found problems exits nonzero, and an action that stopped exits
   nonzero with a different code. A CI job can therefore tell "this project
   has a problem" from "this check could not answer," which are different
   events for whoever is on the other end.

The decision stops at the seam. Which fields a finding carries beyond its
message and its location, whether a finding has a severity and whether a
warning can exist without failing an outcome, the exact text that the
terminal renderer draws, the numbers behind the exit codes, and the crate
that holds the renderers are all left open. Whether an action ever gets a
channel to report progress while it runs is open too; today an outcome
arrives when the run is over.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### The Two Slots of the Chassis

The machinery can implement `Display` and `Serialize` on what it hands to
Clawless and let `--json` choose, which is what every Clawless application
does. There is no new concept, no selection to write, and the flags that the
projection already carries do the work.

Two slots do not hold three readers. Annotations would arrive as a `Display`
that reads the environment, so one trait would give two answers for one
value, and the next reader after that would have no slot at all. The slots
also decide more than they look like they decide: a `Serialize` on the types
that travel from an action is the published schema, which is the coupling
that point five rejects. Rakko still hands its result to the chassis, and
that is compatible with this decision. What changes is that a renderer runs
first, and the chassis carries what the renderer produced.

### Actions Render Their Own Findings

Each action can write what it found, in whatever shape fits it best. The
author of an action knows the problem best, and an action that wraps a tool
with good output could simply pass that output through.

This is the Justfile problem restated. Each recipe formats its own output
today, so the fleet has no uniform output and no machine-readable one, and
that is a cost the vision exists to remove. It also contradicts
[ADR-004][adr-004] directly: an action depends on the contract crate and on
nothing else, and rendering needs a terminal, colors, and eventually a
serialization format. Parallel execution finishes the argument, because
actions that write while they run interleave their lines, and nothing can
reassemble them afterwards.

### One Renderer and a Converter Behind It

The machinery can emit JSON and nothing else, and a separate tool can turn
that into what a reader wants: a GitHub problem matcher for annotations, and
a formatter for a terminal. Cargo works this way, and it keeps exactly one
output path inside the tool.

The developer at a terminal is the primary reader, and their default cannot
be a pipe into a second program. A problem matcher moves the conversion into
a workflow file, so every project in the fleet carries a copy of a regular
expression that Renovate cannot update and that no compiler checks — the
copied configuration that Rakko replaces. It also gives up the readable
answer in CI, where a log line that a human can read is what someone wants
first when a job goes red.

### Renderers in the Chassis

Annotations are not a Rakko concept. Any Aonyx command-line tool that runs in
CI wants them, Clawless already owns the presenter and the output flags, and
a renderer that lives there would reach every tool at once instead of only
this one.

Clawless renders events — a message, a detail, an artifact — and a finding
with a location is not an event. Teaching the chassis about diagnostics
before Rakko has built one renderer would settle the shape of the diagnostic
in the layer that has the least evidence about it. This decision fixes where
the seam is, not which crate holds the code behind it, so a renderer that
proves general can move upstream later, and nothing an action sees would
change.

## Consequences

- A new reader is additive. GitHub annotations, a stable JSON schema, and
  whatever follows them arrive as a renderer, and neither the actions nor the
  contract crate move. The fleet pays nothing for a reader it does not use.
- The default output can be as good as a compiler's without any action doing
  work for it, because the renderer reads the source. The cost is that it
  reads the source after the run: an action that rewrote a file can leave a
  renderer showing an excerpt that no longer matches the finding. How a
  renderer handles that belongs to its specification; this decision only
  fixes that it is not an action's problem.
- An action that wraps a tool with excellent output of its own must translate
  that output into findings and give up the tool's rendering. The uniform
  output is worth more than any one tool's presentation, but the translation
  is real work for the author, and a tool that reports something the finding
  cannot say loses it.
- The JSON schema becomes a published artifact with its own compatibility
  promise, and someone has to maintain it and version it. In exchange the
  contract crate can change shape without breaking a consumer, which is the
  trade this makes on purpose.
- The exit code becomes a contract with every workflow in the fleet on the
  day it first ships. A run drives one action today, and when a run drives
  many, their codes have to combine into one. This decision does not say how.
- The same command produces different bytes in CI than on a laptop. That is
  what the selection rule is for, and it is also a hazard: someone who reads
  a CI log is reading output that they cannot reproduce locally until they
  know that the override exists. The override, and its documentation, is what
  keeps the detection honest.
- Rakko now has more targets than the chassis has slots, so what a renderer
  produces still has to reach the chassis through one of them. This presses
  on the boundary with Clawless. The pressure resolves upstream, in the
  chassis, and not as a workaround inside Rakko.
- [ADR-004][adr-004] sends logic into the contract crate and keeps the
  projection thin. Rendering is the first substantial logic that belongs in
  neither: it needs a terminal, colors, and the source tree, and none of that
  may reach an action. Where it lands is an implementation choice. That it
  stays out of the contract crate is not.
- A finding carries a message and a location, so that is all a renderer can
  show. A severity, an identifier for the rule, a span, and a suggested fix
  are the obvious next requests, and every one of them is a release of the
  crate that the whole fleet depends on. The bottleneck that ADR-004 named
  and [ADR-007][adr-007] started paying for is where this decision meets its
  limit.

[adr-004]: 004-actions-as-libraries.md
[adr-007]: 007-argument-vocabulary.md
[annotations]: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands
[clawless]: https://github.com/aonyx-ai/clawless
[vision]: ../VISION.md
