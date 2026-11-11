# Rakko Format Rust

`rakko-format-rust` provides `format-rust`, an action for [Rakko] that formats
the Rust code of a project.

[Rustfmt] does the work. Cargo reads the manifests of the project, and rustfmt
reads its own configuration and formats every target. So a run of the action
agrees with an editor and with a contributor that runs rustfmt bare. The action
starts the cargo that [mise] installed for the project on the nightly toolchain
that the project pinned. It translates what rustfmt reported into the outcome of
the run.

A run reports by default and rewrites with the `fix` argument. The action skips
visibly in a project whose cargo discovers no workspace.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `rakko-format-rust` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use rakko_format_rust::FormatRust;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(FormatRust).run()
}
```

The action becomes the `format-rust` command of the harness.

## Documentation

The [documentation of `FormatRust`][docs] on docs.rs describes the tools that
the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/rakko-format-rust/latest/rakko_format_rust/struct.FormatRust.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[rustfmt]: https://github.com/rust-lang/rustfmt
