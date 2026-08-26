# 🦦 Rakko

Rakko turns project maintenance into versioned Rust crates. Today, every
Aonyx project maintains itself with a copy of the same Justfile recipes and
linter configs, and the copies drift. With Rakko, maintenance tasks are
published crates called actions. Each project composes the actions it wants
into a small command-line tool, and updates arrive as pull requests.

Rakko is the middle layer of a three-layer architecture. [Mise] provisions
the external tools at pinned versions. Rakko provides the actions and the
machinery to run them. [Clawless] provides the command-line chassis. Each
project ships a tiny binary crate, the harness, that mounts its actions:

```console
$ cargo ra format toml --fix
```

Actions ship in bundles: meta-crates that define what "recommended" means
for the fleet. When a bundle release adds a new check, every project
receives a Renovate pull request, and the CI result of that pull request
shows whether the project already complies. Rollout across the fleet is a
set of merged pull requests, not a manual sweep.

Rakko is in its earliest stage, and it bootstraps on the tooling it exists
to replace. The first milestone is complete when this repository has
migrated to mise and Rakko itself, and its Justfile is gone. The rakko (ラッコ)
is the Japanese sea otter — the otter that keeps a pebble as its tool.

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
[mise]: https://mise.jdx.dev
