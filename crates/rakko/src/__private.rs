//! The items that the macros of this crate call
//!
//! A macro expands in the crate that calls it, so the items that its
//! expansion names must be public. They are no part of the API of the crate,
//! and a release can change them without notice.

use crate::tool::Tools;
pub use crate::tool::{MiseText, ToolPin};

/// Returns the tools of the pins that [`MiseText::pins`] returned
///
/// # Examples
///
/// ```
/// use rakko::__private::{MiseText, ToolPin};
///
/// const PINS: [ToolPin; 1] = MiseText::new("[tools]\ntaplo = \"0.10.0\"\n").pins();
///
/// assert_eq!(rakko::__private::tools(&PINS).requirements().len(), 1);
/// ```
#[must_use]
pub const fn tools(pins: &'static [ToolPin]) -> Tools {
    Tools::from_pins(pins)
}
