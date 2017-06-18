use v2::*;
use futures::Stream;

/// Convenience alias for a stream of `String` tags.
pub type StreamTags = Box<futures::Stream<Item = String, Error = Error>>;

#[derive(Debug,Default,Deserialize,Serialize)]
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
        let req = self.new_request(hyper::Method::Get, url);
        let freq = self.hclient.request(req);
        let fres = freq.and_then(|r| {
                if r.status() != hyper::StatusCode::Ok {
                    return Err(hyper::Error::Status);
                };
                let ok = match r.headers().get_raw("Content-type") {
                    None => false,
                    Some(ct) => ct.iter().any(|v| v == b"application/json"),
                };
                if !ok {
                    return Err(hyper::Error::Header);
                }
                Ok(r)
            })
            .and_then(|r| {
                          r.body()
                              .fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    futures::future::ok::<_, hyper::Error>(v)
                })
                      })
            .from_err()
            .and_then(|chunks| String::from_utf8(chunks).map_err(|e| e.into()))
            .and_then(|body| -> Result<Tags> {
                          serde_json::from_slice(body.as_bytes()).map_err(|e| e.into())
                      })
            .map(|ts| -> Vec<Result<String>> { ts.tags.into_iter().map(|r| Ok(r)).collect() })
            .map(|rs| futures::stream::iter(rs.into_iter()))
            .flatten_stream();
        Ok(Box::new(fres))
    }
}
