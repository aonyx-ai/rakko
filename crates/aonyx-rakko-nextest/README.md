# `aonyx-rakko-nextest`

This crate runs [cargo-nextest][nextest] for Rakko actions and turns its test
reports into findings.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

Each of these actions asks the same questions. Which command does a run write?
What did nextest and cargo report? This crate answers them, so an action gives
the workspace that it wants tested and reads the answer as data.

The crate pins the version of nextest that every such action runs. The version
that the project pins does not change what runs.

## Documentation

The [documentation of the crate][docs] on docs.rs describes a run and its
report.

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

[docs]: https://docs.rs/aonyx-rakko-nextest
[nextest]: https://nexte.st
[rakko]: https://rakko.aonyx.ai
[support]: https://github.com/aonyx-ai/rakko#support
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
