# `aonyx-rakko-format-rust`

This crate provides `format-rust`, a Rakko action that formats Rust code with
rustfmt.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

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
it mounts the actions that the project uses. Add `aonyx-rakko-format-rust` to
the harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_format_rust::FormatRust;

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

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[docs]: https://docs.rs/aonyx-rakko-format-rust/latest/aonyx_rakko_format_rust/struct.FormatRust.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[rakko]: https://rakko.aonyx.ai
[rustfmt]: https://github.com/rust-lang/rustfmt
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
