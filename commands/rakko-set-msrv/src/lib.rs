#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

pub mod set_msrv;

pub use self::set_msrv::{MiseStep, Msrv, Reason, SetMsrv, SetMsrvArgs, SetMsrvError};
