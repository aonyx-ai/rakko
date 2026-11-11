//! The trait that gives the description of a type

use crate::description::Description;

/// A type that describes itself in one sentence
///
/// Every action, every command, and every group implements the trait,
/// because the trait of each of them requires it. A projection reads the
/// description without a run, for example to show the sentence beside the
/// name of the command in its help.
///
/// Derive the trait, and do not write it by hand. The derive takes the
/// description from the first paragraph of the doc comment of the type. The
/// first paragraph ends at the first blank line. The derive puts the lines of
/// that paragraph on one line. So the sentence exists once,
/// and the help and the documentation of the crate cannot differ. The build
/// fails when the type has no doc comment.
///
/// Write the first paragraph as one sentence that a reader understands
/// without the rest. Use no Markdown and no link in it, because a terminal
/// shows the sentence as it is.
///
/// # Examples
///
/// ```
/// use rakko::Documented;
///
/// /// Counts the lines of the project
/// ///
/// /// The count leaves out the files that Git ignores.
/// #[derive(Documented)]
/// struct LineCount;
///
/// assert_eq!(LineCount.description().get(), "Counts the lines of the project");
/// ```
///
/// A type without a doc comment does not compile:
///
/// ```compile_fail
/// use rakko::Documented;
///
/// #[derive(Documented)]
/// struct LineCount;
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no description",
    label = "this type does not implement `Documented`",
    note = "add `#[derive(Documented)]` and a doc comment to the type"
)]
pub trait Documented {
    /// Returns the sentence that describes the type
    fn description(&self) -> Description;
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use crate::Documented;

    /// Counts the lines of the project
    ///
    /// The count leaves out the files that Git ignores.
    #[derive(Documented)]
    struct LineCount;

    /// Runs the action that it wraps
    #[derive(Documented)]
    struct Wrapper<A>(A);

    #[test]
    fn description_of_a_generic_type_returns_the_first_line_of_its_doc_comment() {
        let wrapper = Wrapper(LineCount);

        assert_eq!(wrapper.description().get(), "Runs the action that it wraps");
    }

    #[test]
    fn description_returns_the_first_line_of_the_doc_comment() {
        let action = LineCount;

        assert_eq!(
            action.description().get(),
            "Counts the lines of the project"
        );
    }
}
