#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

mod build_internal_docs;

pub use self::build_internal_docs::{BuildInternalDocs, BuildInternalDocsError};
