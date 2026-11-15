# `aonyx-rakko-terraform`

This crate bundles the actions that Aonyx uses to format and lint Terraform
files.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

A project subscribes to both with one dependency and one line in its harness,
and a release of the bundle reaches the project as a pull request like any
other dependency.

The bundle contains no other bundle, and it does not contain the baseline. A
project with Terraform mounts this bundle beside `aonyx-rakko-baseline`.

## Usage

Add `aonyx-rakko-terraform` to the harness of the project, and let the `main`
of the harness mount what the bundle exports beside the baseline:

```rust
rakko_cli::builder()
    .mount(aonyx_rakko_baseline::bundle())
    .mount(aonyx_rakko_terraform::bundle())
    .run();
```

Each action of the bundle becomes one command of the harness, exactly as if
the harness had mounted that action itself. A harness that wants all of the
bundle but one action filters the list, because the list is an ordinary value.

## Actions

The bundle exports two actions:

- `format-terraform` formats the Terraform files with `terraform fmt`, and it
  rewrites them with `fix`.
- `lint-terraform` inspects the Terraform modules with [TFLint]. TFLint reads
  the `.tflint.hcl` file of the project.

Both actions walk the project from its root, and both skip a directory whose
name starts with a dot, such as the `.terraform` directory of a module.

## Tools

Rakko installs nothing, and an action whose tool [mise] does not report stops
instead of passing quietly. An action runs two kinds of tool: toolchains and
the tools of the action.

Terraform is a toolchain, because the configuration of the project runs on it.
The project pins its version in its `mise.toml`, and `format-terraform` accepts
Terraform 1.0 and later.

TFLint is a tool of `lint-terraform`. The action runs the version that its crate
pins in a `mise.rakko.toml`, and not the version that the project pins.
`doctor --fix` writes that pin into the generated mise file of the project. The
action installs no plugin of TFLint either, so a project that enables one in
`.tflint.hcl` runs `tflint --init` before a run.

## Documentation

The [documentation of the `bundle` function][docs] on docs.rs describes the list
of actions that the function returns.

## Support

This bundle was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[docs]: https://docs.rs/aonyx-rakko-terraform/latest/aonyx_rakko_terraform/fn.bundle.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[tflint]: https://github.com/terraform-linters/tflint
[rakko]: https://rakko.aonyx.ai
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
