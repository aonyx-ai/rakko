# `aonyx-rakko-test-rust-docs`

This crate provides `test-rust-docs`, a Rakko action that runs the examples in
Rust documentation as tests.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

An example in a doc comment is a test. The compiler builds it against the
library that documents it, and the test harness runs it. An example that names a
function which no longer exists fails like any other test. It is also the part
of the documentation that becomes wrong without a warning, because nothing else
in a project uses the symbols of an example.

The action exists beside the action that runs the tests, because [nextest] does
not run the examples and says nothing about leaving them out. Cargo does the
work here: it builds the examples of every package of a workspace with every
feature and runs them. As a result, a run agrees with a contributor that runs
`cargo test --doc` bare. The cargo that runs is the one that [mise] installed
for the project, at the version that the project pinned.

A run only reports, and it takes no argument. An example that failed becomes a
finding at the line that documents it. A build that does not finish becomes
findings from the diagnostics of the compiler. The action skips visibly in a
project whose workspaces hold no library, because cargo can test the
documentation of a library and of nothing else.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-test-rust-docs` to
the harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_test_rust_docs::TestRustDocs;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(TestRustDocs).run()
}
```

The action becomes the `test-rust-docs` command of the harness.

## Documentation

The [documentation of `TestRustDocs`][docs] on docs.rs describes the tools that
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

[docs]: https://docs.rs/aonyx-rakko-test-rust-docs/latest/aonyx_rakko_test_rust_docs/struct.TestRustDocs.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[nextest]: https://nexte.st
[rakko]: https://rakko.aonyx.ai
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
