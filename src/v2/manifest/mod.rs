use v2::*;
use futures::Stream;
use hyper::header::{QualityItem, Accept, qitem};
use hyper::mime;

mod manifest_schema1;
pub use self::manifest_schema1::*;

mod manifest_schema2;
pub use self::manifest_schema2::*;

// TODO(lucab): add variants for other manifest schemas
pub type Manifest = manifest_schema1::ManifestSchema1Signed;
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
        let req = self.new_request(hyper::Method::Get, url.clone());
        let freq = self.hclient.request(req);
        let fres =
            freq.map(move |r| {
                         trace!("GET {:?}", url);
                         r
                     })
                .and_then(move |r| {
                              trace!("Got status: {:?}", r.status());
                              if r.status() != hyper::status::StatusCode::Ok {
                                  return Err(hyper::Error::Status);
                              };
                              Ok(r)
                          })
                .and_then(move |r| {
                              r.body()
                                  .fold(Vec::new(), |mut v, chunk| {
                        v.extend(&chunk[..]);
                        futures::future::ok::<_, hyper::Error>(v)
                    })
                          })
                .from_err()
            .and_then(|body| {
                serde_json::from_slice(body.as_slice()).chain_err(|| "error decoding manifest")
                          });
        return Ok(Box::new(fres));
    }

    /// Check if an image manifest exists.
    ///
    /// The name and reference parameters identify the image.
    /// The reference may be either a tag or digest.
    pub fn has_manifest(&self,
                        name: &str,
                        reference: &str,
                        mediatypes: Option<&[&str]>)
                        -> Result<FutureBool> {
        let url = {
            let ep = format!("{}/v2/{}/manifests/{}",
                             self.base_url.clone(),
                             name,
                             reference);
            try!(hyper::Uri::from_str(ep.as_str()))
        };
        let accept_types = match mediatypes {
            None => Accept(vec![
                qitem(mime!(Star/Star))
            ]),
            Some(ref v) => Accept(try!(to_mimes(v))),
        };
        let req = {
            let mut r = self.new_request(hyper::Method::Head, url);
            r.headers_mut().set(accept_types);
            r
        };
        let freq = self.hclient.request(req);
        let fres = freq.and_then(|r| match r.status() {
                                     hyper::status::StatusCode::Ok => Ok(true),
                                     hyper::status::StatusCode::NotFound => Ok(false),
                                     _ => Err(hyper::Error::Status),
                                 })
            .from_err();
        return Ok(Box::new(fres));
    }

}

fn to_mimes(v: &[&str]) -> Result<Vec<QualityItem<mime::Mime>>> {
    let res: Vec<QualityItem<mime::Mime>>;
    res = v.iter().filter_map(|x| {
        let mtype = x.to_string().parse();
        match mtype {
            Ok(m) => Some(qitem(m)),
            _ => None,
        }
    }).collect();
    Ok(res)
}
