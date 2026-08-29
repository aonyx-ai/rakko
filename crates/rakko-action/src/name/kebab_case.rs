use super::ParseNameError;

/// Defines a name that holds only what kebab case holds
///
/// The macro writes a newtype over a string, the constructors that validate
/// their input, and the traits that let a caller read the name back. Every
/// name of this crate has the same shape, and one copy of that shape keeps
/// two of them from drifting apart.
///
/// The caller writes the documentation of the type, because a reader of the
/// type needs to know what it identifies, and only the caller knows that.
macro_rules! kebab_case_name {
    ($(#[$attribute:meta])* $name:ident) => {
        $(#[$attribute])*
        #[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        pub struct $name(::std::borrow::Cow<'static, str>);

        impl $name {
            /// Constructs a name from a static string
            ///
            /// The function is `const`, so a constant context evaluates it
            /// during the build. In that context, an input that does not
            /// satisfy the rules for a name fails the build.
            ///
            /// # Panics
            ///
            /// Panics when `input` does not satisfy the rules for a name. In
            /// a constant context, the panic becomes a compile error, so call
            /// this function in a constant context.
            pub const fn from_static(input: &'static str) -> Self {
                $crate::name::kebab_case::assert_valid(input);

                Self(::std::borrow::Cow::Borrowed(input))
            }

            /// Returns the text of the name
            pub fn get(&self) -> &str {
                &self.0
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::name::ParseNameError;

            fn from_str(input: &str) -> ::std::result::Result<Self, Self::Err> {
                $crate::name::kebab_case::validate(input)?;

                Ok(Self(::std::borrow::Cow::Owned(input.to_owned())))
            }
        }

        impl ::std::convert::TryFrom<&str> for $name {
            type Error = $crate::name::ParseNameError;

            fn try_from(input: &str) -> ::std::result::Result<Self, Self::Error> {
                input.parse()
            }
        }

        impl ::std::convert::TryFrom<String> for $name {
            type Error = $crate::name::ParseNameError;

            fn try_from(input: String) -> ::std::result::Result<Self, Self::Error> {
                $crate::name::kebab_case::validate(&input)?;

                Ok(Self(::std::borrow::Cow::Owned(input)))
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

pub(crate) use kebab_case_name;

/// Stops the build when `input` does not satisfy the rules for a name
///
/// The function is `const`, so a constant context evaluates it during the
/// build. A name that a literal string states is part of the source of an
/// action, so a literal that breaks a rule is a defect that the build reports
/// and never a run.
///
/// # Panics
///
/// Panics when `input` does not satisfy the rules for a name. In a constant
/// context, the panic becomes a compile error.
// action[impl args.literal]
// action[impl name.literal]
pub(crate) const fn assert_valid(input: &str) {
    let bytes = input.as_bytes();

    assert!(!bytes.is_empty(), "name is empty");
    assert!(
        bytes[0].is_ascii_lowercase(),
        "name starts with an invalid character"
    );

    let mut position = 0;
    let mut previous_was_hyphen = false;
    while position < bytes.len() {
        let byte = bytes[position];
        assert!(
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-',
            "name contains an invalid character"
        );
        assert!(
            byte != b'-' || !previous_was_hyphen,
            "name contains consecutive hyphens"
        );
        previous_was_hyphen = byte == b'-';
        position += 1;
    }

    assert!(bytes[bytes.len() - 1] != b'-', "name ends with a hyphen");
}

/// Validates that `input` satisfies the rules for a name
///
/// # Errors
///
/// Returns a [`ParseNameError`] when `input` does not satisfy the rules for a
/// name.
// action[impl args.name]
pub(crate) fn validate(input: &str) -> Result<(), ParseNameError> {
    // action[impl name.empty]
    let Some(first) = input.chars().next() else {
        return Err(ParseNameError::Empty);
    };

    // action[impl name.start]
    if !first.is_ascii_lowercase() {
        return Err(ParseNameError::InvalidStart { character: first });
    }

    let mut prev_was_hyphen = false;
    for (pos, c) in input.char_indices() {
        // action[impl name.character]
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            return Err(ParseNameError::InvalidCharacter {
                character: c,
                position: pos,
            });
        }
        // action[impl name.hyphens]
        if c == '-' && prev_was_hyphen {
            return Err(ParseNameError::ConsecutiveHyphens { position: pos });
        }
        prev_was_hyphen = c == '-';
    }

    // action[impl name.end]
    if input.ends_with('-') {
        return Err(ParseNameError::TrailingHyphen);
    }

    Ok(())
}
