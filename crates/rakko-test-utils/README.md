# Rakko Test Utils

[Rakko] turns project maintenance into versioned Rust crates, and an action is
a crate that does one maintenance task. `rakko-test-utils` helps the author of
an action test the action. It gives two kinds of help:

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

[docs]: https://docs.rs/rakko-test-utils
[rakko]: https://github.com/aonyx-ai/rakko
[support]: https://github.com/aonyx-ai/rakko#support
