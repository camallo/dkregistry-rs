use errors::{Error, Result};
use futures::{self, stream, Future, Stream};
use reqwest::StatusCode;
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
    pub fn get_catalog(&self, paginate: Option<u32>) -> StreamCatalog {
        let url = {
            let suffix = if let Some(n) = paginate {
                format!("?n={}", n)
            } else {
                "".to_string()
            };
            let ep = format!("{}/v2/_catalog{}", self.base_url.clone(), suffix);
            match reqwest::Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    return Box::new(stream::once::<_, _>(Err(Error::from(format!(
                        "failed to parse url from string '{}': {}",
                        ep, e
                    )))));
                }
            }
        };

        let req = self.build_reqwest(reqwest::async::Client::new().get(url));

        let fres = req
            .send()
            .from_err()
            .and_then(|r| {
                let status = r.status();
                trace!("Got status: {:?}", status);
                match status {
                    StatusCode::OK => Ok(r),
                    _ => Err(format!("get_catalog: wrong HTTP status '{}'", status).into()),
                }
            })
            .and_then(|r| {
                r.into_body().concat2().map_err(|e| {
                    format!("get_catalog: failed to fetch the whole body: {}", e).into()
                })
            })
            .and_then(|body| -> Result<Catalog> {
                serde_json::from_slice(&body).map_err(|e| e.into())
            })
            .map(|cat| futures::stream::iter_ok(cat.repositories.into_iter()))
            .flatten_stream();
        Box::new(fres)
    }
}
