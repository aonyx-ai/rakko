# Rakko Check MSRV

`rakko-check-msrv` provides `check-msrv`, an action for [Rakko] that checks a
project against the Rust version it promises.

A package writes the oldest toolchain that it compiles on as the `rust-version`
of its manifest. Whoever depends on the package reads that as a fact. Only the
compiler can confirm the fact, so this action runs the compiler on the toolchain
that the promise names and reports what it said.

Cargo does the work: it reads the manifests of the project, and rustup selects
the toolchain. So a run agrees with a contributor who checks the same thing by
hand. The action starts the cargo that [mise] installed for the project, on the
toolchain that mise installed for the declared version, and it installs nothing.

A run only reports, and it takes no argument. Every diagnostic of the compiler
becomes a finding. So a run with a warning fails. The older compiler answers for
the promise, and a deprecation that only it reports is part of the answer. The
action applies to a project that holds a manifest of cargo and declares a Rust
version in it, and it skips visibly otherwise.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `rakko-check-msrv` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use rakko_check_msrv::CheckMsrv;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(CheckMsrv).run()
}
```

The action becomes the `check-msrv` command of the harness.

## Documentation

The [documentation of `CheckMsrv`][docs] on docs.rs describes the tools that the
action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/rakko-check-msrv/latest/rakko_check_msrv/struct.CheckMsrv.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
