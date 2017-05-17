//! Docker Registry API v2.

use hyper::{self, client};
use hyper_tls;
use tokio_core::reactor;
use super::errors::*;
use futures;
use serde_json;

use std::str::FromStr;
use futures::Future;

mod config;
pub use self::config::Config;

mod catalog;
pub use self::catalog::{Catalog, StreamCatalog};

mod auth;
pub use self::auth::{TokenAuth, FutureTokenAuth};

pub mod manifest;
pub use self::manifest::{FutureManifest};

mod tags;
pub use self::tags::{Tags, StreamTags};

mod blobs;
pub use self::blobs::FutureBlob;

/// A Client to make outgoing API requests to a registry.
#[derive(Clone,Debug)]
pub struct Client {
    base_url: String,
    credentials: Option<(String, String)>,
    hclient: client::Client<hyper_tls::HttpsConnector>,
    index: String,
    user_agent: Option<String>,
    token: Option<String>,
}

pub type FutureBool = Box<futures::Future<Item = bool, Error = Error>>;

impl Client {
    pub fn configure(handle: &reactor::Handle) -> Config {
        Config::default(handle)
    }

    fn new_request(&self, method: hyper::Method, url: hyper::Uri) -> hyper::client::Request {
        let mut req = client::Request::new(method, url);
        let host = hyper::header::Host::new(self.index.clone(), None);
        req.headers_mut().set(host);
        if let Some(ref t) = self.token {
            req.headers_mut()
                .set(hyper::header::Authorization(hyper::header::Bearer { token: t.to_owned() }));
        };
        if let Some(ref ua) = self.user_agent {
            req.headers_mut()
                .set(hyper::header::UserAgent::new(ua.to_owned()));
        };
        return req;
    }

    pub fn is_v2_supported(&self) -> Result<FutureBool> {
        let api_header = "Docker-Distribution-API-Version";
        let api_version = "registry/2.0";

        let url = try!(hyper::Uri::from_str((self.base_url.clone() + "/v2/").as_str()));
        let req = self.new_request(hyper::Method::Get, url.clone());
        let freq = self.hclient.request(req);
        let fres = freq.map(move |r| {
                                trace!("GET {:?}", url);
                                r
                            })
            .and_then(move |r| match (r.status(), r.headers().get_raw(api_header)) {
                          (hyper::status::StatusCode::Ok, Some(x)) => Ok(x == api_version),
                          (hyper::status::StatusCode::Unauthorized, Some(x)) => {
                              Ok(x == api_version)
                          }
                          (s, v) => {
                trace!("Got status {}, header version {:?}", s, v);
                Ok(false)
            }
                      })
            .and_then(|b| {
                          trace!("v2 API supported: {}", b);
                          Ok(b)
                      })
            .from_err();
        return Ok(Box::new(fres));
    }
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct ApiError {
    code: String,
    message: String,
    detail: String,
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Errors {
    errors: Vec<ApiError>,
}
