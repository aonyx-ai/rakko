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

Each action runs the tool that [mise] installed for the project, at the
version that the project pinned. Rakko installs nothing, and an action whose
tool mise does not report stops instead of passing quietly, so a project that
mounts this bundle pins these tools in its `mise.toml`:

- `node`, for `test-typescript`, and because tsc and oxfmt run on it. Mise
  puts no node on the path for them. The baseline runs on the same node, so
  the pin must also be a version that the tools of the baseline support.
- `npm:typescript`, for `check-typescript`. The editor of a contributor uses
  the TypeScript in `node_modules`, so pin the version that the project
  depends on.
- `oxfmt`, for `format-typescript`.
- `oxlint`, for `lint-typescript`.

Each tool reads its own configuration, so a run of an action agrees with the
editor of a contributor and with a run of the tool alone on what the
configuration asks. An action can ask its tool for more or less than a bare run
does, though, and the documentation of each action says what it asks.

## Tests in TypeScript

Node runs a test file in TypeScript without a build, because it strips the
types when it loads the file. It finds such files only in the releases that
strip types by default: 22.18 and later on the line of 22, and 23.6 and later.
The action was built against node 24.

An older node leaves out every test in TypeScript, and it says nothing about
it. A project without other tests sees the action skip. A project with tests
in JavaScript passes on those tests alone.

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
