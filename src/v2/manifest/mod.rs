//! Manifest types.
use mediatypes;
use v2::*;

use futures::future::Either;
use futures::{future, Stream};
use mime;
use reqwest::{self, header, StatusCode, Url};
use std::iter::FromIterator;
use std::str::FromStr;

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
        let name = name.to_string();

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

        let accept_headers = header::HeaderMap::from_iter(
            [
                // accept header types and their q value, as documented in
                // https://tools.ietf.org/html/rfc7231#section-5.3.2
                (mediatypes::MediaTypes::ManifestV2S2, 0.5),
                (mediatypes::MediaTypes::ManifestV2S1Signed, 0.4),
                // TODO(steveeJ): uncomment this when all the Manifest methods work for it
                // mediatypes::MediaTypes::ManifestList,
            ]
            .iter()
            .filter_map(|(ty, q)| {
                match header::HeaderValue::from_str(&format!("{}; q={}", ty.to_string(), q)) {
                    Ok(header_value) => Some((header::ACCEPT, header_value)),
                    Err(e) => {
                        let msg = format!("failed to parse HeaderValue from str: {}:", e);
                        error!("{}", msg);
                        None
                    }
                }
            }),
        );

        let client_spare0 = self.clone();

        let fres = self
            .build_reqwest(
                reqwest::async::Client::new()
                    .get(url)
                    .headers(accept_headers),
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
                future::ok((res.headers().clone(), res.url().clone())).join(
                    res.into_body()
                        .concat2()
                        .map_err(|e| Error::from(format!("{}", e))),
                )
            })
            .map_err(|e| Error::from(format!("{}", e)))
            .and_then(|((headers, url), body)| {
                let content_digest = match headers.get("docker-content-digest") {
                    Some(content_digest_value) => Some(
                        content_digest_value
                            .to_str()
                            .map_err(Error::from)?
                            .to_string(),
                    ),
                    None => {
                        debug!("cannot find manifestref in headers");
                        None
                    }
                };

                let header_content_type = headers.get(header::CONTENT_TYPE);
                let media_type = evaluate_media_type(header_content_type, &url)?;

                trace!(
                    "content-type: {:?}, media-type: {:?}",
                    header_content_type,
                    media_type
                );

                Ok((body, content_digest, media_type))
            })
            .and_then(move |(body, content_digest, media_type)| {
                match media_type {
                    mediatypes::MediaTypes::ManifestV2S1Signed => {
                        Either::A(futures::future::result(
                            serde_json::from_slice::<ManifestSchema1Signed>(&body)
                                .map_err(|e| Error::from(format!("{}", e)))
                                .map(Manifest::S1Signed),
                        ))
                    }
                    mediatypes::MediaTypes::ManifestV2S2 => Either::B(
                        futures::future::result(serde_json::from_slice::<ManifestSchema2Spec>(
                            &body,
                        ))
                        .map_err(|e| Error::from(format!("{}", e)))
                        .and_then(move |m| {
                            m.fetch_config_blob(client_spare0, name.to_string())
                                .map(Manifest::S2)
                        }),
                    ),
                    mediatypes::MediaTypes::ManifestList => Either::A(futures::future::result(
                        serde_json::from_slice::<ManifestList>(&body)
                            .map_err(|e| Error::from(format!("{}", e)))
                            .map(Manifest::ML),
                    )),
                    unsupported => Either::A(future::err(Error::from(format!(
                        "unsupported mediatype '{:?}'",
                        unsupported
                    )))),
                }
                .and_then(|manifest| Ok((manifest, content_digest)))
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
            .map_err(Error::from)
            .inspect(move |_| {
                trace!("HEAD {:?}", url);
            })
            .and_then(|r| {
                let status = r.status();
                let media_type =
                    evaluate_media_type(r.headers().get(header::CONTENT_TYPE), &r.url())?;

                trace!(
                    "Manifest check status '{:?}', headers '{:?}, media-type: {:?}",
                    r.status(),
                    r.headers(),
                    media_type
                );

                let res = match status {
                    StatusCode::MOVED_PERMANENTLY
                    | StatusCode::TEMPORARY_REDIRECT
                    | StatusCode::FOUND
                    | StatusCode::OK => Some(media_type),
                    StatusCode::NOT_FOUND => None,
                    _ => bail!("has_manifest: wrong HTTP status '{}'", &status),
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

// Evaluate the `MediaTypes` from the the request header.
fn evaluate_media_type(
    content_type: Option<&http::header::HeaderValue>,
    url: &Url,
) -> Result<mediatypes::MediaTypes> {
    let header_content_type = content_type
        .map(|hv| hv.to_str())
        .map(std::result::Result::unwrap_or_default);

    let is_pulp_based = url.path().starts_with("/pulp/docker/v2");

    match (header_content_type, is_pulp_based) {
        (Some(header_value), false) => {
            mediatypes::MediaTypes::from_str(header_value).map_err(Into::into)
        }
        (None, false) => Err(Error::from(
            "no header_content_type given and no workaround to apply".to_string(),
        )),
        (Some(header_value), true) => {
            // TODO: remove this workaround once Satellite returns a proper content-type here
            match header_value {
                "application/x-troff-man" => {
                    trace!("Applying workaround for pulp-based registries, e.g. Satellite");
                    mediatypes::MediaTypes::from_str(
                        "application/vnd.docker.distribution.manifest.v1+prettyjws",
                    )
                    .map_err(Into::into)
                }
                _ => {
                    debug!("Received content-type '{}' from pulp-based registry. Feeling lucky and trying to parse it...", header_value);
                    mediatypes::MediaTypes::from_str(header_value).map_err(Into::into)
                }
            }
        }
        (None, true) => {
            trace!("Applying workaround for pulp-based registries, e.g. Satellite");
            mediatypes::MediaTypes::from_str(
                "application/vnd.docker.distribution.manifest.v1+prettyjws",
            )
            .map_err(Into::into)
        }
    }
}

/// Umbrella type for common actions on the different manifest schema types
#[derive(Debug)]
pub enum Manifest {
    S1Signed(manifest_schema1::ManifestSchema1Signed),
    S2(manifest_schema2::ManifestSchema2),
    ML(manifest_schema2::ManifestList),
}

impl Manifest {
    /// List digests of all layers referenced by this manifest, if available.
    ///
    /// The returned layers list is ordered starting with the base image first.
    pub fn layers_digests(&self, architecture: Option<&str>) -> Result<Vec<String>> {
        match (self, self.architectures(), architecture) {
            (Manifest::S1Signed(m), _, None) => Ok(m.get_layers()),
            (Manifest::S2(m), _, None) => Ok(m.get_layers()),
            (Manifest::S1Signed(m), Ok(ref self_architectures), Some(ref a)) => {
                let self_a = self_architectures
                    .first()
                    .ok_or("no architecture in manifest")?;
                ensure!(self_a == a, "architecture mismatch");
                Ok(m.get_layers())
            }
            (Manifest::S2(m), Ok(ref self_architectures), Some(ref a)) => {
                let self_a = self_architectures
                    .first()
                    .ok_or("no architecture in manifest")?;
                ensure!(self_a == a, "architecture mismatch");
                Ok(m.get_layers())
            }
            // Manifest::ML(_) => TODO(steveeJ),
            _ => Err(format!(
                "Manifest {:?} doesn't support the 'layer_digests' method",
                self
            )
            .into()),
        }
    }

    /// The architectures of the image the manifest points to, if available.
    pub fn architectures(&self) -> Result<Vec<String>> {
        match self {
            Manifest::S1Signed(m) => Ok([m.architecture.clone()].to_vec()),
            Manifest::S2(m) => Ok([m.architecture()].to_vec()),
            // Manifest::ML(_) => TODO(steveeJ),
            _ => Err(format!(
                "Manifest {:?} doesn't support the 'architecture' method",
                self
            )
            .into()),
        }
    }
}
