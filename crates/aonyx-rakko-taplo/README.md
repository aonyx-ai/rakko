# `aonyx-rakko-taplo`

This crate runs [taplo] for Rakko actions to lint and format TOML files.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

Taplo formats and lints TOML files, and one action wraps each of these jobs.
Each of these actions asks the same questions. Does the project hold TOML
files? Which taplo does it run? What did that taplo report? This crate answers
them, so an action gives the operation that it wants and reads the answer as
data.

The crate pins the version of taplo that every such action runs. The version
that the project pins does not change what runs.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each operation and
its report.

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

[docs]: https://docs.rs/aonyx-rakko-taplo
[rakko]: https://rakko.aonyx.ai
[support]: https://github.com/aonyx-ai/rakko#support
[taplo]: https://taplo.tamasfe.dev
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
