# `aonyx-rakko-worktree`

This crate gives Rakko actions a disposable Git checkout so that their work
can leave the original checkout unchanged.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

Some work writes files that nobody asked for: a resolution of dependencies
rewrites a lockfile, and a build writes what it generates. A check that does
such work in the checkout of a contributor leaves the tree dirty.

The copy is a Git worktree of the commit at the HEAD of the project. The crate
syncs the changed files of the checkout over it. It also removes from the copy
each file that the checkout removed. So a job that runs in the copy gives the
result for the tree of the contributor, and the checkout does not change. A
drop of the value removes the copy.

## Documentation

The [documentation of the crate][docs] on docs.rs describes the copy and what
it requires of the project.

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

[docs]: https://docs.rs/aonyx-rakko-worktree
[rakko]: https://rakko.aonyx.ai
[support]: https://github.com/aonyx-ai/rakko#support
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
