//! The tools that the mise file of a crate declares

use crate::tool::{ToolPin, ToolRequirement};

/// The tools that the mise file of a crate declares
///
/// A crate that wraps a tool pins it in the file `mise.rakko.toml` at the
/// root of the crate. Every tool that the file pins is a declared tool, with
/// its identifier, its exact version, and its platforms. Only
/// [`bundled_tools!`] creates such a value, from the file of the crate that
/// calls it. So each value holds only pins of a mise file.
///
/// An action returns the tools of its own file from
/// [`Action::bundled_tools`], and the tools of a helper crate from
/// [`Action::additional_tools`]. A helper crate that wraps a tool offers its
/// tools as a constant, so that each action that uses the helper adds them.
///
/// The value holds the pins that the crate embedded when it compiled. So a
/// published crate carries its declarations, and no run reads a file of the
/// crate.
///
/// [`Action::additional_tools`]: crate::Action::additional_tools
/// [`Action::bundled_tools`]: crate::Action::bundled_tools
/// [`bundled_tools!`]: crate::bundled_tools
///
/// # Examples
///
/// An action that runs no tool declares none:
///
/// ```
/// use rakko::Tools;
///
/// assert!(Tools::NONE.requirements().is_empty());
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Tools(&'static [ToolPin]);

impl Tools {
    /// The declaration of no tool
    ///
    /// It is the default of [`Action::bundled_tools`], for an action that has
    /// no mise file of its own.
    ///
    /// [`Action::bundled_tools`]: crate::Action::bundled_tools
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::Tools;
    ///
    /// assert_eq!(Tools::NONE, Tools::default());
    /// ```
    pub const NONE: Self = Self(&[]);

    /// Wraps the pins that the reader of a mise file returned
    pub(crate) const fn from_pins(pins: &'static [ToolPin]) -> Self {
        Self(pins)
    }

    /// Returns the requirement of each declared tool, in the order of the
    /// file
    ///
    /// Each requirement holds the identifier of the tool, its exact version,
    /// and its platforms. It holds no program, because the action gives the
    /// program when it runs the tool.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::Tools;
    ///
    /// let requirements = Tools::NONE.requirements();
    ///
    /// assert_eq!(requirements, Vec::new());
    /// ```
    #[must_use]
    pub fn requirements(&self) -> Vec<ToolRequirement> {
        self.0.iter().map(ToolPin::requirement).collect()
    }
}

/// Returns the tools that the mise file of the calling crate declares
///
/// The macro reads the file `mise.rakko.toml` at the root of the crate that
/// calls it, and it takes no argument. It embeds the file when the crate
/// compiles and reads every pin of the file in a constant. So a published
/// crate carries its declarations, and no run reads the file.
///
/// The `[tools]` table of the file holds the pins. Each line of the table is
/// a pin, a comment, or empty. A pin has one of two forms, on one line:
///
/// - a version in double quotation marks, as in `taplo = "0.10.0"`, and
/// - a table with the version and the platforms where the tool has a build,
///   as in `zizmor = { version = "1.32.0", os = ["linux", "macos/arm64"] }`.
///
/// The key of a pin can be bare, as `taplo`, or in double quotation marks, as
/// `"npm:markdownlint-cli"`. A platform is an operating system with an
/// optional architecture, in the form of the `os` option of mise. The
/// operating system is `linux`, `macos`, or `windows`, and the architecture
/// is `x64` or `arm64`, as in `windows/x64`. Rakko does not read an alias
/// that mise also accepts, such as `aarch64`.
///
/// The version is exact. It has at least three parts that dots separate, and
/// each part starts with a digit, as `0.10.0`. A `v` can come before the
/// first part, and the tag of a release can come before the version with `@`,
/// as `stiles@0.6.0`.
///
/// A file that the macro does not read fails the build of the crate. These
/// files fail:
///
/// - a crate without the file,
/// - a file that pins no tool, or that pins a tool twice,
/// - a line in another form,
/// - a key outside the `[tools]` table, or another table,
/// - a table of a pin that gives a key twice,
/// - a version that is not exact, such as `latest` or `3`, and
/// - a misspelled platform.
///
/// Mise accepts some of these, such as a platform that it does not know, and
/// gives no warning. The macro refuses them, so that no pin reaches a project
/// in a form that Rakko did not read.
///
/// # Examples
///
/// An action that pins its tool in its own mise file returns the tools of
/// that file:
///
/// ```ignore
/// use rakko::{Action, Context, Documented, Name, Outcome, Tools, action_name};
///
/// /// Checks the TOML files of the project
/// #[derive(Documented)]
/// struct CheckToml;
///
/// impl Action for CheckToml {
///     type Args = ();
///
///     fn name(&self) -> Name {
///         action_name!("check-toml")
///     }
///
///     fn bundled_tools(&self) -> Tools {
///         rakko::bundled_tools!()
///     }
///
///     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
///         Outcome::Passed { summary: None }
///     }
/// }
/// ```
///
/// A helper crate offers the tools of its file as a constant:
///
/// ```ignore
/// use rakko::Tools;
///
/// /// The tools that the helper runs
/// pub const TOOLS: Tools = rakko::bundled_tools!();
/// ```
///
/// A crate without the file fails the build:
///
/// ```compile_fail
/// const TOOLS: rakko::Tools = rakko::bundled_tools!();
/// ```
#[macro_export]
macro_rules! bundled_tools {
    () => {
        $crate::__tools_of_mise_file!(::core::include_str!(::core::concat!(
            ::core::env!("CARGO_MANIFEST_DIR"),
            "/mise.rakko.toml"
        )))
    };
}

/// Returns the tools that the text of a mise file declares
///
/// [`bundled_tools!`] gives this macro the text of the file of the crate. The
/// macro is no part of the API of the crate. The items of its expansion have
/// unusual names. An item of the caller with the same name would shadow the
/// text that the macro receives.
///
/// A malformed pin fails the build:
///
/// ```compile_fail,E0080
/// const TOOLS: rakko::Tools = rakko::__tools_of_mise_file!("[tools]\ntaplo = 0.10.0\n");
/// ```
///
/// A misspelled platform fails the build:
///
/// ```compile_fail,E0080
/// const TOOLS: rakko::Tools = rakko::__tools_of_mise_file!(
///     "[tools]\nstiles = { version = \"0.6.0\", os = [\"macos-arm64\"] }\n"
/// );
/// ```
///
/// The same pins in a form that the macro reads compile:
///
/// ```
/// const TOOLS: rakko::Tools = rakko::__tools_of_mise_file!("[tools]\ntaplo = \"0.10.0\"\n");
/// const LIMITED: rakko::Tools = rakko::__tools_of_mise_file!(
///     "[tools]\nstiles = { version = \"0.6.0\", os = [\"macos/arm64\"] }\n"
/// );
///
/// assert_eq!(TOOLS.requirements().len() + LIMITED.requirements().len(), 2);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __tools_of_mise_file {
    ($text:expr) => {{
        const __RAKKO_MISE_FILE: $crate::__private::MiseText<'static> =
            $crate::__private::MiseText::new($text);
        const __RAKKO_PIN_COUNT: usize = __RAKKO_MISE_FILE.pin_count();
        const __RAKKO_PINS: [$crate::__private::ToolPin; __RAKKO_PIN_COUNT] =
            __RAKKO_MISE_FILE.pins::<__RAKKO_PIN_COUNT>();

        $crate::__private::tools(&__RAKKO_PINS)
    }};
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;
    use crate::tool::{RequiredVersion, ToolVersion};

    #[test]
    fn requirements_keep_the_order_of_the_file() {
        let tools: Tools =
            crate::__tools_of_mise_file!("[tools]\nyamllint = \"1.38.0\"\ntaplo = \"0.10.0\"\n");

        let names: Vec<String> = tools
            .requirements()
            .iter()
            .map(|requirement| requirement.tool().get().to_owned())
            .collect();

        assert_eq!(names, ["yamllint", "taplo"]);
    }

    #[test]
    fn requirements_of_a_pin_hold_its_exact_version() {
        let tools: Tools = crate::__tools_of_mise_file!("[tools]\ntaplo = \"0.10.0\"\n");

        assert_eq!(
            tools.requirements()[0].version(),
            &RequiredVersion::Exact(ToolVersion::new("0.10.0"))
        );
    }

    #[test]
    fn requirements_of_none_are_empty() {
        let tools = Tools::NONE;

        assert!(tools.requirements().is_empty());
    }

    #[test]
    fn tools_in_a_constant_are_the_tools_of_the_file() {
        const TOOLS: Tools = crate::__tools_of_mise_file!("[tools]\ntaplo = \"0.10.0\"\n");

        assert_eq!(TOOLS.requirements().len(), 1);
    }

    #[test]
    fn tools_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Tools>();
    }
}
