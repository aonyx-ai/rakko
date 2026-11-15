# `aonyx-rakko-cargo`

This crate runs [cargo] for Rakko actions and reads its workspaces,
toolchains, and diagnostics.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

Each of these actions wraps one job of cargo, such as a run of clippy, and
each asks the same questions. Does the project hold Rust code? Which cargo
does it run? Which workspaces make up the project? Which toolchain does the
job need? What did cargo report about the build? This crate answers them, so
an action writes the command line of its job and reads the answer as data.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each question and
its answer.

## Support

Aonyx built this crate for its own actions, and the crate mainly serves them.
You are welcome to use it in your own actions. The crates that Aonyx supports
for other authors are those that the section [Support] in the README of the
repository lists.

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[cargo]: https://doc.rust-lang.org/cargo/
[docs]: https://docs.rs/aonyx-rakko-cargo
[rakko]: https://rakko.aonyx.ai
[support]: https://github.com/aonyx-ai/rakko#support
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
