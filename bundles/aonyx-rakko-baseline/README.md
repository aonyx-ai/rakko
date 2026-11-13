# Aonyx Rakko Baseline

`aonyx-rakko-baseline` is the bundle that any project mounts: the actions that
format and examine the files that a project holds, whatever language it is
written in. A project subscribes to the whole set with one dependency and one
line in its harness, and a release of the bundle reaches the project as a pull
request like any other dependency.

The bundle contains no other bundle, and no bundle contains it. A project
mounts it alone, or beside the bundle of its language, such as a Rust bundle or
`aonyx-rakko-typescript`.

## Usage

Add `aonyx-rakko-baseline` to the harness of the project, and let the `main` of
the harness mount what the bundle exports:

```rust
fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(aonyx_rakko_baseline::bundle()).run()
}
```

Each action of the bundle becomes one command of the harness, exactly as if
the harness had mounted that action itself. A harness that wants all of the
bundle but one action filters the list, because the list is an ordinary value.

## Actions

The bundle exports ten actions:

- `check-mise-lock` checks that `mise lock` would leave the lockfile of [mise]
  as it is, so that the lockfile holds an entry for each tool on each platform
  that the project names, and an install in locked mode does not stop on a new
  machine.
- `check-renovate-config` checks the configuration of [Renovate] with
  [renovate-config-validator], the validator that Renovate ships.
- `format-json` formats the JSON files with [prettier].
- `format-markdown` formats the Markdown files with [prettier].
- `format-toml` formats the TOML files with [taplo].
- `format-yaml` formats the YAML files with [prettier].
- `lint-github-actions` examines the workflows of GitHub Actions with
  [zizmor].
- `lint-markdown` examines the Markdown files with [markdownlint].
- `lint-toml` examines the TOML files with [taplo].
- `lint-yaml` examines the YAML files with [yamllint].

A preset of Renovate is a configuration that other configurations extend.
Renovate reads a preset such as `github>owner/repository` from the
`default.json` in the root of that repository. The validator does not find
this file by itself, so `check-renovate-config` also validates a
`default.json` in the root of the project. It does this for every such file,
also for one that belongs to another tool. The action checks the file as the
configuration of a repository, which is stricter than the check of a preset
in Renovate.

The validator of Renovate also reads the global configuration that a
self-hosted Renovate starts with: `config.js` in the root of the project, or
the file that `RENOVATE_CONFIG_FILE` names. A project whose `config.js` is not
a configuration of Renovate can therefore fail or stop `check-renovate-config`.

`check-mise-lock` checks the platforms that the setting `lockfile_platforms`
names in a configuration file of mise of the project, such as `mise.toml`, and
the platform of the machine that runs it. Each project names its own
platforms: those of its CI jobs and those of the machines of its contributors.
The action cannot see the workflows of a project, so a pull request that adds
a platform to CI adds it to `lockfile_platforms` too. Without the setting, mise
checks the platforms that the lockfile already records.

`check-mise-lock` fails on each entry that `mise lock` would add, change, or
remove, such as an entry of a tool that the project no longer pins. A project
that pins tools and has no lockfile therefore fails on every entry. Such a
project creates its lockfile with `check-mise-lock --fix`, which writes what
`mise lock` locks into the project and reports each repair.

## Tools

Rakko installs nothing, and an action whose tool [mise] does not report stops
instead of passing quietly. An action runs two kinds of tool: toolchains and
the tools of the action.

A toolchain is a compiler or runtime that the code of the project builds or
runs with. The project pins its version, and an action accepts a range of
versions. Node is the toolchain of this bundle. Every tool of the npm backend
runs on Node, and mise puts no Node on the path for such a tool. So an action
puts the Node that it resolved first on the path of its tool. The project pins
Node in its `mise.toml`. Each action takes the first version of the `node`
entry that its range accepts. A run stops when the range accepts no version of
the entry. These actions need Node:

- The three actions that format with prettier accept Node 14 and later.
- `lint-markdown` accepts Node 22 and later.
- `check-renovate-config` accepts the Node that the package of Renovate
  supports: 24.11 and later releases of 24, and 26.13 and later releases of 26.

For example, with `node = ["22.12.0", "26.13.0"]`, the prettier actions and
`lint-markdown` run on 22.12.0. `check-renovate-config` runs on 26.13.0. A run
of `check-renovate-config` on a pin of Node 25 stops.

The other tools belong to the actions. Each of these actions runs the version
that its crate pins in a `mise.rakko.toml`, and not the version that the
project pins. So the pin of the project must be that same version, until Rakko
writes the pin into the project itself. When that version is not installed, a
run stops, and its error gives the tool and the version:

- `npm:markdownlint-cli`, for `lint-markdown`, which pins it itself.
- `npm:prettier`, for the three actions that format with prettier. The crate
  `aonyx-rakko-prettier` pins it.
- `npm:renovate`, for `check-renovate-config`, which pins it itself.
- `taplo`, for `format-toml` and `lint-toml`. The crate `aonyx-rakko-taplo` pins
  it.
- `yamllint`, for `lint-yaml`, which pins it itself.
- `zizmor`, for `lint-github-actions`, which pins it itself.

`check-mise-lock` runs mise itself, so it needs no pin. `mise lock` asks the
API of GitHub for the downloads of the tools of the `aqua` and `github`
backends, though, so the action needs the network. A token of GitHub, such as
`GITHUB_TOKEN` or a login of `gh`, keeps mise below the rate limit of GitHub.
A run that reaches the limit stops with the warning of mise.

Renovate publishes several releases a day, so the pin of `npm:renovate` would
bring several update pull requests a day. The shared preset
[`aonyx-ai/renovate-config`][renovate-config] moves that pin once a week. A
project that does not extend the preset adds the same rule to its own
configuration of Renovate:

```json
{
  "packageRules": [
    {
      "matchDatasources": ["npm"],
      "matchPackageNames": ["renovate"],
      "extends": ["schedule:weekly"],
      "updateNotScheduled": false
    }
  ]
}
```

Each tool reads its own configuration, so a run of an action agrees with the
editor of a contributor and with a run of the tool alone on what the
configuration asks. An action can ask its tool for more than a bare run does,
though, and the documentation of each action says what it asks.

## Documentation

The [documentation of the `bundle` function][docs] on docs.rs describes the list
of actions that the function returns.

## Support

This bundle was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-baseline/latest/aonyx_rakko_baseline/fn.bundle.html
[guide]: https://github.com/aonyx-ai/rakko#support
[markdownlint]: https://github.com/DavidAnson/markdownlint
[mise]: https://mise.jdx.dev
[prettier]: https://prettier.io
[renovate]: https://docs.renovatebot.com
[renovate-config]: https://github.com/aonyx-ai/renovate-config
[renovate-config-validator]: https://docs.renovatebot.com/config-validation/
[taplo]: https://taplo.tamasfe.dev
[yamllint]: https://github.com/adrienverge/yamllint
[zizmor]: https://docs.zizmor.sh
