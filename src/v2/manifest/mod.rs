//! Manifest types.
use mediatypes;
use v2::*;

use futures::{future, Stream};
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
        Box::new(
            self.get_manifest_and_ref(name, reference)
                .map(|(manifest, _)| manifest),
        )
    }

    /// Fetch an image manifest and return it with its digest.
    ///
    /// The name and reference parameters identify the image.
    /// The reference may be either a tag or digest.
    pub fn get_manifest_and_ref(&self, name: &str, reference: &str) -> FutureManifestAndRef {
        let url = {
            let ep = format!(
                "{}/v2/{}/manifests/{}",
                self.base_url.clone(),
                name,
                reference
            );
            match reqwest::Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    return Box::new(future::err::<_, _>(Error::from(format!(
                        "failed to parse url from string '{}': {}",
                        ep, e
                    ))));
                }
            }
        };

        let mtype = match header::HeaderValue::from_str(
            &mediatypes::MediaTypes::ManifestV2S2.to_string(),
        ) {
            Ok(headervalue) => headervalue,
            Err(e) => {
                let msg = format!("failed to parse HeaderValue from str: {}:", e);
                error!("{}", msg);
                return Box::new(futures::future::err::<_, _>(Error::from(msg)));
            }
        };

        let fres = self
            .build_reqwest(
                reqwest::async::Client::new()
                    .get(url)
                    .header(reqwest::header::ACCEPT, mtype),
            )
            .send()
            .map_err(|e| Error::from(format!("{}", e)))
            .and_then(|res| {
                trace!("GET '{}' status: {:?}", res.url(), res.status());
                let status = res.status();
                trace!("Got status: {:?}", status);
                match status {
                    reqwest::StatusCode::OK => Ok(res),
                    _ => Err(format!("GET {}: wrong HTTP status '{}'", res.url(), status).into()),
                }
            })
            .and_then(|res| {
                future::ok(res.headers().clone()).join(
                    res.into_body()
                        .concat2()
                        .map_err(|e| Error::from(format!("{}", e))),
                )
            })
            .map_err(|e| Error::from(format!("{}", e)))
            .and_then(|(headers, body)| {
                Ok((
                    body.to_vec(),
                    headers
                        .get("docker-content-digest")
                        .ok_or(Error::from("cannot find manifestref in headers"))?
                        .to_str()?
                        .to_string(),
                ))
            });
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
                    return Box::new(future::err::<_, _>(Error::from(msg)));
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
                return Box::new(future::err::<_, _>(Error::from(format!(
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
                    return Box::new(future::err(Error::from(msg)));
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
