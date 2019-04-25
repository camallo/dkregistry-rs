//! Client library for Docker Registry API v2.
//!
//! This module provides a `Client` which can be used to list
//! images and tags, to check for the presence of blobs (manifests,
//! layers and other objects) by digest, and to retrieve them.
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
//! // Retrieve an image manifest.
//! let mut runtime = Runtime::new()?;
//! let dclient = Client::configure()
//!                      .registry("quay.io")
//!                      .build()?;
//! let fetch = dclient.get_manifest("coreos/etcd", "v3.1.0");
//! let manifest = runtime.block_on(fetch)?;
//! #
//! # Ok(())
//! # };
//! # run().unwrap();
//! # }
//! ```

use super::errors::*;
use futures::prelude::*;
use reqwest::StatusCode;
use serde_json;

mod config;
pub use self::config::Config;

mod catalog;
pub use self::catalog::StreamCatalog;

mod auth;
pub use self::auth::{FutureTokenAuth, TokenAuth};

pub mod manifest;

mod tags;
pub use self::tags::StreamTags;

mod blobs;
pub use self::blobs::FutureBlob;

mod content_digest;

/// A Client to make outgoing API requests to a registry.
#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    credentials: Option<(String, String)>,
    index: String,
    user_agent: Option<String>,
    token: Option<String>,
}

/// Convenience alias for a future boolean result.
pub type FutureBool = Box<Future<Item = bool, Error = Error>>;

/// Convenience alias for a future manifest blob.
pub type FutureManifest = Box<Future<Item = Vec<u8>, Error = Error>>;

/// Convenience alias for a future manifest blob and ref.
pub type FutureManifestAndRef = Box<Future<Item = (Vec<u8>, Option<String>), Error = Error>>;

impl Client {
    pub fn configure() -> Config {
        Config::default()
    }

    /// Ensure remote registry supports v2 API.
    pub fn ensure_v2_registry(self) -> impl Future<Item = Self, Error = Error> {
        self.is_v2_supported()
            .map(move |ok| (ok, self))
            .and_then(|(ok, client)| {
                if !ok {
                    bail!("remote server does not support docker-registry v2 API")
                } else {
                    Ok(client)
                }
            })
    }

    /// Check whether remote registry supports v2 API.
    pub fn is_v2_supported(&self) -> impl Future<Item = bool, Error = Error> {
        let api_header = "Docker-Distribution-API-Version";
        let api_version = "registry/2.0";

        // GET request to bare v2 endpoint.
        let v2_endpoint = format!("{}/v2/", self.base_url);
        let get_v2 = reqwest::Url::parse(&v2_endpoint)
            .chain_err(|| format!("failed to parse url string '{}'", &v2_endpoint))
            .map(|url| {
                trace!("GET {:?}", url);
                self.build_reqwest(reqwest::async::Client::new().get(url))
            })
            .into_future()
            .and_then(|req| req.send().from_err());

        // Check status code and API headers according to spec:
        // https://docs.docker.com/registry/spec/api/#api-version-check
        get_v2
            .and_then(move |r| match (r.status(), r.headers().get(api_header)) {
                (StatusCode::OK, Some(x)) => Ok(x == api_version),
                (StatusCode::UNAUTHORIZED, Some(x)) => Ok(x == api_version),
                (s, v) => {
                    trace!("Got unexpected status {}, header version {:?}", s, v);
                    Ok(false)
                }
            })
            .inspect(|b| {
                trace!("v2 API supported: {}", b);
            })
    }

    /// Takes reqwest's async RequestBuilder and injects an authentication header if a token is present
    fn build_reqwest(
        &self,
        req_builder: reqwest::async::RequestBuilder,
    ) -> reqwest::async::RequestBuilder {
        let mut builder = req_builder;

        if let Some(token) = &self.token {
            builder = builder.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
        }
        if let Some(ua) = &self.user_agent {
            builder = builder.header(reqwest::header::USER_AGENT, ua.as_str());
        };

        builder
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ApiError {
    code: String,
    message: String,
    detail: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Errors {
    errors: Vec<ApiError>,
}
