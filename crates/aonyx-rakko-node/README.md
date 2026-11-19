# `aonyx-rakko-node`

This crate runs tools for Rakko actions on the [Node] that the project pins.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

Many tools of project maintenance are packages for Node, such as prettier and
the compiler of TypeScript. Such a tool starts the first Node that it finds on
the path. That Node can be another release than the one that the project pins.
This crate finds the program of the tool. It also finds the Node that the
project pins. It puts the directory of that Node first on the path of the
program. So the tool runs on the pinned Node, and an action gives only what is
special about its tool.

Node is a toolchain of the project, so the project pins it. The crate pins no
tools and requires no range of Node.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each lookup and how
it fails.

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

[docs]: https://docs.rs/aonyx-rakko-node
[node]: https://nodejs.org
[rakko]: https://rakko.aonyx.ai
[support]: https://github.com/aonyx-ai/rakko#support
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
