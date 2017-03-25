use v2::*;

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Manifest {
    name: String,
    tag: String,
    history: String,
    signature: String,
}
pub type FutureManifest = Box<futures::Future<Item = Manifest, Error = Error>>;

impl Client {

    /// Fetch an image manifest.
    ///
    /// The name and reference parameters identify the image.
    /// The reference may be either a tag or digest.
    pub fn get_manifest(&self, name: &str, reference: &str) -> Result<FutureManifest> {
        let url = try!(hyper::Uri::from_str(&format!("{}/v2/{}/manifests/{}",
                                                     self.base_url.clone(),
                                                     name,
                                                     reference)));
        let req = self.new_request(hyper::Method::Get, url);
        let freq = self.hclient.request(req);
        let fres = freq.map_err(|e| e.into())
            .and_then(move |r| {
                          if r.status() != &hyper::status::StatusCode::Ok {
                              return Err(hyper::Error::Status);
                          };
                          Ok(r)
                      })
            .and_then(move |r| {
                          r.body().fold(Vec::new(), |mut v, chunk| {
                    v.extend(&chunk[..]);
                    futures::future::ok::<_, hyper::Error>(v)
                })
                      })
            .and_then(|chunks| {
                          let s = String::from_utf8(chunks).unwrap();
                          Ok(s)
                      })
            .and_then(move |body| {
                          serde_json::from_slice(body.as_bytes()).map_err(|_| hyper::Error::Status)
                      })
            .map_err(|e| e.into());
        return Ok(Box::new(fres));
    }

    /// Check if an image manifest exists.
    ///
    /// The name and reference parameters identify the image.
    /// The reference may be either a tag or digest.
    pub fn has_manifest(&self, name: &str, reference: &str) -> Result<FutureBool> {
        let url = {
            let ep = format!("{}/v2/{}/manifests/{}",
                             self.base_url.clone(),
                             name,
                             reference);
            try!(hyper::Uri::from_str(ep.as_str()))
        };
        let req = self.new_request(hyper::Method::Head, url);
        let freq = self.hclient.request(req);
        let fres = freq.and_then(|r| match r.status() {
                                     &hyper::status::StatusCode::Ok => Ok(true),
                                     &hyper::status::StatusCode::NotFound => Ok(false),
                                     _ => Err(hyper::Error::Status),
                                 })
            .map_err(|e| e.into());
        return Ok(Box::new(fres));
    }
}
