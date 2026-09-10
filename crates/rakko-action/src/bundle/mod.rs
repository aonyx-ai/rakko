use std::fmt;

use getset::Getters;

use crate::erased_action::ErasedAction;

/// The list of actions that a bundle exports
///
/// A bundle is a crate that exports a set of actions, so that a project adopts
/// the set with one dependency and one line in its harness. This type is what
/// a bundle exports: the same list of erased actions that a harness mounts. A
/// harness passes the list to its mount as it is, so a bundle needs no type of
/// its own and no code that registers it.
///
/// The list is an ordinary value. A bundle contains another bundle when it
/// extends its own list with the list of that bundle. A harness joins two
/// lists the same way, and it leaves an action out when it filters the list
/// and collects the rest into a new one. Nothing in this crate reads the list,
/// so the harness alone names what runs.
///
/// A bundle exports the list through a function, because the mount takes
/// ownership of every action in the list. Each call builds a fresh list. The
/// function is named `bundle`, so that a harness mounts a bundle by the name
/// of its crate.
///
/// # Examples
///
/// A bundle exports its actions as one list:
///
/// ```
/// use rakko_action::Bundle;
/// # use rakko_action::{Action, Context, Name, Outcome, action_name};
/// # struct FormatJson;
/// # impl Action for FormatJson {
/// #     type Args = ();
/// #     fn name(&self) -> Name { action_name!("format-json") }
/// #     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
/// #         Outcome::Passed { summary: None }
/// #     }
/// # }
/// # struct LintYaml;
/// # impl Action for LintYaml {
/// #     type Args = ();
/// #     fn name(&self) -> Name { action_name!("lint-yaml") }
/// #     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
/// #         Outcome::Passed { summary: None }
/// #     }
/// # }
///
/// /// Returns the actions that any project runs
/// pub fn bundle() -> Bundle {
///     Bundle::new(vec![Box::new(FormatJson), Box::new(LintYaml)])
/// }
/// #
/// # assert_eq!(bundle().actions().len(), 2);
/// ```
///
/// A bundle contains another bundle when it includes the list of that bundle
/// in its own. The crate `rakko_rust_library` depends on the crate
/// `rakko_rust` and exports every action of it:
///
/// ```
/// # use rakko_action::Bundle;
/// mod rakko_rust {
///     use rakko_action::Bundle;
/// #   use rakko_action::{Action, Context, Name, Outcome, action_name};
/// #   struct LintRust;
/// #   impl Action for LintRust {
/// #       type Args = ();
/// #       fn name(&self) -> Name { action_name!("lint-rust") }
/// #       async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
/// #           Outcome::Passed { summary: None }
/// #       }
/// #   }
///
///     /// Returns the actions that any Rust project runs
///     pub fn bundle() -> Bundle {
///         Bundle::new(vec![Box::new(LintRust)])
///     }
/// }
///
/// mod rakko_rust_library {
///     use rakko_action::Bundle;
/// #   use rakko_action::{Action, Context, Name, Outcome, action_name};
/// #   use super::rakko_rust;
/// #   struct CheckMsrv;
/// #   impl Action for CheckMsrv {
/// #       type Args = ();
/// #       fn name(&self) -> Name { action_name!("check-msrv") }
/// #       async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
/// #           Outcome::Passed { summary: None }
/// #       }
/// #   }
///
///     /// Returns the actions that a Rust library runs
///     pub fn bundle() -> Bundle {
///         let mut actions = rakko_rust::bundle();
///         actions.push(Box::new(CheckMsrv));
///         actions
///     }
/// }
/// #
/// # fn main() {
/// #     assert_eq!(rakko_rust_library::bundle().actions().len(), 2);
/// # }
/// ```
// action[impl bundle.list]
#[derive(Getters)]
pub struct Bundle {
    /// The actions, in the order in which the bundle lists them
    #[getset(get = "pub")]
    actions: Vec<Box<dyn ErasedAction>>,
}

impl Bundle {
    /// Creates a bundle from the given actions
    ///
    /// The bundle keeps the order of the actions, and a mount receives them
    /// in that order.
    // action[impl bundle.list]
    pub fn new(actions: Vec<Box<dyn ErasedAction>>) -> Self {
        Self { actions }
    }

    /// Adds an action after the actions that the bundle holds
    // action[impl bundle.list]
    pub fn push(&mut self, action: Box<dyn ErasedAction>) {
        self.actions.push(action);
    }
}

