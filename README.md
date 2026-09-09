# 🦦 Rakko

Rakko turns project maintenance into versioned Rust crates. Aonyx built it to
replace the Justfile recipes that every project copies, because the copies
drift. A maintenance task, such as formatting the TOML files or checking the
licenses of the dependencies, is a crate called an action. A project mounts
the actions it wants in a small binary crate, the harness, and an update to an
action arrives as a pull request, like any other dependency.

Rakko is the middle layer of three. [Mise] provisions the external tools at
pinned versions, Rakko provides the actions and the machinery that runs them,
and [Clawless] turns the mounted actions into a command line. [VISION.md]
describes where the project is going, and [GLOSSARY.md] defines its terms.

Rakko (ラッコ) is Japanese for sea otter, the otter that keeps a pebble as its
tool.

## Usage

A project adopts Rakko by writing a harness, a small package that mounts the
actions the project runs. Mise builds and runs it:

```console
mise run rakko
```

Activate mise in your shell, as the [mise documentation][mise-activate]
describes. Activation puts the pinned tools on your `PATH`, and with them the
`bin` directory of the project, which holds a stub for every mise task.
`rakko` is then the same command, from every directory of the project. On
Windows, where mise activates no native shell, `bin\rakko` from the root of
the project is that stub.

A run without a command lists the commands. Each action is one, and
`pre-commit` runs every action that guards a commit:

```console
rakko format-toml --fix
rakko pre-commit
```

This repository is the first adopter: the package in `tools/rakko` mounts the
actions that maintain Rakko itself.

## Development

[Mise] provisions every tool that this repository needs, at the versions that
`mise.toml` pins:

```console
mise install
```

The first installation builds [Tracey] from source, which takes several
minutes. Later installations reuse the binary.

`pre-commit install` installs the Git hook, which runs `rakko pre-commit
--fix` before every commit. Mise writes the stubs in `bin` once, so a change
to the tasks of `mise.toml` wants a fresh `mise generate task-stubs`.

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
