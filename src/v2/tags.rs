use futures::prelude::*;
use hyper::{self, header};
use v2::*;

/// Convenience alias for a stream of `String` tags.
pub type StreamTags = Box<futures::Stream<Item = String, Error = Error>>;

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
    pub fn get_tags(&self, name: &str, paginate: Option<u32>) -> StreamTags {
        let dclient = self.clone();
        let base_url = format!("{}/v2/{}/tags/list", self.base_url, name);

        let fres = futures::stream::unfold(Some(String::new()), move |link| {
            // Stream ends when response has no `Link` header.
            let last = match link {
                None => return None,
                Some(ref s) if s == "" => None,
                s => s,
            };

            let full_url = match (paginate, last) {
                (Some(p), None) => format!("{}?n={}", base_url, p),
                (None, Some(l)) => format!("{}?next_page={}", base_url, l),
                (Some(p), Some(l)) => format!("{}?n={}&next_page={}", base_url, p, l),
                _ => base_url.to_string(),
            };
            let client = dclient.clone();
            let url = hyper::Uri::from_str(&full_url);

            let freq = futures::future::result(url)
                .from_err()
                .and_then(move |url| {
                    trace!("GET {:?}", &url);
                    let req = client.new_request(hyper::Method::GET, url);
                    futures::future::result(req)
                        .and_then(move |req| client.hclient.request(req).from_err())
                }).and_then(|resp| {
                    let status = resp.status();
                    match status {
                        hyper::StatusCode::OK => Ok(resp),
                        _ => Err(format!("get_tags: wrong HTTP status '{}'", status).into()),
                    }
                }).and_then(|resp| {
                    let ct_hdr = resp.headers().get(header::CONTENT_TYPE).cloned();
                    let ok = match ct_hdr {
                        None => false,
                        Some(ref ct) => ct.to_str()?.starts_with("application/json"),
                    };
                    if !ok {
                        return Err(format!("get_tags: wrong content type '{:?}'", ct_hdr).into());
                    }
                    Ok(resp)
                }).and_then(|resp| {
                    let hdr = resp.headers().get(header::LINK).cloned();
                    trace!("next_page {:?}", hdr);
                    resp.into_body()
                        .concat2()
                        .map_err(|e| {
                            format!("get_tags: failed to fetch the whole body: {}", e).into()
                        }).and_then(move |body| Ok((body, parse_link(hdr))))
                }).and_then(|(body, hdr)| -> Result<(TagsChunk, Option<String>)> {
                    serde_json::from_slice(&body.into_bytes())
                        .map_err(|e| e.into())
                        .map(|tags_chunk| (tags_chunk, hdr))
                }).map(|(tags_chunk, last)| {
                    (futures::stream::iter_ok(tags_chunk.tags.into_iter()), last)
                });
            Some(freq)
        }).flatten();

        Box::new(fres)
    }
}

/// Parse a `Link` header.
///
/// Format is described at https://docs.docker.com/registry/spec/api/#listing-image-tags#pagination.
fn parse_link(hdr: Option<header::HeaderValue>) -> Option<String> {
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
    let uri = sval.trim_right_matches(">; rel=\"next\"");
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
