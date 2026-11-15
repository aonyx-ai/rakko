# `rakko-test-utils`

This crate helps authors test their Rakko actions and their declarations of
access to project files.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

The crate gives two kinds of help:

- A test writes a path with a forward slash between the parts, and the crate
  builds the path that the platform uses. So one literal in a test describes
  the same file on every platform.
- An action declares whether a run reads or writes the project. A test runs
  the action through the crate, and the test fails when a run that declares a
  read changed a file of the project. So the test also checks the declaration.

## Usage

Add the crate as a development dependency of the crate of the action:

```console
cargo add --dev rakko-test-utils
```

Use a release whose dependency on `rakko` is compatible with the release of
`rakko` that the action uses. Otherwise the action does not implement the
`Action` trait that the helpers expect, and the tests do not compile.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each helper.

## Support

Aonyx uses Rakko internally, and we maintain and support this crate. The
section [Support] in the README of the repository lists the crates that Aonyx
supports for other authors.

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[docs]: https://docs.rs/rakko-test-utils
[rakko]: https://rakko.aonyx.ai
[support]: https://github.com/aonyx-ai/rakko#support
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
