//! A pure-Rust asynchronous library for Docker Registry API.
//!
//! It provides support for asynchronous interaction with
//! container registries conformant to the Docker Registry HTTP API V2.

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio_core;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
#[macro_use(slog_trace, slog_log, slog_record, slog_record_static, slog_b, slog_kv)]
extern crate slog;
#[macro_use]
extern crate slog_scope;

mod errors;
pub use errors::*;

pub mod mediatypes;
pub mod v2;

/// Default User-Agent client identity.
pub static USER_AGENT: &'static str = "camallo-dkregistry/0.0";
