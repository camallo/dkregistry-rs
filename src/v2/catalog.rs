use futures::Stream;
use v2::*;

/// Convenience alias for a stream of `String` repos.
pub type StreamCatalog = Box<futures::Stream<Item = String, Error = Error>>;

#[derive(Debug, Default, Deserialize, Serialize)]
struct Catalog {
    pub repositories: Vec<String>,
}

impl Client {
    pub fn get_catalog(&self, paginate: Option<u32>) -> Result<StreamCatalog> {
        let url = {
            let mut s = self.base_url.clone() + "/v2/_catalog";
            if let Some(n) = paginate {
                s = s + &format!("?n={}", n);
            };
            try!(hyper::Uri::from_str(s.as_str()))
        };
        let req = self.new_request(hyper::Method::Get, url);
        let freq = self.hclient.request(req);
        let fres = freq
            .and_then(|resp| {
                if resp.status() != hyper::StatusCode::Ok {
                    return Err(hyper::Error::Status);
                };
                Ok(resp)
            }).and_then(|r| {
                r.body().fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    futures::future::ok::<_, hyper::Error>(v)
                })
            }).map_err(|e| e.into())
            .and_then(|chunks| String::from_utf8(chunks).map_err(|e| e.into()))
            .and_then(|body| -> Result<Catalog> {
                serde_json::from_slice(body.as_bytes()).map_err(|e| e.into())
            }).map(|cat| -> Vec<Result<String>> {
                cat.repositories.into_iter().map(|r| Ok(r)).collect()
            }).map(|rs| futures::stream::iter(rs.into_iter()))
            .flatten_stream();
        return Ok(Box::new(fres));
    }
}
