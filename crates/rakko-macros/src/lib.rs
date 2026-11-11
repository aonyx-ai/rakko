//! The derive macros of the Rakko contract
//!
//! The [`rakko`][rakko] crate exports every macro of this crate again, and
//! the code that a macro writes refers to items of that crate. So an action,
//! a command, or a group depends on `rakko` and never on this crate.
//!
//! [rakko]: https://docs.rs/rakko

mod documented;

use proc_macro::TokenStream;

/// Derives the description of a type from its doc comment
///
/// The derive implements the `Documented` trait of the `rakko` crate. The
/// description that the trait returns is the first paragraph of the doc
/// comment of the type. The first paragraph ends at the first blank line of
/// the doc comment. The derive puts the lines of that paragraph on one line,
/// with one space between them. So the sentence that describes an
/// action, a command, or a group exists once. The help of a command line and
/// the documentation of the crate cannot differ.
///
/// The derive works on a struct, an enum, and a union, with or without
/// generic parameters.
///
/// A type without a doc comment fails the build, because it has no paragraph
/// to take. So does a type whose doc comment holds only blank lines. The derive
/// reads only the doc comments that the type states as text. It does not read
/// a doc comment that another macro supplies, such as
/// `#[doc = include_str!("README.md")]`.
#[proc_macro_derive(Documented)]
pub fn derive_documented(input: TokenStream) -> TokenStream {
    self::documented::expand(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
