# Aonyx Rakko Format Terraform

`aonyx-rakko-format-terraform` provides `format-terraform`, an action for
[Rakko] that formats the Terraform files of a project.

[Terraform] does the work with `terraform fmt`. It walks the project from its
root and formats every Terraform file, such as a `.tf` or a `.tfvars` file. It
skips a directory whose name starts with a dot, such as the `.terraform`
directory of a module. Terraform takes no configuration for its formatter, so
every project that runs the same version formats in the same way.

Terraform is a toolchain of the project. The action runs the Terraform that the
project pins in its mise configuration, which [mise] installed, and it accepts
Terraform 1.0 and later. It translates the list of files that Terraform
reported into the outcome of the run.

A run reports by default and rewrites with the `fix` argument. A file that
Terraform cannot parse stops the run, because Terraform stops at that file and
the files after it stay unread.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-format-terraform`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_format_terraform::FormatTerraform;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(FormatTerraform).run()
}
```

The action becomes the `format-terraform` command of the harness.

## Documentation

The [documentation of `FormatTerraform`][docs] on docs.rs describes the
toolchain that the action runs, what a run reports, and when a run stops with
an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-format-terraform/latest/aonyx_rakko_format_terraform/struct.FormatTerraform.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[terraform]: https://developer.hashicorp.com/terraform/cli/commands/fmt
