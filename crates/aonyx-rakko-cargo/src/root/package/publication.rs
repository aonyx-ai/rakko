//! Where the manifest of a package lets cargo publish it

/// Where the manifest of a package lets cargo publish it
///
/// A manifest without the `publish` key allows every registry, crates.io
/// included. A manifest that lists registries allows only those registries.
/// It allows crates.io when the list holds `crates-io`, the name that cargo
/// gives to crates.io. A manifest that sets `publish = false`, or
/// `publish = []`, forbids every registry. A caller that asks whether a
/// package can reach crates.io reads this value. It does not read the
/// manifest again.
///
/// A package that a caller builds without this value is
/// [`CratesIo`][Publication::CratesIo], because cargo reads a manifest
/// without the key in the same way.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_cargo::{CargoPackage, DocumentedNames, PackageName, Publication};
///
/// let package = CargoPackage::builder()
///     .name(PackageName::new("harness"))
///     .documented_names(DocumentedNames::Unique)
///     .publication(Publication::Forbidden)
///     .build();
///
/// assert_eq!(package.publication(), Publication::Forbidden);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Publication {
    /// The manifest lets cargo publish the package to crates.io
    #[default]
    CratesIo,

    /// The manifest lets cargo publish the package only to registries other
    /// than crates.io
    OtherRegistries,

    /// The manifest forbids every registry
    Forbidden,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Publication>();
    }
}
