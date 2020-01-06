//! A pure-Rust asynchronous library for Docker Registry API.
//!
//! This library provides support for asynchronous interaction with
//! container registries conformant to the Docker Registry HTTP API V2.
//!
//! ## Example
//!
//! ```rust,no_run
//! # extern crate dkregistry;
//! # extern crate tokio;
//! # fn main() {
//! # fn run() -> dkregistry::errors::Result<()> {
//! #
//! use tokio::runtime::current_thread::Runtime;
//! use dkregistry::v2::Client;
//!
//! // Check whether a registry supports API v2.
//! let host = "quay.io";
//! let mut runtime = Runtime::new()?;
//! let dclient = Client::configure()
//!                      .insecure_registry(false)
//!                      .registry(host)
//!                      .build()?;
//! let check = dclient.is_v2_supported();
//! match runtime.block_on(check)? {
//!     false => println!("{} does NOT support v2", host),
//!     true => println!("{} supports v2", host),
//! };
//! #
//! # Ok(())
//! # };
//! # run().unwrap();
//! # }
//! ```

#![deny(missing_debug_implementations)]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate strum_macros;

pub mod errors;
pub mod mediatypes;
pub mod reference;
pub mod render;
pub mod v2;

use errors::Result;
use std::collections::HashMap;
use std::io::Read;

/// Default User-Agent client identity.
pub static USER_AGENT: &str = "camallo-dkregistry/0.0";

/// Get registry credentials from a JSON config reader.
///
/// This is a convenience decoder for docker-client credentials
/// typically stored under `~/.docker/config.json`.
pub fn get_credentials<T: Read>(
    reader: T,
    index: &str,
) -> Result<(Option<String>, Option<String>)> {
    let map: Auths = serde_json::from_reader(reader)?;
    let real_index = match index {
        // docker.io has some special casing in config.json
        "docker.io" | "registry-1.docker.io" => "https://index.docker.io/v1/",
        other => other,
    };
    let auth = match map.auths.get(real_index) {
        Some(x) => base64::decode(x.auth.as_str())?,
        None => bail!("no auth for index {}", real_index),
    };
    let s = String::from_utf8(auth)?;
    let creds: Vec<&str> = s.splitn(2, ':').collect();
    let up = match (creds.get(0), creds.get(1)) {
        (Some(&""), Some(p)) => (None, Some(p.to_string())),
        (Some(u), Some(&"")) => (Some(u.to_string()), None),
        (Some(u), Some(p)) => (Some(u.to_string()), Some(p.to_string())),
        (_, _) => (None, None),
    };
    trace!("Found credentials for user={:?} on {}", up.0, index);
    Ok(up)
}

#[derive(Debug, Deserialize, Serialize)]
struct Auths {
    auths: HashMap<String, AuthObj>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct AuthObj {
    auth: String,
}
