use v2::*;
use hyper::status::StatusCode;
use futures::future::{self, Either};
use futures::Stream;

/// Convenience alias for future binary blob.
pub type FutureBlob = Box<futures::Future<Item = Vec<u8>, Error = Error>>;

impl Client {
    /// Check if a blob exists.
    pub fn has_blob(&self, name: &str, digest: &str) -> Result<FutureBool> {
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url.clone(), name, digest);
            try!(hyper::Uri::from_str(ep.as_str()))
        };
        let req = self.new_request(hyper::Method::Head, url);
        let freq = self.hclient.request(req);
        let fres = freq.and_then(|r| match r.status() {
                                     hyper::status::StatusCode::Ok => Ok(true),
                                     _ => Ok(false),
                                 })
            .map_err(|e| e.into());
        return Ok(Box::new(fres));
    }

    /// Retrieve blob.
    pub fn get_blob(&self, name: &str, digest: &str) -> Result<FutureBlob> {
        let cl = self.clone();
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url.clone(), name, digest);
            try!(hyper::Uri::from_str(ep.as_str()))
        };
        let req = self.new_request(hyper::Method::Get, url);
        let freq = self.hclient.request(req);
        let fres = freq.and_then(move |r| {
                match r.status() {
                    StatusCode::MovedPermanently |
                    StatusCode::Found => {}
                    _ => return Either::A(future::ok(r)),
                };
                let redirect: Option<String> = match r.headers().get_raw("Location") {
                    None => return Either::A(future::result(Err(hyper::error::Error::Status))),
                    Some(ct) => {
                        ct.clone()
                            .one()
                            .and_then(|h| String::from_utf8(h.to_vec()).ok())
                    }
                };
                if let Some(u) = redirect {
                    // TODO(lucab): get rid of this unwrap!
                    let ur = hyper::Uri::from_str(u.as_str()).unwrap();
                    let req = client::Request::new(hyper::Method::Get, ur);
                    return Either::B(cl.hclient.request(req));
                };
                Either::A(future::ok(r))
            })
            .and_then(|r| {
                          r.body()
                              .fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    futures::future::ok::<_, hyper::Error>(v)
                })
                      })
            .from_err();
        return Ok(Box::new(fres));
    }
}
