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
use hyper::{self, client, header};
use hyper_rustls;
use serde_json;

use std::str::FromStr;

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

/// A Client to make outgoing API requests to a registry.
#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    credentials: Option<(String, String)>,
    hclient: client::Client<hyper_rustls::HttpsConnector<client::HttpConnector>>,
    index: String,
    user_agent: Option<String>,
    token: Option<String>,
}

/// Convenience alias for a future boolean result.
pub type FutureBool = Box<Future<Item = bool, Error = Error>>;

/// Convenience alias for a future manifest blob.
pub type FutureManifest = Box<Future<Item = Vec<u8>, Error = Error>>;

/// Convenience alias for a future manifest blob and ref.
pub type FutureManifestAndRef = Box<Future<Item = (Vec<u8>, String), Error = Error>>;

impl Client {
    pub fn configure() -> Config {
        Config::default()
    }

    fn new_request(
        &self,
        method: hyper::Method,
        url: hyper::Uri,
    ) -> Result<hyper::Request<hyper::Body>> {
        let mut req = hyper::Request::default();
        *req.method_mut() = method;
        *req.uri_mut() = url;
        req.headers_mut()
            .append(header::HOST, header::HeaderValue::from_str(&self.index)?);
        if let Some(ref t) = self.token {
            let bearer = format!("Bearer {}", t);
            req.headers_mut().append(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&bearer)?,
            );
        };
        if let Some(ref ua) = self.user_agent {
            req.headers_mut()
                .append(header::USER_AGENT, header::HeaderValue::from_str(ua)?);
        };
        Ok(req)
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
    pub fn is_v2_supported(&self) -> FutureBool {
        let api_header = "Docker-Distribution-API-Version";
        let api_version = "registry/2.0";

        let url = match hyper::Uri::from_str((self.base_url.clone() + "/v2/").as_str()) {
            Ok(url) => url,
            Err(e) => {
                return Box::new(futures::future::err::<_, _>(Error::from(format!(
                    "failed to parse url from string: {}",
                    e
                ))));
            }
        };
        let req = match self.new_request(hyper::Method::GET, url.clone()) {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("new_request failed: {}", e);
                error!("{}", msg);
                return Box::new(futures::future::err::<_, _>(Error::from(msg)));
            }
        };
        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .inspect(move |_| {
                trace!("GET {:?}", url);
            })
            .and_then(move |r| match (r.status(), r.headers().get(api_header)) {
                (hyper::StatusCode::OK, Some(x)) => Ok(x == api_version),
                (hyper::StatusCode::UNAUTHORIZED, Some(x)) => Ok(x == api_version),
                (s, v) => {
                    trace!("Got status {}, header version {:?}", s, v);
                    Ok(false)
                }
            })
            .inspect(|b| {
                trace!("v2 API supported: {}", b);
            });
        Box::new(fres)
    }

    /// Takes reqwest's async RequestBuilder and injects an authentication header if a token is present
    fn build_reqwest(
        &self,
        client: reqwest::async::RequestBuilder,
    ) -> reqwest::async::RequestBuilder {
        if let Some(token) = &self.token {
            client.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
        } else {
            client
        }
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
