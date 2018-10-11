use errors::{Error, Result};
use futures::{self, Future, Stream};
use hyper;
use serde_json;
use std::str::FromStr;
use v2;

/// Convenience alias for a stream of `String` repos.
pub type StreamCatalog = Box<futures::Stream<Item = String, Error = Error>>;

#[derive(Debug, Default, Deserialize, Serialize)]
struct Catalog {
    pub repositories: Vec<String>,
}

impl v2::Client {
    pub fn get_catalog(&self, paginate: Option<u32>) -> Result<StreamCatalog> {
        let url = {
            let mut s = self.base_url.clone() + "/v2/_catalog";
            if let Some(n) = paginate {
                s = s + &format!("?n={}", n);
            };
            try!(hyper::Uri::from_str(s.as_str()))
        };
        let req = self.new_request(hyper::Method::GET, url);
        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .and_then(|r| {
                let status = r.status();
                trace!("Got status: {:?}", status);
                match status {
                    hyper::StatusCode::OK => Ok(r),
                    _ => Err(format!("get_catalog: wrong HTTP status '{}'", status).into()),
                }
            }).and_then(|r| {
                r.into_body().concat2().map_err(|e| {
                    format!("get_catalog: failed to fetch the whole body: {}", e).into()
                })
            }).and_then(|body| -> Result<Catalog> {
                serde_json::from_slice(&body.into_bytes()).map_err(|e| e.into())
            }).map(|cat| futures::stream::iter_ok(cat.repositories.into_iter()))
            .flatten_stream();
        Ok(Box::new(fres))
    }
}
