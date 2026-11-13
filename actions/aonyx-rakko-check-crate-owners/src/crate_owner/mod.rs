//! An owner that every crate of a project must have

mod kind;

pub(crate) use self::kind::OwnerKind;

/// The prefix that crates.io gives to the login of a team
const TEAM_PREFIX: &str = "github:";

typed_fields::name! {
    /// An owner that every crate of a project must have on crates.io
    ///
    /// The value is the login that crates.io shows for the owner. For a user,
    /// this is the login of the user on GitHub, such as `jdno`. For a team, it
    /// is `github:` with the organization and the team, such as
    /// `github:aonyx-ai:crates-io`. The value is also the argument that
    /// `cargo owner --add` takes for the owner.
    ///
    /// GitHub ignores the case of a login. So the action ignores the case of
    /// ASCII letters when it looks for an owner among the owners that
    /// crates.io lists. The `==` operator of this type does not ignore the case.
    ///
    /// # Examples
    ///
    /// ```
    /// use aonyx_rakko_check_crate_owners::CrateOwner;
    ///
    /// let owner = CrateOwner::new("github:aonyx-ai:crates-io");
    ///
    /// assert_eq!(owner.get(), "github:aonyx-ai:crates-io");
    /// ```
    CrateOwner
}

impl CrateOwner {
    /// Returns whether crates.io adds the owner at once or invites it
    ///
    /// crates.io adds a team at once. It sends a user an invitation, and the
    /// user becomes an owner only after they accept it.
    pub(crate) fn kind(&self) -> OwnerKind {
        if self.get().starts_with(TEAM_PREFIX) {
            OwnerKind::Team
        } else {
            OwnerKind::User
        }
    }

    /// Returns whether two logins refer to the same owner
    ///
    /// GitHub ignores the case of a login, and crates.io shows the login in
    /// the case that GitHub gives it. So the comparison ignores the case of
    /// ASCII letters.
    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.get().eq_ignore_ascii_case(other.get())
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
    fn crate_owner_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CrateOwner>();
    }

    #[test]
    fn kind_of_a_team_is_team() {
        let owner = CrateOwner::new("github:aonyx-ai:crates-io");

        assert_eq!(owner.kind(), OwnerKind::Team);
    }

    #[test]
    fn kind_of_a_user_is_user() {
        let owner = CrateOwner::new("jdno");

        assert_eq!(owner.kind(), OwnerKind::User);
    }

    #[test]
    fn matches_a_login_in_another_case() {
        let owner = CrateOwner::new("jdno");

        assert!(owner.matches(&CrateOwner::new("JDNO")));
    }

    #[test]
    fn matches_no_other_login() {
        let owner = CrateOwner::new("jdno");

        assert!(!owner.matches(&CrateOwner::new("martijnarts")));
    }
}
