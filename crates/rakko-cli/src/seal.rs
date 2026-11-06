/// A trait that only this crate implements
///
/// The view of a command that hides its type requires this trait, and no
/// other crate can name it. So only this crate implements the view, and a
/// later release can add a method to the view without a break.
pub trait Sealed {}
