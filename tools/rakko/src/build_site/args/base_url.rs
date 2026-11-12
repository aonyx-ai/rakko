//! The address that a build of the website gives its links

/// The address that a build of the website gives its links, such as
/// `https://rakko.aonyx.ai`
///
/// Zola writes every link and every asset of a page as an absolute URL below
/// this address. A build for another host, such as the preview of a pull
/// request, therefore needs the address of that host, or its pages load from
/// the live website.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct BaseUrl(String);

impl BaseUrl {
    /// Returns the address of a URL that starts with `http://` or `https://`
    /// and holds no whitespace, or `None` for any other text
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let rest = text
            .strip_prefix("https://")
            .or_else(|| text.strip_prefix("http://"))?;

        if rest.is_empty() || text.chars().any(char::is_whitespace) {
            return None;
        }

        Some(Self(text.to_owned()))
    }

    /// Returns the address as it was written
    pub(crate) fn get(&self) -> &str {
        &self.0
    }
}
