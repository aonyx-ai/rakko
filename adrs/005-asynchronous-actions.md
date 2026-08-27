# ADR-005: Asynchronous Actions

## Status

Accepted

## Context

[ADR-004][adr-004] made the action a library crate that implements the
`Action` trait from the contract crate. The work of an action happens in its
`run` method, and the signature of that method fixes one thing for every
action in the fleet: whether `run` blocks the thread that calls it, or
returns a future that yields while the action waits.

The work of an action is I/O. The normal integration path
[wraps][glossary] an external tool as a subprocess, so a typical run spawns
a child, streams its output, and waits for its exit; the rest reads and
writes the files of the project. A run computes little and waits much, and
an asynchronous run turns the wait into a value — a future — that the
machinery can compose. The planned scheduler is that machinery: it runs
actions in parallel under a conflict graph, and it wants to start many runs
at once, impose deadlines on them, and abandon the runs that still wait
when an earlier one has already decided the result.

The choice is also close to irreversible, and the two possible mistakes cost
different amounts. The contract crate reaches every action and every harness,
so flipping the signature of `run` later is the fleet-wide major migration
that ADR-004 warns about. A synchronous body runs unchanged inside an
asynchronous `run`: it simply never yields. An asynchronous body inside a
synchronous `run` needs a runtime bridged in from the outside, one blocking
adapter per action. Choosing async without needing it costs little; choosing
sync and needing async later costs the migration.

## Decision

`run` is asynchronous. Three details give the decision its exact shape.

1. **The trait spells the return type out.** The trait declares `run` as
   returning `impl Future<Output = Outcome> + Send`, not with the `async fn`
   keyword. An implementer still writes `async fn run`; the two forms are
   interchangeable on the implementing side. The explicit form exists for the
   `Send` bound: a plain `async fn` in a trait gives the caller no way to
   require that the returned future can move between threads, and stable
   Rust has no syntax to add that requirement at the call site. The
   scheduler moves futures between threads, so the bound must live in the
   trait.

2. **Actions are thread-safe.** The trait requires `Send + Sync` of the
   action and of its `Args`. A run that crosses threads needs both, and a
   bound that arrives later is a breaking change for every action. Stating
   the bounds as supertraits also moves the compile error: an action that
   holds thread-bound state fails at its own `impl` block, not in the
   distant generic machinery of a scheduler.

3. **The contract names no runtime.** The trait uses `Future` from the
   standard library and nothing else. Which runtime drives the futures is
   the choice of the harness, outside the dependency tree of the contract
   crate, exactly as Clawless stays outside it.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### A Synchronous run Method

`run` can block, and the scheduler can run actions on threads. This is the
simplest model, and for plain parallelism it would suffice: a thread waits on
a child process as well as a future does. Two forces rule it out. First,
cancellation: a dropped future stops at its next yield point, while a thread
cannot be stopped from the outside, so deadlines and fail-fast behavior would
need cooperation from every action instead of coming from the machinery.
Second, the asymmetry from the context: synchronous work inside an
asynchronous signature is free, the reverse is a bridge in every action, so
the synchronous signature is the one we would regret.

### A Plain async fn in the Trait

The declaration `async fn run(...) -> Outcome` reads better than the
explicit return type and compiles to the same thing — minus the `Send`
bound. The caller of a plain `async fn` trait method cannot require the
future to be `Send`, and stable Rust offers no syntax at the call site to
add the requirement later. The scheduler needs the bound, so the trait must
carry it, and only the explicit return type can.

### The async-trait Crate

The [async-trait][async-trait] macro rewrites the trait to return boxed
futures, which reads naturally and makes the trait dyn-compatible for free.
It is also a dependency in the crate whose churn radius is the whole fleet,
an allocation on every call whether needed or not, and a macro between the
reader and the signature. ADR-004 set the bar: everything that can live
elsewhere does. Boxing can live at the erasure boundary.

### A Boxed Future in the Signature

`run` can return a pinned, boxed future directly, which makes the trait
dyn-compatible without a separate erased view. But ADR-004 wrote the
`Action` trait for action authors, and this signature taxes every one of
them with wrapping boilerplate that the erased action already pays once, at
the boundary where erasure happens. The registry gains nothing: it holds
erased actions either way.

## Consequences

- Every harness carries an async runtime. The contract crate names none, but
  actions that spawn subprocesses will reach for the process API of a
  concrete runtime, so the fleet converges on one in practice. This
  convergence is real coupling that no manifest of the contract crate shows;
  it just does not cascade through the version of the contract crate.
- A purely synchronous action writes `async fn run` with no `await` inside
  and pays nothing at run time.
- An action that blocks — heavy computation, blocking I/O — stalls a runtime
  worker. The contract cannot detect this. Like the thinness of `rakko-cli`,
  it is a discipline that review must hold.
- Cancellation is structural but shallow: dropping a future stops the action
  at its next yield point, yet a child process it spawned keeps running.
  Killing children on cancel is machinery polish that this decision enables
  but does not deliver.
- The `Send + Sync` bounds rule out single-threaded state such as `Rc` and
  `RefCell` in every action, and the compiler reports the conflict at the
  `impl` block of the action.
- Tests stay light. A future that never yields resolves on its first poll,
  so the contract crate tests its trait with a no-op waker from the standard
  library and no runtime. Action crates test with the harness of their
  runtime.
- The signature of `run` in the trait is unusual to read, while every
  implementation of it is still written as `async fn`. The documentation of
  the trait must show this, or every first-time author stumbles over it.

[adr-004]: 004-actions-as-libraries.md
[async-trait]: https://crates.io/crates/async-trait
[glossary]: ../GLOSSARY.md
