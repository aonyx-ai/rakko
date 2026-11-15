# `aonyx-rakko-lint-terraform`

This crate provides `lint-terraform`, a Rakko action that lints Terraform
modules with TFLint.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

[TFLint] does the work. It walks the project from its root and inspects every
directory that holds Terraform files. It skips a directory whose name starts
with a dot, such as the `.terraform` directory of a module. TFLint reads the
`.tflint.hcl` file of the project, so a run agrees with a contributor that runs
`tflint --recursive` bare. The action declares the version of TFLint that it
runs. It starts that version, which [mise] installed, and translates the report
of TFLint into the outcome of the run.

A run only reports. Each issue of TFLint is a finding over the range that
TFLint gave, with its severity and the name of its rule. An issue of any
severity fails the run, as it fails a bare run of TFLint. A module that TFLint
cannot load stops the run.

The action installs nothing, and that includes the plugins that `.tflint.hcl`
enables. Run `tflint --init` to install them before a run.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-lint-terraform`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_lint_terraform::LintTerraform;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(LintTerraform).run()
}
```

The action becomes the `lint-terraform` command of the harness.

## Documentation

The [documentation of `LintTerraform`][docs] on docs.rs describes the tool that
the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
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

[docs]: https://docs.rs/aonyx-rakko-lint-terraform/latest/aonyx_rakko_lint_terraform/struct.LintTerraform.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[rakko]: https://rakko.aonyx.ai
[tflint]: https://github.com/terraform-linters/tflint
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
