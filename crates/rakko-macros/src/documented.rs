//! The derive that takes the description of a type from its doc comment

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Error, Expr, ExprLit, Lit, Meta};

/// Writes the implementation of `Documented` for the type of the input
///
/// # Errors
///
/// Returns an error when the input is no type that a derive accepts. Also
/// returns an error when the doc comment of the type gives no line to take.
pub(crate) fn expand(input: TokenStream) -> Result<TokenStream, Error> {
    let input: DeriveInput = syn::parse2(input)?;

    let Some(title) = title(&input.attrs) else {
        return Err(Error::new_spanned(
            &input.ident,
            "`Documented` takes the description from the first paragraph of the doc comment, and \
             this type has none",
        ));
    };

    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::rakko::Documented for #ident #type_generics #where_clause {
            fn description(&self) -> ::rakko::Description {
                ::rakko::Description::new(#title)
            }
        }
    })
}

/// Returns the first paragraph of the doc comment, on one line
///
/// The compiler turns each line of a doc comment into one `doc` attribute,
/// and a block comment into one attribute with many lines. The first
/// paragraph starts at the first line of them all that is not blank, and it
/// ends before the next blank line. The answer joins its lines with one
/// space, without the space around each line. A `doc` attribute of another
/// form, such as `#[doc(hidden)]`, holds no text.
fn title(attributes: &[Attribute]) -> Option<String> {
    let texts: Vec<String> = attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("doc"))
        .filter_map(|attribute| match &attribute.meta {
            Meta::NameValue(pair) => match &pair.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(text),
                    ..
                }) => Some(text.value()),
                _ => None,
            },
            _ => None,
        })
        .collect();

    let paragraph: Vec<&str> = texts
        .iter()
        .flat_map(|text| {
            if text.is_empty() {
                vec![""]
            } else {
                text.lines().collect()
            }
        })
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty())
        .collect();

    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph.join(" "))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Returns what the derive writes for the input, without any space
    fn expanded(input: TokenStream) -> String {
        expand(input).unwrap().to_string().replace(' ', "")
    }

    /// Returns the message of the error that the derive reports for the input
    fn refused(input: TokenStream) -> String {
        expand(input).unwrap_err().to_string()
    }

    #[test]
    fn expand_with_a_blank_first_line_takes_the_first_line_with_text() {
        let input = quote! {
            ///
            /// Formats the TOML files of a project
            struct FormatToml;
        };

        let output = expanded(input);

        assert!(output.contains(r#"Description::new("FormatstheTOMLfilesofaproject")"#));
    }

    #[test]
    fn expand_with_a_block_comment_takes_its_first_line_with_text() {
        let input = quote! {
            #[doc = "\n    Formats the TOML files of a project\n\n    More text.\n"]
            struct FormatToml;
        };

        let output = expanded(input);

        assert!(output.contains(r#"Description::new("FormatstheTOMLfilesofaproject")"#));
    }

    #[test]
    fn expand_with_a_block_comment_whose_paragraph_has_two_lines_joins_the_lines() {
        let input = quote! {
            #[doc = "\n    Formats the TOML files\n    of a project\n\n    More text.\n"]
            struct FormatToml;
        };

        let output = expand(input).unwrap().to_string();

        assert!(output.contains(r#""Formats the TOML files of a project""#));
    }

    #[test]
    fn expand_with_a_doc_comment_implements_the_trait_for_the_type() {
        let input = quote! {
            /// Formats the TOML files of a project
            struct FormatToml;
        };

        let output = expanded(input);

        assert!(output.contains("impl::rakko::DocumentedforFormatToml{"));
    }

    #[test]
    fn expand_with_a_first_paragraph_over_two_lines_joins_the_lines() {
        let input = quote! {
            /// Checks a project against the newest versions
            /// of its dependencies
            ///
            /// Cargo does the work.
            struct CheckLatestDeps;
        };

        let output = expand(input).unwrap().to_string();

        assert!(
            output
                .contains(r#""Checks a project against the newest versions of its dependencies""#)
        );
    }

    #[test]
    fn expand_with_a_hidden_type_reads_past_the_attribute_without_text() {
        let input = quote! {
            #[doc(hidden)]
            /// Formats the TOML files of a project
            struct FormatToml;
        };

        let output = expanded(input);

        assert!(output.contains(r#"Description::new("FormatstheTOMLfilesofaproject")"#));
    }

    #[test]
    fn expand_with_a_long_doc_comment_takes_only_the_first_paragraph() {
        let input = quote! {
            /// Formats the TOML files of a project
            ///
            /// Taplo does the work.
            struct FormatToml;
        };

        let output = expanded(input);

        assert!(!output.contains("Taplo"));
    }

    #[test]
    fn expand_with_another_attribute_that_holds_text_reads_only_the_doc_comment() {
        let input = quote! {
            #[must_use = "Call the action"]
            /// Formats the TOML files of a project
            struct FormatToml;
        };

        let output = expanded(input);

        assert!(output.contains(r#"Description::new("FormatstheTOMLfilesofaproject")"#));
    }

    #[test]
    fn expand_with_generic_parameters_keeps_the_parameters_and_the_bounds() {
        let input = quote! {
            /// Runs the action that it wraps
            struct Wrapper<A: Clone>(A) where A: Send;
        };

        let output = expanded(input);

        assert!(output.contains("impl<A:Clone>::rakko::DocumentedforWrapper<A>whereA:Send{"));
    }

    #[test]
    fn expand_with_no_item_reports_the_error_of_the_parser() {
        let input = quote! { 42 };

        let message = refused(input);

        assert!(message.contains("expected"));
    }

    #[test]
    fn expand_with_only_blank_lines_reports_the_missing_doc_comment() {
        let input = quote! {
            ///
            #[doc = "   "]
            struct FormatToml;
        };

        let message = refused(input);

        assert!(message.contains("this type has none"));
    }

    #[test]
    fn expand_without_a_doc_comment_reports_the_missing_doc_comment() {
        let input = quote! {
            #[derive(Debug)]
            struct FormatToml;
        };

        let message = refused(input);

        assert!(message.contains("this type has none"));
    }
}
