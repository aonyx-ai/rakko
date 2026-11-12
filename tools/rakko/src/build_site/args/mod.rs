//! The arguments that a run of the build-site action reads

mod base_url;

use rakko::{Args, ArgsSchema, ArgsValues, Argument, ArgumentShape, ReadArgsError, argument_name};

pub(crate) use self::base_url::BaseUrl;

/// The documentation of the base-url argument
const BASE_URL_DOCUMENTATION: &str =
    "Build the links for this address instead of the one in site/zola.toml";

/// The arguments that a run of the build-site action reads
///
/// The action reads one optional argument. Without it, the build gives its
/// links the address that `site/zola.toml` holds, which is the address of the
/// live website. With it, the build gives them the address that the run
/// names, so that a preview on another host links to its own pages.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub(crate) struct BuildSiteArgs {
    /// The address that replaces the one in `site/zola.toml`, if the run gave
    /// one
    base_url: Option<BaseUrl>,
}

impl BuildSiteArgs {
    /// Returns the address that replaces the one in `site/zola.toml`, or
    /// `None` when the build keeps that address
    pub(crate) fn base_url(&self) -> Option<&BaseUrl> {
        self.base_url.as_ref()
    }
}

impl Args for BuildSiteArgs {
    fn schema() -> ArgsSchema {
        ArgsSchema::new([Argument::builder()
            .name(argument_name!("base-url"))
            .shape(ArgumentShape::Text)
            .documentation(BASE_URL_DOCUMENTATION)
            .build()])
    }

    fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
        let name = argument_name!("base-url");
        let Some(value) = values.get(&name) else {
            return Ok(Self::default());
        };

        let Some(base_url) = BaseUrl::parse(value.get()) else {
            return Err(ReadArgsError::UnreadableValue {
                name,
                value: value.clone(),
            });
        };

        Ok(Self {
            base_url: Some(base_url),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::ArgumentValue;

    use super::*;

    /// Returns the values of a run that gives the base-url argument this text
    fn with_base_url(text: &str) -> ArgsValues {
        ArgsValues::new([(argument_name!("base-url"), ArgumentValue::new(text))])
    }

    #[test]
    fn from_values_with_a_base_url_reads_the_address() {
        let args = BuildSiteArgs::from_values(&with_base_url("https://pr-7-rakko.example.dev"))
            .expect("the address is readable");

        assert_eq!(
            args.base_url().map(BaseUrl::get),
            Some("https://pr-7-rakko.example.dev")
        );
    }

    #[test]
    fn from_values_with_a_base_url_without_a_host_is_unreadable() {
        let result = BuildSiteArgs::from_values(&with_base_url("https://"));

        assert!(matches!(result, Err(ReadArgsError::UnreadableValue { .. })));
    }

    #[test]
    fn from_values_with_a_base_url_with_whitespace_is_unreadable() {
        let result = BuildSiteArgs::from_values(&with_base_url("https://rakko.example.dev/a b"));

        assert!(matches!(result, Err(ReadArgsError::UnreadableValue { .. })));
    }

    #[test]
    fn from_values_with_a_base_url_without_a_scheme_is_unreadable() {
        let result = BuildSiteArgs::from_values(&with_base_url("rakko.example.dev"));

        assert!(matches!(result, Err(ReadArgsError::UnreadableValue { .. })));
    }

    #[test]
    fn from_values_with_an_http_base_url_reads_the_address() {
        let args = BuildSiteArgs::from_values(&with_base_url("http://127.0.0.1:1111"))
            .expect("the address is readable");

        assert_eq!(
            args.base_url().map(BaseUrl::get),
            Some("http://127.0.0.1:1111")
        );
    }

    #[test]
    fn from_values_without_a_base_url_keeps_the_configured_address() {
        let args = BuildSiteArgs::from_values(&ArgsValues::new([])).expect("no value is readable");

        assert_eq!(args.base_url(), None);
    }
}
