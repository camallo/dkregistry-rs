use futures::future::{self, Either};
use futures::Stream;
use hyper::StatusCode;
use v2::*;

/// Convenience alias for future binary blob.
pub type FutureBlob = Box<futures::Future<Item = Vec<u8>, Error = Error>>;

impl Client {
    /// Check if a blob exists.
    pub fn has_blob(&self, name: &str, digest: &str) -> Result<FutureBool> {
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
            hyper::Uri::from_str(ep.as_str())?
        };
        let req = self.new_request(hyper::Method::HEAD, url.clone());
        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .inspect(move |_| {
                trace!("HEAD {:?}", url);
            }).and_then(|r| {
                trace!("Blob check result: {:?}", r.status());
                match r.status() {
                    StatusCode::MOVED_PERMANENTLY
                    | StatusCode::TEMPORARY_REDIRECT
                    | StatusCode::FOUND
                    | StatusCode::OK => Ok(true),
                    _ => Ok(false),
                }
            });
        Ok(Box::new(fres))
    }

    /// Retrieve blob.
    pub fn get_blob(&self, name: &str, digest: &str) -> Result<FutureBlob> {
        let cl = self.clone();
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url.clone(), name, digest);
            hyper::Uri::from_str(ep.as_str())?
        };
        let req = self.new_request(hyper::Method::GET, url.clone());
        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .inspect(move |_| {
                trace!("GET {:?}", url);
            }).and_then(move |r| {
                match r.status() {
                    StatusCode::MOVED_PERMANENTLY
                    | StatusCode::TEMPORARY_REDIRECT
                    | StatusCode::FOUND => {
                        trace!("Got moved status {:?}", r.status());
                    }
                    _ => return Either::A(future::ok(r)),
                };
                let redirect: Option<String> = match r.headers().get("Location") {
                    None => {
                        return Either::A(future::err(Error::from(
                            "get_blob: missing location header",
                        )))
                    }
                    Some(loc) => {
                        trace!("Got Location header {:?}", loc);
                        String::from_utf8(loc.as_bytes().to_vec()).ok()
                    }
                };
                if let Some(u) = redirect {
                    let new_url = match hyper::Uri::from_str(u.as_str()) {
                        Ok(u) => u,
                        _ => {
                            return Either::A(future::err(
                                format!("get_blob: wrong URL '{}'", u).into(),
                            ))
                        }
                    };
                    trace!("Following redirection to {}", new_url);
                    let mut req = hyper::Request::default();
                    *req.method_mut() = hyper::Method::GET;
                    *req.uri_mut() = new_url;
                    return Either::B(cl.hclient.request(req).from_err());
                };
                Either::A(future::ok(r))
            }).and_then(|r| {
                r.into_body()
                    .concat2()
                    .map_err(|e| format!("get_blob: failed to fetch the whole body: {}", e).into())
            }).and_then(|body| Ok(body.into_bytes().to_vec()));
        Ok(Box::new(fres))
    }
}
