use crate::errors::{Error, Result};
use crate::v2::*;
use futures::stream::{self, BoxStream, StreamExt};
use reqwest::{self, header, Url};
use std::fmt::Debug;
use std::pin::Pin;

/// Convenience alias for a stream of `String` tags.
pub type StreamTags<'a> = BoxStream<'a, Result<String>>;

/// A chunk of tags for an image.
///
/// This contains a non-strict subset of the whole list of tags
/// for an image, depending on pagination option at request time.
#[derive(Debug, Default, Deserialize, Serialize)]
struct TagsChunk {
    /// Image repository name.
    name: String,
    /// Subset of tags.
    tags: Vec<String>,
}

impl Client {
    /// List existing tags for an image.
    pub fn get_tags<'a, 'b: 'a, 'c: 'a>(
        &'b self,
        name: &'c str,
        paginate: Option<u32>,
    ) -> StreamTags<'a> {
        let inner = stream::unfold(Some(String::new()), move |last| async move {
            let base_url = format!("{}/v2/{}/tags/list", self.base_url, name);

            // Stream ends when response has no `Link` header.
            let link = match last {
                None => return None,
                Some(ref s) if s == "" => None,
                s => s,
            };

            match self.fetch_tag(paginate, &base_url, &link).await {
                Ok((tags_chunk, next)) => Some((Ok(tags_chunk), next)),
                Err(err) => Some((Err(err), None)),
            }
        })
        .map(|r| match r {
            Ok(tags_chunk) => stream::iter(
                tags_chunk
                    .tags
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

    async fn fetch_tag(
        &self,
        paginate: Option<u32>,
        base_url: &String,
        link: &Option<String>,
    ) -> Result<(TagsChunk, Option<String>)> {
        let url_paginated = match (paginate, link) {
            (Some(p), None) => format!("{}?n={}", base_url, p),
            (None, Some(l)) => format!("{}?next_page={}", base_url, l),
            (Some(p), Some(l)) => format!("{}?n={}&next_page={}", base_url, p, l),
            _ => base_url.to_string(),
        };
        let url = Url::parse(&url_paginated).map_err(|e| Error::from(format!("{}", e)))?;

        let resp = self
            .build_reqwest(reqwest::Client::new().get(url.clone()))
            .header(header::ACCEPT, "application/json")
            .send()
            .await?
            .error_for_status()
            .map_err(|e| Error::from(format!("{}", e)))?;

        // ensure the CONTENT_TYPE header is application/json
        let ct_hdr = resp.headers().get(header::CONTENT_TYPE).cloned();

        trace!("page url {:?}", ct_hdr);

        let ok = match ct_hdr {
            None => false,
            Some(ref ct) => ct
                .to_str()
                .map_err(|e| Error::from(format!("{}", e)))?
                .starts_with("application/json"),
        };
        if !ok {
            // TODO:(steveeJ): Make this an error once Satellite
            // returns the content type correctly
            debug!("get_tags: wrong content type '{:?}', ignoring...", ct_hdr);
        }

        // extract the response body and parse the LINK header
        let next = parse_link(resp.headers().get(header::LINK));
        trace!("next_page {:?}", next);

        let tags_chunk = resp.json::<TagsChunk>().await?;
        Ok((tags_chunk, next))
    }
}

/// Parse a `Link` header.
///
/// Format is described at https://docs.docker.com/registry/spec/api/#listing-image-tags#pagination.
fn parse_link(hdr: Option<&header::HeaderValue>) -> Option<String> {
    // TODO(lucab): this a brittle string-matching parser. Investigate
    // whether there is a a common library to do this, in the future.

    // Raw Header value bytes.
    let hval = match hdr {
        Some(v) => v,
        None => return None,
    };

    // Header value string.
    let sval = match hval.to_str() {
        Ok(v) => v.to_owned(),
        _ => return None,
    };

    // Query parameters for next page URL.
    let uri = sval.trim_end_matches(">; rel=\"next\"");
    let query: Vec<&str> = uri.splitn(2, "next_page=").collect();
    let params = match query.get(1) {
        Some(v) if *v != "" => v,
        _ => return None,
    };

    // Last item in current page (pagination parameter).
    let last: Vec<&str> = params.splitn(2, '&').collect();
    match last.get(0).cloned() {
        Some(v) if v != "" => Some(v.to_string()),
        _ => None,
    }
}
