/// Whether cargo can test the examples in the documentation of a workspace
///
/// Cargo runs the examples in the documentation of a library, and a workspace
/// can hold no library at all: the harness of a project is a binary, and a
/// library that builds a C library links no example. Cargo refuses a run that
/// asks for the documentation examples of such a workspace, so a root states
/// which of the two it is, and a caller runs its job where the examples can
/// be tested. Whether they were is another question, which the run that
/// tested them answers.
///
/// The `doctest` flag of a manifest decides something else. It selects
/// whether a plain run of the tests includes the examples, and a run that
/// names the documentation itself tests them either way.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Documentation {
    /// The workspace holds a library whose examples cargo can test
    Testable,

    /// The workspace holds no library whose examples cargo can test
    Untestable,
}
