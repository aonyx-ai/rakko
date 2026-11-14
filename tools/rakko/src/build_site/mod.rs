//! The action that builds the website of the repository

mod args;
mod error;

use std::path::Path;

use rakko::{
    Access, Action, Context, Documented, Finding, Location, Name, Outcome, RequiredVersion,
    Summary, ToolOutput, ToolRequirement, VersionRange, action_name,
};
use rakko_tool::{Execution, Launch, ShownStreams, Tool};

pub(crate) use self::args::{BaseUrl, BuildSiteArgs};
pub(crate) use self::error::BuildSiteError;

/// The tool of mise that builds the website, and its program
const ZOLA: &str = "zola";

/// The versions of Zola that the templates and the pages of the website build
/// with
///
/// A release of Zola can change the syntax of the templates, as 0.22 and 0.23
/// did. So the range stops before the next minor release.
const ZOLA_RANGE: &str = ">=0.23.6, <0.24";

/// The directory of the website, relative to the root of the project
const SITE: &str = "site";

/// What the finding says when Zola ends without success and writes nothing
const NO_DIAGNOSIS: &str = "zola did not build the website and reported nothing";

/// Builds the website of the repository with Zola
///
/// The website lives in `site/`, and the build writes it into `site/public`.
/// The build fails on a broken `@/` link or anchor between two pages. It also
/// fails on an example that a page shows from a file or a region that does
/// not exist. So the build is the check of the website as well, and CI runs it
/// on every pull request.
///
/// Zola compiles the templates and the pages of the project, and they are
/// written for the 0.23 releases of Zola. So Zola is a toolchain of the
/// project: the root `mise.toml` pins it, and the action declares the range of
/// versions that it accepts.
///
/// Zola writes every link as an absolute URL below the address in
/// `site/zola.toml`. A build for another host, such as the preview of a pull
/// request, gives that host with `--base-url`.
///
/// A build that fails gives one finding, with what Zola reported. A run stops
/// with an error when the project pins no Zola in the range, or when Zola
/// does not start. A run declares a write, because Zola replaces
/// `site/public`.
#[derive(Copy, Clone, Debug, Default, Documented)]
pub(crate) struct BuildSite;

impl Action for BuildSite {
    type Args = BuildSiteArgs;

