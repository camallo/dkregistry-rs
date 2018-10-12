//! Client library for Docker Registry API v2.
//!
//! This module provides a `Client` which can be used to list
//! images and tags, to check for the presence of blobs (manifests,
//! layers and other objects) by digest, and to retrieve them.
//!
//! ## Example
//!
//! ```rust
//! # extern crate dkregistry;
//! # extern crate tokio_core;
//! # fn main() {
//! # fn run() -> dkregistry::errors::Result<()> {
//! #
//! use tokio_core::reactor::Core;
//! use dkregistry::v2::Client;
//!
//! // Retrieve an image manifest.
//! let mut tcore = Core::new()?;
//! let dclient = Client::configure(&tcore.handle())
//!                      .registry("quay.io")
//!                      .build()?;
//! let fetch = dclient.get_manifest("coreos/etcd", "v3.1.0")?;
//! let manifest = tcore.run(fetch)?;
//! #
//! # Ok(())
//! # };
//! # run().unwrap();
//! # }
//! ```

use super::errors::*;
use futures;
use hyper::{self, client, header};
use hyper_rustls;
use serde_json;
use tokio_core::reactor;

use futures::Future;
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

pub mod blobs;
pub use self::blobs::Blob;
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
pub type FutureBool = Box<futures::Future<Item = bool, Error = Error>>;

/// Convenience alias for a future manifest blob.
pub type FutureManifest = Box<futures::Future<Item = Vec<u8>, Error = Error>>;

impl Client {
    pub fn configure(handle: &reactor::Handle) -> Config {
        Config::default(handle)
    }

    fn new_request(&self, method: hyper::Method, url: hyper::Uri) -> hyper::Request<hyper::Body> {
        // TODO(lucab): get rid of all unwraps here.
        let mut req = hyper::Request::default();
        *req.method_mut() = method;
        *req.uri_mut() = url;
        req.headers_mut().append(
            header::HOST,
            header::HeaderValue::from_str(&self.index).unwrap(),
        );
        if let Some(ref t) = self.token {
            let bearer = format!("Bearer {}", t);
            req.headers_mut().append(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&bearer).unwrap(),
            );
        };
        if let Some(ref ua) = self.user_agent {
            req.headers_mut().append(
                header::USER_AGENT,
                header::HeaderValue::from_str(ua).unwrap(),
            );
        };
        req
    }

    pub fn is_v2_supported(&self) -> Result<FutureBool> {
        let api_header = "Docker-Distribution-API-Version";
        let api_version = "registry/2.0";

        let url = try!(hyper::Uri::from_str(
            (self.base_url.clone() + "/v2/").as_str()
        ));
        let req = self.new_request(hyper::Method::GET, url.clone());
        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .inspect(move |_| {
                trace!("GET {:?}", url);
            }).and_then(move |r| match (r.status(), r.headers().get(api_header)) {
                (hyper::StatusCode::OK, Some(x)) => Ok(x == api_version),
                (hyper::StatusCode::UNAUTHORIZED, Some(x)) => Ok(x == api_version),
                (s, v) => {
                    trace!("Got status {}, header version {:?}", s, v);
                    Ok(false)
                }
            }).inspect(|b| {
                trace!("v2 API supported: {}", b);
            });
        Ok(Box::new(fres))
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
