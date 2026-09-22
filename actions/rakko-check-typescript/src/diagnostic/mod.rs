//! One thing that tsc reported about a project
//!
//! A run of tsc reports one diagnostic for each rule of the language that the
//! code breaks, and one for each thing that stopped it from checking the code
//! at all. This module holds one of them: where it points, what kind tsc gave
//! it, which rule of TypeScript it belongs to, and what tsc said about it.

/// The kind that tsc gave a diagnostic
mod category;
/// The place in the project that a diagnostic points at
mod origin;

use bon::Builder;
use getset::Getters;

pub use self::category::Category;
pub use self::origin::Origin;

/// The text that separates the number of a diagnostic from what tsc said
const NUMBER_CLOSE: &str = ": ";

/// The character that separates the kind of a diagnostic from its number
const KIND_CLOSE: char = ' ';

/// One thing that tsc reported about a project
///
/// A diagnostic points at a place of the project, or at no place at all. Tsc
/// writes a place for everything that it found in a file, and it writes none
/// for what it found about the run itself, such as a configuration file that
/// it could not find.
///
/// The number is the identifier that TypeScript gives the rule, as tsc writes
/// it, so `TS2322` and not `2322`. It is the one part of a diagnostic that
/// survives a rewording of the message, which is why a caller that recognizes
/// a diagnostic recognizes it by the number.
///
/// The text is what tsc said, with everything that it added below the first
/// line joined into it. Tsc explains a diagnostic in indented lines underneath
/// it where one sentence does not carry the answer, and those lines name which
/// type disagreed with which, so a message without them tells a reader that
/// two types do not fit and never says why.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, Getters)]
pub struct Diagnostic {
    /// The place that the diagnostic points at, or `None` for no place
    #[getset(get = "pub")]
    origin: Option<Origin>,

    /// The kind that tsc gave the diagnostic
    #[getset(get = "pub")]
    category: Category,

    /// The rule of TypeScript, as tsc numbers it
    #[builder(into)]
    #[getset(get = "pub")]
    number: String,

    /// What tsc said about the diagnostic
    #[builder(into)]
    #[getset(get = "pub")]
    text: String,
}

impl Diagnostic {
    /// Returns the sentence that tsc wrote about the diagnostic
    ///
    /// The sentence holds the kind, the number, and the text, in the order
    /// that tsc writes them, so that a finding reads like the line that a
    /// contributor sees when they run tsc themselves.
    pub fn message(&self) -> String {
        format!(
            "{}{KIND_CLOSE}{}{NUMBER_CLOSE}{}",
            self.category, self.number, self.text
        )
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn message_reads_like_the_line_that_tsc_writes() {
        let diagnostic = Diagnostic::builder()
            .category(Category::Error)
            .number("TS2322")
            .text("Type 'string' is not assignable to type 'number'.")
            .build();

        assert_eq!(
            diagnostic.message(),
            "error TS2322: Type 'string' is not assignable to type 'number'."
        );
    }
}
