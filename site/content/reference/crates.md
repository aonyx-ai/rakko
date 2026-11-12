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
