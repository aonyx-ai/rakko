//! The branch of a pull request

/// The start of the branch of a release pull request of Otto
const RELEASE: &str = "release/";

typed_fields::name! {
    /// The name of the head branch of a pull request, such as `fix-parser`
    ///
    /// Otto opens the pull request of a release from the branch
    /// `release/<unit>`, so the branch tells a release pull request from any
    /// other.
    Branch
}

impl Branch {
    /// Returns the unit that a release branch of Otto names, or `None` when
    /// the branch is no release branch
    ///
    /// The branch `release/<unit>` names the unit `<unit>`. The caller still
    /// checks that the unit exists, because any author can name a branch so.
    pub(crate) fn released_unit(&self) -> Option<&str> {
        self.get()
            .strip_prefix(RELEASE)
            .filter(|unit| !unit.is_empty())
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn released_unit_of_a_branch_of_another_name_is_none() {
        let branch = Branch::new("fix-release-notes");

        let unit = branch.released_unit();

        assert_eq!(unit, None);
    }

    #[test]
    fn released_unit_of_a_release_branch_is_the_unit() {
        let branch = Branch::new("release/rakko-cli");

        let unit = branch.released_unit();

        assert_eq!(unit, Some("rakko-cli"));
    }

    #[test]
    fn released_unit_of_a_release_branch_without_a_unit_is_none() {
        let branch = Branch::new("release/");

        let unit = branch.released_unit();

        assert_eq!(unit, None);
    }
}
