use v2::*;
use futures::Stream;

// TODO(lucab): add variants for other manifest schemas
pub type Manifest = ManifestSchema1Signed;

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct ManifestSchema1Signed {
    #[serde(rename = "schemaVersion")]
    schema_version: u16,
    pub name: String,
    pub tag: String,
    pub architecture: String,
    #[serde(rename = "fsLayers")]
    fs_layers: Vec<Layer>,
    history: Vec<V1Compat>,
    signatures: Vec<Signature>,
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Signature {
    // TODO(lucab): switch to jsonwebtokens crate
    // https://github.com/Keats/rust-jwt/pull/23
    header: serde_json::Value,
    signature: String,
    protected: String,
}

#[derive(Debug,Deserialize,Serialize)]
pub struct V1Compat {
    #[serde(rename = "v1Compatibility")]
    v1_compat: String,
}

pub type FutureManifest = Box<futures::Future<Item = Manifest, Error = Error>>;

#[derive(Debug,Deserialize,Serialize)]
pub struct Layer {
    #[serde(rename = "blobSum")]
    blob_sum: String,
}

impl ManifestSchema1Signed {
    pub fn get_layers(&self) -> Vec<String> {
        self.fs_layers
            .iter()
            .map(|l| l.blob_sum.clone())
            .collect()
    }
}

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
                                     hyper::status::StatusCode::Ok => Ok(true),
                                     hyper::status::StatusCode::NotFound => Ok(false),
                                     _ => Err(hyper::Error::Status),
                                 })
            .from_err();
        return Ok(Box::new(fres));
    }
}