/// Shows the names of the actions, because an erased action shows nothing
impl fmt::Debug for Bundle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<_> = self.actions.iter().map(|action| action.name()).collect();

        formatter
            .debug_struct("Bundle")
            .field("actions", &names)
            .finish()
    }
}

/// Adds actions after the actions that the bundle holds
///
/// A bundle is an iterator of its actions, so a bundle includes another
/// bundle by extending itself with that bundle.
// action[impl bundle.contains]
impl Extend<Box<dyn ErasedAction>> for Bundle {
    fn extend<I: IntoIterator<Item = Box<dyn ErasedAction>>>(&mut self, actions: I) {
        self.actions.extend(actions);
    }
}

/// Collects actions into a bundle, in the order of the iterator
///
/// A harness that filters the list of a bundle collects what remains into a
/// new bundle.
// action[impl bundle.list]
impl FromIterator<Box<dyn ErasedAction>> for Bundle {
    fn from_iter<I: IntoIterator<Item = Box<dyn ErasedAction>>>(actions: I) -> Self {
        Self::new(actions.into_iter().collect())
    }
}

/// Yields the actions of the bundle, in order
///
/// A mount takes an iterator of erased actions, so a harness passes a bundle
/// to it as it is.
// action[impl bundle.mount]
impl IntoIterator for Bundle {
    type Item = Box<dyn ErasedAction>;
    type IntoIter = std::vec::IntoIter<Box<dyn ErasedAction>>;

    fn into_iter(self) -> Self::IntoIter {
        self.actions.into_iter()
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;
    use crate::action::Action;
    use crate::action_name;
    use crate::context::Context;
    use crate::name::Name;
    use crate::outcome::Outcome;

    /// An action that stands in for a formatter of a bundle
    struct FormatJson;

    impl Action for FormatJson {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("format-json")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// An action that stands in for a linter of a bundle
    struct LintYaml;

    impl Action for LintYaml {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("lint-yaml")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// Returns the list of a bundle that another bundle contains
    fn contained() -> Bundle {
        Bundle::new(vec![Box::new(FormatJson)])
    }

    /// Takes what the mount of a projection takes, and returns the names in
    /// the order in which they arrived
    fn mount(actions: impl IntoIterator<Item = Box<dyn ErasedAction>>) -> Vec<String> {
        actions
            .into_iter()
            .map(|action| action.name().get().to_owned())
            .collect()
    }

    /// Returns the names of the actions of a bundle, in the order of the list
    fn names(bundle: &Bundle) -> Vec<String> {
        bundle
            .actions()
            .iter()
            .map(|action| action.name().get().to_owned())
            .collect()
    }

    // action[verify bundle.list]
    #[test]
    fn bundle_collects_the_actions_that_remain_after_a_filter() {
        let bundle = Bundle::new(vec![Box::new(FormatJson), Box::new(LintYaml)]);

        let filtered: Bundle = bundle
            .into_iter()
            .filter(|action| action.name().get() != "format-json")
            .collect();

        assert_eq!(names(&filtered), ["lint-yaml"]);
    }

    // action[verify bundle.contains]
    #[test]
    fn bundle_holds_the_list_of_another_bundle() {
        let mut bundle = Bundle::new(vec![Box::new(LintYaml)]);

        bundle.extend(contained());

        assert_eq!(names(&bundle), ["lint-yaml", "format-json"]);
    }

    // action[verify bundle.list]
    #[test]
    fn bundle_holds_the_erased_actions_that_it_exports() {
        let bundle = Bundle::new(vec![Box::new(FormatJson), Box::new(LintYaml)]);

        let names = names(&bundle);

        assert_eq!(names, ["format-json", "lint-yaml"]);
    }

    // action[verify bundle.mount]
    #[test]
    fn bundle_passes_to_a_mount_as_it_is() {
        let bundle = Bundle::new(vec![Box::new(FormatJson), Box::new(LintYaml)]);

        let mounted = mount(bundle);

        assert_eq!(mounted, ["format-json", "lint-yaml"]);
    }

    // action[verify bundle.list]
    #[test]
    fn push_adds_the_action_after_the_others() {
        let mut bundle = Bundle::new(vec![Box::new(FormatJson)]);

        bundle.push(Box::new(LintYaml));

        assert_eq!(names(&bundle), ["format-json", "lint-yaml"]);
    }
}
