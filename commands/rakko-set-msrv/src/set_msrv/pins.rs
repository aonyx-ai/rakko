use toml_edit::{DocumentMut, Item, Value};

use super::args::Msrv;

/// Sets the Rust pin of mise at the current version to the new version
///
/// A pin is a version as text, or a table whose `version` holds it, and
/// `rust` holds one pin or an array of them. The pin keeps its position in
/// the array and its other keys.
///
/// The first pin of the array is the default toolchain of the project. A
/// project that pins its minimum supported version beside the default pins
/// the same version twice when the two agree, so the run moves the last pin
/// that matches, and the default stays where it is. A default that is the
/// only pin at that version moves with the minimum supported version.
///
/// Returns whether a pin matched the current version.
// setmsrv[impl pin.version]
pub(crate) fn set(configuration: &mut DocumentMut, current: &Msrv, msrv: &Msrv) -> bool {
    let Some(pins) = configuration
        .get_mut("tools")
        .and_then(|tools| tools.get_mut("rust"))
    else {
        return false;
    };

    let versions: Vec<&mut Value> = match pins {
        Item::Value(Value::Array(pins)) => pins.iter_mut().filter_map(version).collect(),
        Item::Value(pin) => version(pin).into_iter().collect(),
        Item::Table(pin) => pin
            .get_mut("version")
            .and_then(Item::as_value_mut)
            .into_iter()
            .collect(),
        Item::ArrayOfTables(pins) => pins
            .iter_mut()
            .filter_map(|pin| pin.get_mut("version").and_then(Item::as_value_mut))
            .collect(),
        Item::None => Vec::new(),
    };

    let Some(pin) = versions
        .into_iter()
        .rev()
        .find(|version| version.as_str() == Some(current.get()))
    else {
        return false;
    };

    super::replace(pin, msrv);

    true
}

/// Returns the value that holds the version of a pin
fn version(pin: &mut Value) -> Option<&mut Value> {
    match pin {
        Value::InlineTable(table) => table.get_mut("version"),
        version => Some(version),
    }
}
