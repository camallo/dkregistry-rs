use crate::errors::{Error, Result};
use crate::v2;
use futures::{
    self,
    stream::{self, BoxStream, StreamExt},
};
use reqwest::{RequestBuilder, StatusCode};
use std::pin::Pin;

/// Convenience alias for a stream of `String` repos.
pub type StreamCatalog<'a> = BoxStream<'a, Result<String>>;

#[derive(Debug, Default, Deserialize, Serialize)]
struct Catalog {
    pub repositories: Vec<String>,
}

impl v2::Client {
    pub fn get_catalog<'a, 'b: 'a>(&'b self, paginate: Option<u32>) -> StreamCatalog<'a> {
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
                    let b = Box::new(stream::iter(vec![Err(Error::from(format!(
                        "failed to parse url from string '{}': {}",
                        ep, e
                    )))]));
                    return unsafe { Pin::new_unchecked(b) };
                }
            }
        };

        let req = self.build_reqwest(reqwest::Client::new().get(url));
        let inner = stream::once(fetch_catalog(req))
            .map(|r| match r {
                Ok(catalog) => stream::iter(
                    catalog
                        .repositories
                        .into_iter()
                        .map(|t| Ok(t))
                        .collect::<Vec<_>>(),
                ),
                Err(err) => stream::iter(vec![Err(err)]),
            })
            .flatten();

        let b = Box::new(inner);
        unsafe { Pin::new_unchecked(b) }
    }
}

async fn fetch_catalog(req: RequestBuilder) -> Result<Catalog> {
    match req.send().await {
        Ok(r) => {
            let status = r.status();
            trace!("Got status: {:?}", status);
            match status {
                StatusCode::OK => r
                    .json::<Catalog>()
                    .await
                    .map_err(|e| format!("get_catalog: failed to fetch the whole body: {}", e)),
                _ => Err(format!("get_catalog: wrong HTTP status '{}'", status)),
            }
        }
        Err(err) => Err(format!("{}", err)),
    }
    .map_err(|e| Error::from(e))
}
