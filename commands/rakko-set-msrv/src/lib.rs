#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod set_msrv;

pub use self::set_msrv::{
    MiseStep, Msrv, ParseTomlError, Reason, SetMsrv, SetMsrvArgs, SetMsrvError,
};
