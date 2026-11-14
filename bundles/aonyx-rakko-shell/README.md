# Aonyx Rakko Shell

`aonyx-rakko-shell` is the bundle that any project with shell scripts mounts:
the formatter and the linter. A project subscribes to both with one dependency
and one line in its harness, and a release of the bundle reaches the project as
a pull request like any other dependency.

The bundle contains no other bundle, and it does not contain the baseline. A
project with shell scripts mounts this bundle beside `aonyx-rakko-baseline`
and the bundle of its language.

## Usage

Add `aonyx-rakko-shell` to the harness of the project, and let the `main` of
the harness mount what the bundle exports beside the baseline:

```rust
rakko_cli::builder()
    .mount(aonyx_rakko_baseline::bundle())
    .mount(aonyx_rakko_shell::bundle())
    .run();
```

Each action of the bundle becomes one command of the harness, exactly as if
the harness had mounted that action itself. A harness that wants all of the
bundle but one action filters the list, because the list is an ordinary value.

## Actions

The bundle exports two actions:

- `format-shell` formats the shell scripts with [shfmt], and it rewrites them
  with `fix`. Shfmt reads its options from the `.editorconfig` files of the
  project.
- `lint-shell` examines the shell scripts with [ShellCheck]. ShellCheck reads
  the `.shellcheckrc` files of the project.

Both actions find the scripts with shfmt, by their extension and by their
shebang. So they examine the same scripts, and an `.editorconfig` section with
`ignore = true` keeps a script away from both. A project without a shell
script skips both actions.

## Tools

Rakko installs nothing, and an action whose tool [mise] does not report stops
instead of passing quietly. Each action runs the version that its crate pins in
a `mise.rakko.toml`, and not the version that the project pins. `doctor --fix`
writes those pins into the generated mise file of the project:

- `shfmt`, for both actions.
- `shellcheck`, for `lint-shell`.

## Documentation

The [documentation of the `bundle` function][docs] on docs.rs describes the list
of actions that the function returns.

## Support

This bundle was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-shell/latest/aonyx_rakko_shell/fn.bundle.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[shellcheck]: https://www.shellcheck.net
[shfmt]: https://github.com/mvdan/sh
