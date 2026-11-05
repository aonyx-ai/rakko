/// How a run of an action uses the project
///
/// An action declares its access after its arguments are bound and before it
/// runs. The answer can therefore depend on the arguments of the run: a
/// formatter writes only when a run asks it to repair what it finds. A caller
/// that drives several actions can read the declarations to order them. For
/// example, it can run the actions that write before the actions that read.
///
/// Both answers cover the whole project. A run writes only when it changes
/// files that another action could read or change. These changes do not
/// count as a write:
///
/// - a change that only the tool of the action reads, such as a
///   `.tsbuildinfo` file or a `.tracey/` directory,
/// - a change inside `.git/`, and
/// - a change in a directory that the tool locks against parallel use
///   itself, such as the `target/` directory of cargo.
///
/// The declaration covers the action and its tool. It does not cover the
/// code of the project that the tool runs. So an action that runs the tests
/// of the project reads.
///
/// Nothing enforces the declaration yet. An action can declare a read and
/// change files anyway. Then it can change what another action reads at the
/// same time.
///
/// A later release can add a kind of access. So a match on `Access` outside
/// this crate needs an arm for a kind that it does not know. A caller outside
/// this crate that runs a [`BoundAction`] asks for its access first, also
/// when it runs only one action. A caller that meets a kind that it does not
/// know must not run the action, because the caller does not know what the
/// kind asks of the run. If the caller runs the action anyway, the run can
/// change the project. The caller stops with an error instead, and the error
/// tells the user to update to a release that knows the kind. A caller that
/// orders several actions by their declarations stops before it runs the
/// first of them.
///
/// [`BoundAction`]: crate::BoundAction
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum Access {
    /// The run reads the project and changes no file of it
    ///
    /// A read is shared: other actions that read can run at the same time.
    Read,
    /// The run can change files of the project
    ///
    /// A write is exclusive: no other action reads or writes the project
    /// during the run. This is the answer of an action that declares nothing,
    /// and the cautious answer when the arguments alone do not settle it.
    Write,
}
