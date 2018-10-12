//! Manifest types.
use mediatypes;
use v2::*;

use futures::Stream;
use hyper::header;
use hyper::StatusCode;
use mime;

mod manifest_schema1;
pub use self::manifest_schema1::*;

mod manifest_schema2;
pub use self::manifest_schema2::*;

impl Client {
    /// Fetch an image manifest.
    ///
    /// The name and reference parameters identify the image.
    /// The reference may be either a tag or digest.
    pub fn get_manifest(&self, name: &str, reference: &str) -> Result<FutureManifest> {
        let url = try!(hyper::Uri::from_str(&format!(
            "{}/v2/{}/manifests/{}",
            self.base_url.clone(),
            name,
            reference
        )));
        let req = {
            let mut r = self.new_request(hyper::Method::GET, url.clone());
            let mtype = mediatypes::MediaTypes::ManifestV2S2.to_string();
            r.headers_mut()
                .append(header::ACCEPT, header::HeaderValue::from_str(&mtype)?);
            r
        };
        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .inspect(move |_| {
                trace!("GET {:?}", url);
            }).and_then(|r| {
                let status = r.status();
                trace!("Got status: {:?}", status);
                match status {
                    hyper::StatusCode::OK => Ok(r),
                    _ => Err(format!("get_manifest: wrong HTTP status '{}'", status).into()),
                }
            }).and_then(|r| {
                r.into_body().concat2().map_err(|e| {
                    format!("get_manifest: failed to fetch the whole body: {}", e).into()
                })
            }).and_then(|body| Ok(body.into_bytes().to_vec()));
        Ok(Box::new(fres))
    }

    /// Check if an image manifest exists.
    ///
    /// The name and reference parameters identify the image.
    /// The reference may be either a tag or digest.
    pub fn has_manifest(
        &self,
        name: &str,
        reference: &str,
        mediatypes: Option<&[&str]>,
    ) -> Result<mediatypes::FutureMediaType> {
        let url = {
            let ep = format!(
                "{}/v2/{}/manifests/{}",
                self.base_url.clone(),
                name,
                reference
            );
            try!(hyper::Uri::from_str(ep.as_str()))
        };
        let accept_types = match mediatypes {
            None => vec![mediatypes::MediaTypes::ManifestV2S2.to_mime()],
            Some(ref v) => try!(to_mimes(v)),
        };
        let req = {
            let mut r = self.new_request(hyper::Method::HEAD, url.clone());
            for v in accept_types {
                let _ = header::HeaderValue::from_str(&v.to_string())
                    .map(|hval| r.headers_mut().append(hyper::header::ACCEPT, hval));
            }
            r
        };
        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .inspect(move |_| {
                trace!("HEAD {:?}", url);
            }).and_then(|r| {
                let status = r.status();
                let mut ct = None;
                if let Some(h) = r.headers().get(header::CONTENT_TYPE) {
                    if let Ok(s) = h.to_str() {
                        ct = mediatypes::MediaTypes::from_str(s).ok();
                    }
                }
                trace!("Manifest check result: {:?}", r.status());
                let res = match status {
                    StatusCode::MOVED_PERMANENTLY
                    | StatusCode::TEMPORARY_REDIRECT
                    | StatusCode::FOUND
                    | StatusCode::OK => ct,
                    StatusCode::NOT_FOUND => None,
                    _ => return Err(format!("has_manifest: wrong HTTP status '{}'", status).into()),
                };
                Ok(res)
            });
        Ok(Box::new(fres))
    }
}

fn to_mimes(v: &[&str]) -> Result<Vec<mime::Mime>> {
    let res = v
        .iter()
        .filter_map(|x| {
            let mtype = mediatypes::MediaTypes::from_str(x);
            match mtype {
                Ok(m) => Some(m.to_mime()),
                _ => None,
            }
        }).collect();
    Ok(res)
}
