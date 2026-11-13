#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod check_msrv;

pub use self::check_msrv::{CheckMsrv, CheckMsrvError};
