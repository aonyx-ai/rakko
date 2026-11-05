/// The directory that holds the configuration of the tools of a project
mod config_directory;

use bon::bon;
use getset::Getters;

pub use self::config_directory::ConfigDirectory;
use super::project_root::ProjectRoot;

/// The directory layout of a project
///
/// A layout tells an action where the directories of a project are. Each
/// directory has a default that comes from the project root, and a caller
/// can supply a path in place of any default.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Layout {
    /// The configuration directory of the project
    #[getset(get = "pub")]
    config: ConfigDirectory,
}

#[bon]
impl Layout {
    /// Creates a layout from a project root
    ///
    /// When `config` is absent the layout uses the default path: `.config`
    /// inside the project root.
    #[builder]
    pub fn new(
        #[builder(into)] root: ProjectRoot,
        #[builder(into)] config: Option<ConfigDirectory>,
    ) -> Self {
        let config = config.unwrap_or_else(|| ConfigDirectory::new(root.get().join(".config")));

        Self { config }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;

    #[test]
    fn config_default_is_dot_config_in_root() {
        let layout = Layout::builder().root("/tmp/my-project").build();

        assert_eq!(
            layout.config().get(),
            &PathBuf::from("/tmp/my-project/.config"),
        );
    }

    #[test]
    fn override_accepts_custom_config_directory() {
        let layout = Layout::builder()
            .root("/tmp/my-project")
            .config("/tmp/other/.config")
            .build();

        assert_eq!(layout.config().get(), &PathBuf::from("/tmp/other/.config"));
    }
}
