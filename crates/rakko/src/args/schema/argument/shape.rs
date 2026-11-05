/// The kind of value that an argument holds
///
/// A shape says what an argument reads. It never says how a user writes it,
/// because the syntax of a command line is the same for every action and
/// belongs to the projection that builds it. A projection turns a shape into
/// the syntax of its own medium, and a command line renders a boolean as a
/// switch and a text as a flag that takes a value.
///
/// The set of shapes is closed. Every projection renders every shape, so a
/// shape that a projection cannot render stops the build of that projection
/// and not a run. A new shape is therefore a breaking release of this crate,
/// and that is what the guarantee costs.
///
/// The shapes cover the input that actions read today. An argument that the
/// user repeats, and an argument that takes one of a fixed set of values,
/// have no shape yet.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ArgumentShape {
    /// A value that is true or false
    ///
    /// The user asks for a behavior or leaves it alone. An argument that
    /// tells an action to rewrite the files that it examines has this shape.
    Boolean,

    /// A whole number, with or without a sign
    ///
    /// An action reads the number into a type of its own, so this shape says
    /// nothing about the range that the action accepts.
    Integer,

    /// The path of a file or of a directory
    ///
    /// A value travels as text, so a path that is not valid UTF-8 does not
    /// reach an action.
    Path,

    /// A line of text
    Text,
}
