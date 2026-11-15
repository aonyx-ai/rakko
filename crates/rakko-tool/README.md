# `rakko-tool`

This crate finds and runs the external tools of Rakko actions through mise.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

An action does the work of an external tool, such as a formatter or a linter,
and it starts that tool as a subprocess. `rakko-tool` finds the tool, and it
describes the command that starts the tool. So every action reaches its tool
in the same way.

[Mise] tells the crate where a tool is. The crate searches no path of its own
and installs nothing. An action declares each tool that it runs as a
requirement. A tool that the action wraps resolves at the exact version that
the action pins. So a run starts the version that the tests of the action
cover. A toolchain, such as Rust or Node, resolves at the version that the
project pins, when the range of the action accepts it.

## Documentation

The [documentation of the crate][docs] on docs.rs describes how an action
declares, finds, and starts its tools.

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

[docs]: https://docs.rs/rakko-tool
[mise]: https://mise.jdx.dev
[rakko]: https://rakko.aonyx.ai
[support]: https://github.com/aonyx-ai/rakko#support
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
