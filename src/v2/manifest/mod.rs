//! Manifest types.
use mediatypes;
use v2::*;

use futures::{future, Stream};
use mime;
use reqwest::{self, header, StatusCode, Url};

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
                    .header(header::ACCEPT, mtype),
            )
            .send()
            .map_err(|e| Error::from(format!("{}", e)))
            .and_then(|res| {
                let status = res.status();
                trace!("GET '{}' status: {:?}", res.url(), status);

                match status {
                    StatusCode::OK => Ok(res),
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
                let content_digest = match headers.get("docker-content-digest") {
                    Some(content_digest_value) => Some(content_digest_value.to_str()?.to_string()),
                    None => {
                        debug!("cannot find manifestref in headers");
                        None
                    }
                };
                Ok((body.to_vec(), content_digest))
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
            match Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    return Box::new(future::err::<_, _>(Error::from(format!(
                        "failed to parse url from string '{}': {}",
                        ep, e
                    ))));
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

        let mut accept_headers = header::HeaderMap::with_capacity(accept_types.len());
        for accept_type in accept_types {
            match header::HeaderValue::from_str(&accept_type.to_string()) {
                Ok(header_value) => accept_headers.insert(header::ACCEPT, header_value),
                Err(e) => {
                    return Box::new(future::err::<_, _>(Error::from(format!(
                        "failed to parse mime '{}' as accept_header: {}",
                        accept_type, e
                    ))));
                }
            };
        }

        let fres = self
            .build_reqwest(reqwest::async::Client::new().get(url.clone()))
            .headers(accept_headers)
            .send()
            .map_err(|e| Error::from(format!("{}", e)))
            .inspect(move |_| {
                trace!("HEAD {:?}", url);
            })
            .and_then(|r| {
                let status = r.status();
                let ct = {
                    let header_content_type = match r.headers().get(header::CONTENT_TYPE) {
                        Some(header_value) => Some(header_value.to_str()?),
                        None => None,
                    };

                    let is_pulp_based = r.url().path().starts_with("/pulp/docker/v2");

                    match (header_content_type, is_pulp_based) {
                        (Some(header_value), false) => mediatypes::MediaTypes::from_str(header_value).ok(),
                        (None, false) => None,
                        (Some(header_value), true)  =>  {
                            // TODO: remove this workaround once Satellite returns a proper content-type here
                            match header_value {
                            "application/x-troff-man" => {
                                trace!("Applying workaround for pulp-based registries, e.g. Satellite");
                                mediatypes::MediaTypes::from_str("application/vnd.docker.distribution.manifest.v1+prettyjws").ok()
                                },
                                _ => {
                                    debug!("Received content-type '{}' from pulp-based registry. Feeling lucky and trying to parse it...", header_value);
                                    mediatypes::MediaTypes::from_str(header_value).ok()
                                },
                            }
                        },
                        (None, true) => {
                            trace!("Applying workaround for pulp-based registries, e.g. Satellite");
                            mediatypes::MediaTypes::from_str("application/vnd.docker.distribution.manifest.v1+prettyjws").ok()
                        },

                    }
                };

                trace!("Manifest check status '{:?}', headers '{:?}, content-type: {:?}", r.status(), r.headers(), ct);
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
