# Rakko Baseline

`rakko-baseline` is the bundle that any project mounts: the formatters and the
linters for the files that a repository holds, whatever language it is written
in. A project subscribes to the whole set with one dependency and one line in
its harness, and a release of the bundle reaches the project as a pull request
like any other dependency.

The bundle contains no other bundle, and no bundle contains it. A project
without Rust mounts it alone, and a Rust project mounts it beside a Rust
bundle.

## Usage

Add `rakko-baseline` to the harness of the project, and let the `main` of the
harness mount what the bundle exports:

```rust
rakko_cli::builder().mount(rakko_baseline::bundle()).run();
```

Each action of the bundle becomes one command of the harness, exactly as if
the harness had mounted that action itself. A harness that wants all of the
bundle but one action filters the list, because the list is an ordinary value.

## Actions

The bundle exports eight actions:

- `format-json` formats the JSON files with [prettier].
- `format-markdown` formats the Markdown files with [prettier].
- `format-toml` formats the TOML files with [taplo].
- `format-yaml` formats the YAML files with [prettier].
- `lint-github-actions` examines the workflows of GitHub Actions with
  [zizmor].
- `lint-markdown` examines the Markdown files with [markdownlint].
- `lint-toml` examines the TOML files with [taplo].
- `lint-yaml` examines the YAML files with [yamllint].

## Tools

Each action runs the tool that [mise] installed for the project, at the
version that the project pinned. Rakko installs nothing, and an action whose
tool mise does not report stops instead of passing quietly, so a project that
mounts this bundle pins these tools in its `mise.toml`:

- `node`, because prettier runs on it and mise puts no node on the path for a
  tool of its npm backend.
- `npm:markdownlint-cli`, for `lint-markdown`.
- `npm:prettier`, for the three actions that format with prettier.
- `taplo`, for `format-toml` and `lint-toml`.
- `yamllint`, for `lint-yaml`.
- `zizmor`, for `lint-github-actions`.

Each tool reads its own configuration, so a run of an action agrees with the
editor of a contributor and with a run of the tool alone.

[markdownlint]: https://github.com/DavidAnson/markdownlint
[mise]: https://mise.jdx.dev
[prettier]: https://prettier.io
[taplo]: https://taplo.tamasfe.dev
[yamllint]: https://github.com/adrienverge/yamllint
[zizmor]: https://docs.zizmor.sh
