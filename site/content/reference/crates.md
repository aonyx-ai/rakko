+++
title = "Reference"
description = "Find the four supported crates and look up commands in your harness."
weight = 10
+++

## The Crates

Aonyx maintains and supports four crates for action and harness authors.
Each crate has API documentation on docs.rs for each released version.

| Crate              | What it gives you                                       | Documentation                                |
| ------------------ | ------------------------------------------------------- | -------------------------------------------- |
| `rakko`            | The contract for actions and groups                     | [docs.rs/rakko][rakko]                       |
| `rakko-cli`        | The builder that turns your harness into a command line | [docs.rs/rakko-cli][rakko-cli]               |
| `rakko-tool`       | Finds a tool through [mise] and runs it                 | [docs.rs/rakko-tool][rakko-tool]             |
| `rakko-test-utils` | Helpers for the tests of your action                    | [docs.rs/rakko-test-utils][rakko-test-utils] |

Aonyx also publishes the actions, bundles, and custom commands that it built for
its own workflows, and the helper crates that only those crates use. Their names
start with `aonyx-rakko-`, such as `aonyx-rakko-lint-rust`, so that you can see
that they are not among the four crates above. The four crates above have short
names that start with `rakko`, and so does `rakko-macros`, which `rakko` uses
for its derive macros. You are welcome to use the `aonyx-rakko-` crates, and
they stay focused on the workflows of Aonyx.

Anyone can publish a crate whose name starts with `aonyx-rakko-` or with
`rakko`. Aonyx publishes its crates with these names from the [Rakko
repository][repository]. A crate with such a name that is not in the Rakko
repository is probably not a crate of Aonyx.

The crate of your own action can have any name. If you want, you can put the
name of your organization and then `rakko` in front of it, such as
`otter-rakko-check-readme`.

## The Command Line

Your harness defines its own set of commands. Use its help to list the
available commands or see the flags for one command. For example, if your
harness mounts `format-rust`, you can read its help with:

```sh
mise run rakko -- --help
mise run rakko -- format-rust --help
```

[mise]: https://mise.jdx.dev
[rakko]: https://docs.rs/rakko
[rakko-cli]: https://docs.rs/rakko-cli
[rakko-test-utils]: https://docs.rs/rakko-test-utils
[rakko-tool]: https://docs.rs/rakko-tool
[repository]: https://github.com/aonyx-ai/rakko
