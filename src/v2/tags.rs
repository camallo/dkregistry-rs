use futures::{self, Stream};
use hyper::{self, header};
use v2::*;

/// Convenience alias for a stream of `String` tags.
pub type StreamTags = Box<futures::Stream<Item = String, Error = Error>>;

#[derive(Debug, Default, Deserialize, Serialize)]
struct Tags {
    name: String,
    tags: Vec<String>,
}

impl Client {
    /// List existing tags for an image.
    pub fn get_tags(&self, name: &str, paginate: Option<u32>) -> Result<StreamTags> {
        let url = {
            let mut s = format!("{}/v2/{}/tags/list", self.base_url, name);
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
                if status != hyper::StatusCode::OK {
                    return Err(format!("get_tags: wrong HTTP status '{}", status).into());
                };
                {
                    let ct_hdr = r.headers().get(header::CONTENT_TYPE);
                    let ok = match ct_hdr {
                        None => false,
                        Some(ct) => ct.to_str()?.starts_with("application/json"),
                    };
                    if !ok {
                        return Err(format!("get_tags: wrong content type '{:?}'", ct_hdr).into());
                    }
                }
                Ok(r)
            }).and_then(|r| {
                r.into_body()
                    .concat2()
                    .map_err(|e| format!("get_tags: failed to fetch the whole body: {}", e).into())
            }).and_then(|body| -> Result<Tags> {
                serde_json::from_slice(&body.into_bytes()).map_err(|e| e.into())
            }).map(|ts| futures::stream::iter_ok(ts.tags.into_iter()))
            .flatten_stream();
        Ok(Box::new(fres))
    }
}
