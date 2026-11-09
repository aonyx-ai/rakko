# Rakko TypeScript

`rakko-typescript` is the bundle that any TypeScript project mounts: the
formatter, the linter, the type checker, and the test runner. A project
subscribes to the whole set with one dependency and one line in its harness,
and a release of the bundle reaches the project as a pull request like any
other dependency.

The bundle contains no other bundle, and it does not contain the baseline, so
a TypeScript project mounts two names: this bundle beside `rakko-baseline`.

## Usage

Add `rakko-typescript` to the harness of the project, and let the `main` of the
harness mount what the bundle exports beside the baseline:

```rust
rakko_cli::builder()
    .mount(rakko_baseline::bundle())
    .mount(rakko_typescript::bundle())
    .run();
```

Each action of the bundle becomes one command of the harness, exactly as if
the harness had mounted that action itself. A harness that wants all of the
bundle but one action filters the list, because the list is an ordinary value.

## Actions

The bundle exports four actions:

- `check-typescript` type checks the project with [tsc], from the
  `tsconfig.json` at the root of the project. Tsc does not follow the
  `references` of that configuration, so a root configuration that only
  lists references, with `"files": []`, passes without a check.
- `format-typescript` formats the TypeScript and the JavaScript files with
  [oxfmt]. It leaves JSON, Markdown, and YAML to the actions of the baseline.
- `lint-typescript` examines the TypeScript and the JavaScript files with
  [oxlint].
- `test-typescript` runs the tests of the project with the [test runner] that
  Node carries.

## Tools

Rakko installs nothing, and an action whose tool [mise] does not report stops
instead of passing quietly. An action runs two kinds of tool: toolchains and
the tools of the action.

A toolchain is a compiler or runtime that the code of the project builds or
runs with. The project pins its version in its `mise.toml`, and an action
accepts a range of versions. Each action takes the first version of the `node`
or `npm:typescript` entry that its range accepts. A run stops when the range
accepts no version of the entry. For example, with
`node = ["22.12.0", "26.13.0"]`, `format-typescript` runs on 22.12.0. The
toolchains of this bundle are Node and TypeScript:

- `check-typescript` accepts `npm:typescript` 5 and later. The editor of a
  contributor uses the TypeScript in `node_modules`, so pin the version that
  the project depends on. Tsc runs on Node, and the action accepts Node 12.20
  and later, the range of TypeScript 5.0. A newer TypeScript can need a newer
  Node, such as 16.20 and later for TypeScript 7.
- `format-typescript` accepts the Node that oxfmt supports: 20.19 and later
  releases of 20, and 22.12 and later releases. So a run on a pin of Node 21
  stops.
- `test-typescript` accepts 22.18 and later releases of 22, and 23.6 and later
  releases, as the next section explains.

Every tool of the npm backend runs on Node, and mise puts no Node on the path
for such a tool. So `check-typescript` and `format-typescript` put the Node that
they resolved at the start of the path of their tool. The baseline reads the
same `node` entry, so the entry must hold a version in the range of each action
of both bundles.

The other tools belong to the actions. Each of these actions runs the version
that its crate pins in a `mise.rakko.toml`, and not the version that the
project pins. So the pin of the project must be that same version, until Rakko
writes the pin into the project itself. When that version is not installed, a
run stops, and its error gives the tool and the version:

- `oxfmt`, for `format-typescript`, which pins it itself.
- `oxlint`, for `lint-typescript`, which pins it itself.

Each tool reads its own configuration, so a run of an action agrees with the
editor of a contributor and with a run of the tool alone on what the
configuration asks. An action can ask its tool for more or less than a bare run
does, though, and the documentation of each action says what it asks.

## Tests in TypeScript

Node runs a test file in TypeScript without a build, because it strips the
types when it loads the file. It finds such files only in the releases that
strip types by default: 22.18 and later on the line of 22, and 23.6 and later.
The action was built against node 24.

Another release leaves out every test in TypeScript, and it says nothing about
it. Without a range, the action skips in a project without other tests. In a
project with tests in JavaScript, it passes on those tests alone. So
`test-typescript` accepts only the releases that strip types by default, and a
run on another pin stops.

Node only removes the types, so it refuses some code that tsc accepts. Set
these options in the `tsconfig.json` of the project, so that `check-typescript`
reports such code before a test fails on it:

- `erasableSyntaxOnly`, because Node cannot run an enum, a namespace, or a
  parameter property.
- `verbatimModuleSyntax`, because Node needs `import type` for an import that
  names only a type.
- `allowImportingTsExtensions` or `rewriteRelativeImportExtensions`, because
  Node needs the `.ts` extension in a relative import, and tsc refuses that
  extension without one of the two.

The [documentation of Node][node-typescript] lists every limit.

## Dependencies of the Project

Tsc and the tests read the npm packages that the project depends on from
`node_modules`, and Rakko does not install them. Install the dependencies of
the project before a run of `check-typescript` or `test-typescript`, with the
package manager of the project, such as `npm ci`. This applies to every job of
a CI workflow that starts from a clean checkout, and to every step that removes
`node_modules`. Without the packages, tsc reports every import of a package as
an error, and a test that imports a package fails.

[mise]: https://mise.jdx.dev
[node-typescript]: https://nodejs.org/api/typescript.html
[oxfmt]: https://oxc.rs/docs/guide/usage/formatter.html
[oxlint]: https://oxc.rs/docs/guide/usage/linter.html
[test runner]: https://nodejs.org/api/test.html
[tsc]: https://www.typescriptlang.org/docs/handbook/compiler-options.html
