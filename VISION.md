# The Rakko Vision

The rakko (ラッコ) is the sea otter: the otter that keeps a favorite pebble
and uses it to crack open what it needs. Rakko is that pebble for our
repositories — the tool each project carries to maintain itself.

## Why Rakko Exists

Every Aonyx project maintains itself with a copy of the same tooling: a
Justfile full of recipes, a stack of linter and formatter configs, and an
environment definition that provisions the tools. This setup has served us,
but its costs grow with every project we add:

- **Recipes are shell scripts.** Every line runs in its own shell, logic
  cannot be shared between recipes, and the tricky parts — git-clean guards,
  CI detection, parallel execution — are the parts bash is worst at. None of
  it works on Windows.
- **The copies drift.** A fix to a recipe or a config in one project does
  not reach the others. Every project is a fork of our conventions, and
  forks diverge.
- **Rollout is manual.** Adopting a new check across the fleet means editing
  every project by hand. In practice, this means new checks reach some
  projects and not others.
- **Updates are invisible.** Our environment tooling has no Renovate
  integration, so the tools our checks depend on quietly fall behind.

Underneath these problems sits one belief: our maintenance tooling is
software, and we should build it like software — typed, tested, versioned,
and released, not scripted, copied, and forgotten.

## What Rakko Is

Rakko turns maintenance tasks into versioned Rust crates that every project
composes into its own small command-line tool. It is the middle layer of a
three-layer architecture: [mise] provisions the external tools, Rakko
provides the actions and the machinery to run them, and [Clawless] provides
the command-line framework.

- An **action** is a library crate that implements Rakko's `Action` trait.
  Most actions are checks: they examine the working tree and report findings,
  and with `--fix`, some can repair what they find. An action depends only on
  Rakko's small contract crate — not on Clawless, not on clap.
- A **bundle** is a meta-crate that exports a list of actions — our
  definition of what "recommended" means, as a dependency. Bundles can
  contain bundles.
- The **harness** is a tiny binary crate in each project that mounts the
  bundles and actions the project wants. The harness is the project's whole
  adoption surface: a few lines that name what runs here.
- **Renovate** keeps both dimensions fresh: `mise.toml` pins the external
  tools, `Cargo.toml` pins the actions, and both get update pull requests
  like any other dependency.

## An Example

```console
$ mise run rakko format toml --fix
   Fixed  deny.toml
 Checked  14 files, fixed 1
```

What happened, layer by layer:

1. `mise run rakko` is a task in the project's `mise.toml` that builds and
   runs the harness crate. The harness is a package of its own, outside the
   workspace of the project, so nothing it depends on reaches the crates that
   the project publishes.
2. The harness mounts its actions. Rakko hands their descriptors to Clawless,
   which generates the command tree, the help text from documentation
   comments, and the shared output flags — `--quiet`, `--verbose`, `--json`.
3. `format toml` resolves to the TOML formatting action; `--fix` selects the
   fix mode instead of the default check mode.
4. The action formats via taplo as a library — no subprocess, no PATH
   lookup, no shell. Actions that wrap external tools resolve them through
   mise, so the pinned version runs no matter what shell invoked cargo.
5. Findings flow through structured output: human-readable text by default,
   JSON for machines, annotations in CI.

The harness crate that makes this possible is, in its entirety:

```rust
fn main() {
    rakko::cli()
        .mount(rakko_recommended::actions())
        .run()
}
```

Crate and function names are illustrative; the shape is the point. When a new
action joins the recommended bundle, this file does not change — the next
Renovate pull request delivers the action, and its CI run shows whether this
project already complies.

## Guardrails

These principles hold as the features arrive. Each one earns its own ADR;
this is the short form.

- **Actions are libraries, not commands.** The contract crate stays small and
  stable. The CLI is a projection that Rakko generates, not something an
  action author writes.
- **Composition is explicit data.** Bundles export plain lists of actions,
  and the harness names what it mounts. No linker magic, no dynamic plugins, no
  registration at a distance. Reading the harness tells you what runs.
- **Declare, don't detect.** An action declares what it reads and writes as a
  function of its arguments. The declaration is conservative by default —
  exclusive access to everything — and actions opt into precision. Parallel
  scheduling and sandboxing derive from declarations; they never guess.
- **Wrap well, link when possible.** Wrapping an external tool as a
  subprocess is the polished, first-class path. Consuming a tool as a Rust
  library is a per-action optimization, taken when a real API exists.
- **Self-gating actions.** An action detects whether it applies to a
  project and skips visibly when it does not. Bundles stay broad because
  actions stay honest.
- **Renovate is the rollout.** Publishing a bundle release rolls a change
  across the fleet as pull requests. A red pull request is a visible,
  per-project rollout gate — not a silent gap in coverage.

## The Long-Term Outcome

When all the sparkle is in place:

- A new project adopts the full house standard by adding mise, a marker file
  that names its root, and a three-line harness crate.
- Adding a check across the fleet means publishing one bundle release and
  merging the green pull requests that follow. The red ones are the honest
  to-do list.
- The scheduler runs actions in parallel based on their declared access —
  eventually at file-set granularity, where the TOML formatter and the
  Markdown formatter fix the tree at the same time because their declarations
  cannot overlap.
- Sandboxes enforce what actions declare, so a lying declaration is a caught
  bug instead of a race condition.
- A contributor on Windows clones any repository and everything just works,
  because no maintenance logic lives in a shell.
- Checks report through one output system: readable in a terminal, JSON in a
  pipe, annotations on a pull request.

## What Rakko Is Not

- **Not a build system.** Cargo builds; Rakko maintains.
- **Not a provisioning tool.** Mise installs tools; Rakko asks mise where
  they are.
- **Not a CI system.** CI runs the same commands a developer runs, and gets
  the same answers.
- **Not a plugin platform.** Composition happens at compile time, in code
  that names its parts. There is no dynamic loading and no registry to
  query at runtime.
- **Not a recipe language.** There is no DSL to learn. Logic is Rust, with
  everything that buys: types, tests, editors, and a debugger.

## The First Milestone

Rakko bootstraps on the tooling it exists to replace: the repository starts
with a Justfile and the familiar configs, because nothing else exists yet.
That irony defines the finish line. The MVP is complete when the Rakko
repository has fully migrated to mise and Rakko itself — its own checks run
through `mise run rakko`, and the Justfile is gone.

Until then, the repository doubles as the playground: the harness consumes new
actions as path dependencies, so every action is exercised in a real
repository the moment it compiles. The exceptions are actions for stacks this
repository does not contain — those wait for their first real adopter.

[clawless]: https://github.com/aonyx-ai/clawless
[mise]: https://mise.jdx.dev
