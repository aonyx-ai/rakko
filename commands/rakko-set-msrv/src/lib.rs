#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

/// The command, the arguments that a run reads, and the errors that stop a run
mod set_msrv;

pub use self::set_msrv::{
    MiseStep, Msrv, ParseTomlError, Reason, SetMsrv, SetMsrvArgs, SetMsrvError,
};
