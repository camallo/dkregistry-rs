//! Docker Registry API v2.

use hyper::{self, client};
use hyper_tls;
use tokio_core::reactor;
use super::errors::*;
use futures;
use serde_json;

use std::str::FromStr;
use futures::{Future, Stream};

mod config;
pub use self::config::Config;

mod auth;
pub use self::auth::{TokenAuth, FutureTokenAuth};

mod manifest;
pub use self::manifest::{Manifest, FutureManifest};

mod tags;
pub use self::tags::{Tags, FutureTags};

mod blobs;
pub use self::blobs::FutureUuid;

/// A Client to make outgoing API requests to a registry.
#[derive(Debug)]
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
        if let Some(ref t) = self.token {
            req.headers_mut().set(hyper::header::Authorization(hyper::header::Bearer {
                                                                   token: t.to_owned(),
                                                               }));
        };
        if let Some(ref ua) = self.user_agent {
            req.headers_mut().set(hyper::header::UserAgent(ua.to_owned()));
        };
        return req;
    }

    pub fn is_v2_supported(&self) -> Result<FutureBool> {
        let api_header = "Docker-Distribution-API-Version";
        let api_version = "registry/2.0";

        let url = try!(hyper::Uri::from_str((self.base_url.clone() + "/v2/").as_str()));
        let req = self.new_request(hyper::Method::Get, url);
        let freq = self.hclient.request(req);
        let fres =
            freq.and_then(move |r| match (r.status(), r.headers().get_raw(api_header)) {
                              (hyper::status::StatusCode::Ok, Some(x)) => Ok(x == api_version),
                              (hyper::status::StatusCode::Unauthorized, Some(x)) => {
                                  Ok(x == api_version)
                              }
                              (_, _) => Ok(false),
                          })
                .map_err(|e| e.into());
        return Ok(Box::new(fres));
    }

    pub fn get_catalog(&self, limit: Option<u32>) -> Result<FutureCatalog> {
        let url = {
            let mut s = self.base_url.clone() + "/v2/_catalog";
            if let Some(n) = limit {
                s = s + &format!("?n={}", n);
            };
            try!(hyper::Uri::from_str(s.as_str()))
        };
        let req = self.new_request(hyper::Method::Get, url);
        let freq = self.hclient.request(req);
        let fres = freq.map_err(|e| e.into())
            .and_then(move |r| {
                          if r.status() != hyper::status::StatusCode::Ok {
                              return Err(hyper::Error::Status);
                          };
                          Ok(r)
                      })
            .and_then(move |r| {
                          r.body().fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    futures::future::ok::<_, hyper::Error>(v)
                })
                      })
            .and_then(|chunks| {
                          let s = String::from_utf8(chunks).unwrap();
                          Ok(s)
                      })
            .and_then(move |body| {
                          serde_json::from_slice(body.as_bytes()).map_err(|_| hyper::Error::Status)
                      })
            .map_err(|e| e.into());
        return Ok(Box::new(fres));
    }
}

pub type FutureCatalog = Box<futures::Future<Item = Catalog, Error = Error>>;
#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Catalog {
    pub repositories: Vec<String>,
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
