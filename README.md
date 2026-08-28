# 🦦 Rakko

Rakko turns project maintenance into versioned Rust crates. Today, every
Aonyx project maintains itself with a copy of the same Justfile recipes and
linter configs, and the copies drift. With Rakko, maintenance tasks are
published crates called actions. Each project composes the actions it wants
into a small command-line tool, and updates arrive as pull requests.

Rakko is the middle layer of a three-layer architecture. [Mise] provisions
the external tools at pinned versions. Rakko provides the actions and the
machinery to run them. [Clawless] provides the command-line framework. Each
project ships a tiny binary crate, the harness, that mounts its actions:

```console
cargo ra format toml --fix
```

Actions ship in bundles: meta-crates that define what "recommended" means
for the fleet. When a bundle release adds a new check, every project
receives a Renovate pull request, and the CI result of that pull request
shows whether the project already complies. Rollout across the fleet is a
set of merged pull requests, not a manual sweep.

Rakko is in its earliest stage, and it bootstraps on the tooling it exists
to replace. The first milestone is complete when this repository has
migrated to mise and Rakko itself, and its Justfile is gone. The long-term
picture is in [VISION.md], and the terms of the design are in [GLOSSARY.md].
The rakko (ラッコ) is the Japanese sea otter — the otter that keeps a pebble
as its tool.

## Development

[Mise] provisions every tool that this repository needs. `mise.toml` pins the
version of each tool, and mise installs them:

```console
mise install
```

The installation ends with a build of [Tracey] from source, which takes
several minutes. Later installations reuse that binary.

Activate mise in your shell, as the [mise documentation][mise-activate]
describes. The activation puts the pinned tools on your `PATH`, where your
editor and your scripts find them. Without activation, reach a tool through
`mise exec`:

```console
mise exec -- just pre-commit
```

A recipe in the justfile enters the mise environment itself, so a recipe always
runs the pinned version of a tool. The Git hook does the same, and it therefore
needs only mise on your `PATH`. `pre-commit install` installs that hook, `just`
lists the recipes, and `just pre-commit` runs the checks that the hook runs.

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT)
  or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[clawless]: https://github.com/aonyx-ai/clawless
[glossary.md]: ./GLOSSARY.md
[mise]: https://mise.jdx.dev
[mise-activate]: https://mise.jdx.dev/getting-started.html#activate-mise
[tracey]: https://tracey.bearcove.eu/
[vision.md]: ./VISION.md
