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
#[macro_use]
extern crate log;
#[macro_use]
extern crate mime;
extern crate strum;
#[macro_use]
extern crate strum_macros;

mod errors;
pub use errors::*;

pub mod mediatypes;
pub mod v2;

/// Default User-Agent client identity.
pub static USER_AGENT: &'static str = "camallo-dkregistry/0.0";
