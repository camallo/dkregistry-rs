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
    pub fn get_manifest(&self, name: &str, reference: &str) -> FutureManifest {
        let url = match hyper::Uri::from_str(&format!(
            "{}/v2/{}/manifests/{}",
            self.base_url.clone(),
            name,
            reference
        )) {
            Ok(url) => url,
            Err(e) => {
                let msg = format!("failed to parse Uri from str: {}", e);
                error!("{}", msg);
                return Box::new(futures::future::err::<_, _>(Error::from(msg)));
            }
        };
        let req = {
            let mut req = match self.new_request(hyper::Method::GET, url.clone()) {
                Ok(r) => r,
                Err(e) => {
                    let msg = format!("new_request failed: {}", e);
                    error!("{}", msg);
                    return Box::new(futures::future::err(Error::from(msg)));
                }
            };
            let mtype = mediatypes::MediaTypes::ManifestV2S2.to_string();
            req.headers_mut().append(
                header::ACCEPT,
                match header::HeaderValue::from_str(&mtype) {
                    Ok(headervalue) => headervalue,
                    Err(e) => {
                        let msg = format!("failed to parse HeaderValue from str: {}:", e);
                        error!("{}", msg);
                        return Box::new(futures::future::err::<_, _>(Error::from(msg)));
                    }
                },
            );
            req
        };
        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .inspect(move |_| {
                trace!("GET {:?}", url);
            })
            .and_then(|r| {
                let status = r.status();
                trace!("Got status: {:?}", status);
                match status {
                    hyper::StatusCode::OK => Ok(r),
                    _ => Err(format!("get_manifest: wrong HTTP status '{}'", status).into()),
                }
            })
            .and_then(|r| {
                r.into_body().concat2().map_err(|e| {
                    format!("get_manifest: failed to fetch the whole body: {}", e).into()
                })
            })
            .and_then(|body| Ok(body.into_bytes().to_vec()));
        Box::new(fres)
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
    ) -> mediatypes::FutureMediaType {
        let url = {
            let ep = format!(
                "{}/v2/{}/manifests/{}",
                self.base_url.clone(),
                name,
                reference
            );
            match hyper::Uri::from_str(ep.as_str()) {
                Ok(url) => url,
                Err(e) => {
                    let msg = format!("failed to parse Uri from str: {}", e);
                    error!("{}", msg);
                    return Box::new(futures::future::err::<_, _>(Error::from(msg)));
                }
            }
        };
        let accept_types = match {
            match mediatypes {
                None => {
                    if let Ok(m) = mediatypes::MediaTypes::ManifestV2S2.to_mime() {
                        Ok(vec![m])
                    } else {
                        Err(Error::from("to_mime failed"))
                    }
                }
                Some(ref v) => to_mimes(v),
            }
        } {
            Ok(x) => x,
            Err(e) => {
                return Box::new(futures::future::err::<_, _>(Error::from(format!(
                    "failed to match mediatypes: {}",
                    e
                ))));
            }
        };

        let req = {
            let mut req = match self.new_request(hyper::Method::HEAD, url.clone()) {
                Ok(r) => r,
                Err(e) => {
                    let msg = format!("new_request failed: {}", e);
                    error!("{}", msg);
                    return Box::new(futures::future::err(Error::from(msg)));
                }
            };
            for v in accept_types {
                let _ = header::HeaderValue::from_str(&v.to_string())
                    .map(|hval| req.headers_mut().append(hyper::header::ACCEPT, hval));
            }
            req
        };
        let freq = self.hclient.request(req);
        let fres = freq
            .from_err()
            .inspect(move |_| {
                trace!("HEAD {:?}", url);
            })
            .and_then(|r| {
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
        Box::new(fres)
    }
}

fn to_mimes(v: &[&str]) -> Result<Vec<mime::Mime>> {
    let res = v
        .iter()
        .filter_map(|x| {
            let mtype = mediatypes::MediaTypes::from_str(x);
            match mtype {
                Ok(m) => Some(match m.to_mime() {
                    Ok(mime) => mime,
                    Err(e) => {
                        error!("to_mime failed: {}", e);
                        return None;
                    }
                }),
                _ => None,
            }
        })
        .collect();
    Ok(res)
}
