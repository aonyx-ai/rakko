/// The directory that an action writes disposable data to
mod cache_directory;
/// The directory that holds the configuration of the tools of a project
mod config_directory;

pub use cache_directory::CacheDirectory;
pub use config_directory::ConfigDirectory;

use super::project_root::ProjectRoot;
use bon::bon;
use getset::Getters;

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
    /// The cache directory of the project
    #[getset(get = "pub")]
    cache: CacheDirectory,
}

#[bon]
impl Layout {
    /// Creates a layout from a project root
    ///
    /// When `config` is absent the layout uses the default path: `.config`
    /// inside the project root. When `cache` is absent the layout uses the
    /// default path: `target/rakko` inside the project root.
    // action[impl layout.config]
    // action[impl layout.cache]
    // action[impl layout.override]
    #[builder]
    pub fn new(
        #[builder(into)] root: ProjectRoot,
        #[builder(into)] config: Option<ConfigDirectory>,
        #[builder(into)] cache: Option<CacheDirectory>,
    ) -> Self {
        let config = config.unwrap_or_else(|| ConfigDirectory::new(root.get().join(".config")));
        let cache =
            cache.unwrap_or_else(|| CacheDirectory::new(root.get().join("target").join("rakko")));

        Self { config, cache }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;

    // action[verify layout.cache]
    #[test]
    fn cache_default_is_target_rakko_in_root() {
        let layout = Layout::builder().root("/tmp/my-project").build();

        assert_eq!(
            layout.cache().get(),
            &PathBuf::from("/tmp/my-project/target/rakko"),
        );
    }

    // action[verify layout.config]
    #[test]
    fn config_default_is_dot_config_in_root() {
        let layout = Layout::builder().root("/tmp/my-project").build();

        assert_eq!(
            layout.config().get(),
            &PathBuf::from("/tmp/my-project/.config"),
        );
    }

    #[test]
    fn override_accepts_custom_cache_directory() {
        let layout = Layout::builder()
            .root("/tmp/my-project")
            .cache("/tmp/other/cache")
            .build();

        assert_eq!(layout.cache().get(), &PathBuf::from("/tmp/other/cache"));
    }

    // action[verify layout.override]
    #[test]
    fn override_accepts_custom_config_directory() {
        let layout = Layout::builder()
            .root("/tmp/my-project")
            .config("/tmp/other/.config")
            .build();

        assert_eq!(layout.config().get(), &PathBuf::from("/tmp/other/.config"));
    }
}
