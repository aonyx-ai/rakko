//! The seal on the run of a tool

/// A trait that only this crate implements
///
/// The `Launch` trait requires this trait, and no other crate can refer to
/// it. So only this crate implements `Launch`, and a later compatible release
/// can add a method to `Launch`.
pub trait Sealed {}
