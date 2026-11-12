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

## Support

Rakko is built so that anyone can write actions for their own workflows.
Aonyx uses Rakko internally, and we maintain and support the crates that you
build on:

- `rakko`, which holds the contract for actions and groups
- `rakko-cli`, which turns your harness into a command line
- `rakko-tool`, which finds a tool through mise and runs it
- `rakko-test-utils`, which helps you test your action

The [guide] shows how to build your first action. When you publish it, give its
repository the GitHub topic `rakko-action` and its crate the crates.io keyword
`rakko-action`, so that others can find it. A new action lives in a crate of
its own, with the people who use it. So this repository does not take new
actions from other authors.

The actions, bundles, and commands in this repository are purpose-built for
the workflows of Aonyx. You are welcome to use them, and we keep them focused
on how Aonyx works. If one does not fit your needs, we encourage you to create
your own. When we stop using an action, we say so in its README.

## Development

[Mise] provisions every tool that this repository needs. `mise.toml` pins the
toolchains and the tools that no action declares. Each action pins the tools
that it runs, and the harness writes those pins into
`.config/mise/conf.d/rakko.toml`. Mise reads that file beside `mise.toml`.
Install the tools:

```console
mise install
```

After you change the mounted actions or the pin of an action, write
`.config/mise/conf.d/rakko.toml` and `.config/mise/mise.lock` again. Until
`rakko.toml` matches the tools of the mounted actions, every command of the
harness stops. Only the help, the list of the commands, and `doctor` still
run. This command writes both files:

```console
mise run rakko -- doctor --fix
```

`pre-commit install` installs the Git hook, which runs `rakko pre-commit
--fix` before every commit. Mise writes the stubs in `bin` once, so a change
to the tasks of `mise.toml` wants a fresh `mise generate task-stubs`.

The website lives in `site/`, and [Zola] builds it into `site/public`. The
Rust code on its pages comes from the example crates in `site/examples/`.
These crates are members of the workspace, so the checks of the workspace
compile and test them. A page links to another page with a link that starts
with `@/`, such as `@/how-to/build-an-action.md`, because the build fails on
such a link when its page or its anchor does not exist. This command builds
the site:

```console
mise run rakko -- build-site
```

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT)
  or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

We welcome fixes to the crates in this repository. We take a fix to an action,
a bundle, or a command when the fix fits the workflows of Aonyx.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[clawless]: https://github.com/aonyx-ai/clawless
[glossary.md]: ./GLOSSARY.md
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[mise-activate]: https://mise.jdx.dev/getting-started.html#activate-mise
[vision.md]: ./VISION.md
[zola]: https://www.getzola.org
