# `rakko-macros`

This crate provides the derive macros that the `rakko` crate exports.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

A command and a group are other parts that a harness mounts. The `Documented`
macro takes the description of an action, a command, or a group from the first
paragraph of its doc comment.

The [`rakko`][rakko-crate] crate exports every macro of this crate again, and
the code that a macro writes refers to items of `rakko`. So an action, a
command, or a group depends on `rakko`, and never on this crate.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each macro.

## Support

Use this crate through `rakko`. The section [Support] in the README of the
repository lists `rakko` among the crates that Aonyx supports for other
authors.

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[docs]: https://docs.rs/rakko-macros
[rakko]: https://rakko.aonyx.ai
[rakko-crate]: https://crates.io/crates/rakko
[support]: https://github.com/aonyx-ai/rakko#support
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
