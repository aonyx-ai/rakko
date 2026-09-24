use std::path::Path;

use toml_edit::{DocumentMut, Item, RawString};

use super::args::{Msrv, Reason};
use super::error::SetMsrvError;

/// The key that declares the minimum supported Rust version
const KEY: &str = "rust-version";

/// The character that starts a comment in TOML
const COMMENT: char = '#';

/// Sets the version of a root manifest and replaces the comment above it
///
/// Returns the version that the manifest declared before the change. A
/// manifest that the run refuses stays as it was.
///
/// # Errors
///
/// Returns [`SetMsrvError::UndeclaredMsrv`] when the manifest declares no
/// version, and [`SetMsrvError::InlineMsrv`] when it declares the version in
/// an inline table. The TOML that Cargo reads allows no comment inside an
/// inline table, so the reason has no place there.
// setmsrv[impl manifest.version]
// setmsrv[impl manifest.comment]
// setmsrv[impl manifest.undeclared]
// setmsrv[impl manifest.inline]
pub(crate) fn set(
    manifest: &mut DocumentMut,
    path: &Path,
    msrv: &Msrv,
    reason: &Reason,
) -> Result<Msrv, SetMsrvError> {
    let undeclared = || SetMsrvError::UndeclaredMsrv {
        path: path.to_path_buf(),
    };

    let table = declaration(manifest).ok_or_else(undeclared)?;
    if table.is_inline_table() {
        return Err(SetMsrvError::InlineMsrv {
            path: path.to_path_buf(),
        });
    }

    let (mut key, value) = table
        .as_table_like_mut()
        .and_then(|table| table.get_key_value_mut(KEY))
        .ok_or_else(undeclared)?;
    let value = value.as_value_mut().ok_or_else(undeclared)?;
    let current = Msrv::new(value.as_str().ok_or_else(undeclared)?);

    super::replace(value, msrv);

    let prefix = key
        .leaf_decor()
        .prefix()
        .and_then(RawString::as_str)
        .unwrap_or_default();
    let prefix = comment(prefix, reason);
    key.leaf_decor_mut().set_prefix(prefix);

    Ok(current)
}

/// Returns the table that declares the version as text
///
/// The members of a workspace inherit the version of `[workspace.package]`,
/// so that table wins over the `[package]` of a root that is a package too.
fn declaration(manifest: &mut DocumentMut) -> Option<&mut Item> {
    let declares = |table: Option<&Item>| {
        table
            .and_then(|table| table.get(KEY))
            .is_some_and(Item::is_str)
    };

    let workspace = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"));

    if declares(workspace) {
        manifest.get_mut("workspace")?.get_mut("package")
    } else if declares(manifest.get("package")) {
        manifest.get_mut("package")
    } else {
        None
    }
}

/// Returns the text in front of the key, with the reason as its comment
///
/// The text holds the lines between the entry before the key and the key
/// itself. The comment lines at its end are the comment of the key, and the
/// reason replaces them with one line. The blank lines and the comments
/// above them stay, and the new line takes the indentation of the key.
fn comment(prefix: &str, reason: &Reason) -> String {
    let (lines, indentation) = match prefix.rfind('\n') {
        Some(end) => prefix.split_at(end + 1),
        None => ("", prefix),
    };

    let mut kept: Vec<&str> = lines.split_inclusive('\n').collect();
    while kept
        .last()
        .is_some_and(|line| line.trim_start().starts_with(COMMENT))
    {
        kept.pop();
    }

    format!(
        "{kept}{indentation}{COMMENT} {reason}\n{indentation}",
        kept = kept.concat()
    )
}