    fn name(&self) -> Name {
        action_name!("build-site")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Write
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![requirement()]
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        let zola = match Tool::resolve_requirement()
            .requirement(&requirement())
            .context(context)
            .call()
            .await
        {
            Ok(zola) => zola,
            Err(source) => {
                return Outcome::Errored {
                    source: Box::new(BuildSiteError::UnresolvedZola { source }),
                };
            }
        };

        match build(
            &zola,
            context.root().get(),
            args.base_url(),
            context.tool_output(),
        )
        .await
        {
            Ok(execution) => outcome(&execution),
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Returns the requirement of the Zola that builds the website
///
/// Zola is a toolchain, so the requirement holds the range of versions that
/// the action accepts, and the project pins the version.
fn requirement() -> ToolRequirement {
    ToolRequirement::builder()
        .tool(ZOLA)
        .executable(ZOLA)
        .version(RequiredVersion::Range(VersionRange::new(ZOLA_RANGE)))
        .build()
}

/// Runs Zola on the website below `root`, and returns how it ended
///
/// A `base_url` replaces the address in `site/zola.toml` for this build. The
/// build sends what Zola writes to the tool output.
///
/// # Errors
///
/// Returns [`BuildSiteError::ZolaUnavailable`] when Zola does not start.
async fn build(
    zola: &Tool,
    root: &Path,
    base_url: Option<&BaseUrl>,
    output: &ToolOutput,
) -> Result<Execution, BuildSiteError> {
    let mut args = vec!["--root", SITE, "build"];
    if let Some(base_url) = base_url {
        args.extend(["--base-url", base_url.get()]);
    }

    zola.invocation()
        .args(args)
        .in_directory(root)
        .launch_with(output, ShownStreams::Both)
        .await
        .map_err(|source| BuildSiteError::ZolaUnavailable { source })
}

/// Returns the outcome of a build that Zola ended
fn outcome(execution: &Execution) -> Outcome {
    if execution.status().success() {
        return Outcome::Passed {
            summary: Some(Summary::new("built the website into site/public")),
        };
    }

    Outcome::Failed {
        findings: vec![
            Finding::builder()
                .message(diagnosis(execution))
                .location(Location::Project)
                .build(),
        ],
        repairs: Vec::new(),
    }
}

/// Returns what Zola reported about a build that ended without success
///
/// Zola writes its progress and its errors to the standard error.
fn diagnosis(execution: &Execution) -> String {
    let diagnosis = execution.stderr().to_string_lossy();
    let text = diagnosis.trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::sync::{Arc, Mutex};

    use rakko::ToolEvent;

    use super::*;

    /// Returns the context of this repository, which pins Zola
    fn repository() -> Context {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("the harness lives two directories below the root of the repository");

        Context::builder().root(root).build()
    }

    /// Returns the Zola that this repository pins
    async fn zola() -> Tool {
        Tool::resolve_requirement()
            .requirement(&requirement())
            .context(&repository())
            .call()
            .await
            .expect("the test resolves the Zola that the repository pins")
    }

    #[test]
    fn access_is_write() {
        let access = BuildSite.access(&repository(), &BuildSiteArgs::default());

        assert_eq!(access, Access::Write);
    }

    #[tokio::test]
    async fn build_of_a_site_that_builds_passes() {
        let project = tempfile::tempdir().expect("the test creates a temporary directory");
        let site = project.path().join(SITE);
        std::fs::create_dir(&site).expect("the test creates the site");
        std::fs::create_dir(site.join("templates")).expect("the test creates the templates");
        std::fs::write(
            site.join("zola.toml"),
            "base_url = \"https://example.com\"\n",
        )
        .expect("the test writes the configuration");

        let execution = build(&zola().await, project.path(), None, &ToolOutput::discard())
            .await
            .expect("the test starts zola");

        assert!(matches!(outcome(&execution), Outcome::Passed { .. }));
    }

    #[tokio::test]
    async fn build_of_a_site_with_a_base_url_links_below_that_address() {
        let project = tempfile::tempdir().expect("the test creates a temporary directory");
        let site = project.path().join(SITE);
        std::fs::create_dir(&site).expect("the test creates the site");
        std::fs::create_dir(site.join("templates")).expect("the test creates the templates");
        std::fs::write(
            site.join("templates").join("index.html"),
            "{{ get_url(path='style.css') }}",
        )
        .expect("the test writes the template");
        std::fs::write(
            site.join("zola.toml"),
            "base_url = \"https://example.com\"\n",
        )
        .expect("the test writes the configuration");
        let base_url = BaseUrl::parse("https://preview.example.dev").expect("the address parses");

        build(
            &zola().await,
            project.path(),
            Some(&base_url),
            &ToolOutput::discard(),
        )
        .await
        .expect("the test starts zola");

        let page = std::fs::read_to_string(site.join("public").join("index.html"))
            .expect("the build writes the landing page");
        assert_eq!(page, "https://preview.example.dev/style.css");
    }

    #[tokio::test]
    async fn build_of_a_site_without_a_configuration_fails_with_what_zola_reported() {
        let project = tempfile::tempdir().expect("the test creates a temporary directory");
        std::fs::create_dir(project.path().join(SITE)).expect("the test creates the site");

        let execution = build(&zola().await, project.path(), None, &ToolOutput::discard())
            .await
            .expect("the test starts zola");

        assert!(matches!(
            outcome(&execution),
            Outcome::Failed { findings, .. }
                if findings.iter().any(|finding| finding.message().get().contains("zola.toml"))
        ));
    }

    #[tokio::test]
    async fn build_sends_the_start_of_zola_to_the_tool_output() {
        let project = tempfile::tempdir().expect("the test creates a temporary directory");
        let events = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&events);
        let output = ToolOutput::new(move |event| {
            received
                .lock()
                .expect("no test panics while it holds the lock")
                .push(event);
        });

        let _execution = build(&zola().await, project.path(), None, &output).await;

        assert!(
            events
                .lock()
                .expect("no test panics while it holds the lock")
                .iter()
                .any(|event| matches!(event, ToolEvent::Started(_)))
        );
    }

    #[test]
    fn build_site_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<BuildSite>();
    }

    #[test]
    fn name_is_build_site() {
        let name = BuildSite.name();

        assert_eq!(name.get(), "build-site");
    }

    #[tokio::test]
    async fn requirement_accepts_the_zola_that_the_repository_pins() {
        let resolution = Tool::resolve_requirement()
            .requirement(&requirement())
            .context(&repository())
            .call()
            .await;

        assert!(resolution.is_ok());
    }

    #[tokio::test]
    async fn run_in_a_project_that_pins_no_zola_is_an_error() {
        let project = tempfile::tempdir().expect("the test creates a temporary directory");
        let context = Context::builder().root(project.path()).build();

        let outcome = BuildSite.run(&context, &BuildSiteArgs::default()).await;

        assert!(matches!(
            outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref::<BuildSiteError>(),
                    Some(BuildSiteError::UnresolvedZola { .. })
                )
        ));
    }

    #[test]
    fn tools_declare_zola() {
        let tools = BuildSite.tools();

        assert_eq!(tools, vec![requirement()]);
    }
}
