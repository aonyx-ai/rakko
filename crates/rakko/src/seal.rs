//! The seal on the views of an action that hide its type

/// A trait that only this crate implements, for every action
///
/// The view of an action that hides its type requires this trait, and no
/// other crate can name it. The view also has an implementation for every
/// action. So no other crate implements the view, and a later release can
/// add a method to it without a break.
pub trait Sealed {}

/// A trait that only this crate implements, for the bound view of an action
///
/// The view of an action whose arguments are bound requires this trait. Only
/// the type that `bind` returns implements it. So no other crate implements
/// the view, also not for a type that implements the action trait. A later
/// release can therefore add a method to the view without a break.
pub trait SealedBound {}
