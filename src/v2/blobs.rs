use v2::*;

/// Convenience alias for future `String` result.
pub type FutureUuid = Box<futures::Future<Item = String, Error = Error>>;

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
                                     &hyper::status::StatusCode::Ok => Ok(true),
                                     _ => Ok(false),
                                 })
            .map_err(|e| e.into());
        return Ok(Box::new(fres));
    }
}
