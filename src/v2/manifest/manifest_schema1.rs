use v2::*;

/// Manifest version 2 schema 1, signed.
///
/// Specification is at https://docs.docker.com/registry/spec/manifest-v2-1/.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ManifestSchema1Signed {
    #[serde(rename = "schemaVersion")]
    schema_version: u16,
    pub name: String,
    pub tag: String,
    pub architecture: String,
    #[serde(rename = "fsLayers")]
    fs_layers: Vec<S1Layer>,
    history: Vec<V1Compat>,
    signatures: Vec<Signature>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Signature {
    // TODO(lucab): switch to jsonwebtokens crate
    // https://github.com/Keats/rust-jwt/pull/23
    header: serde_json::Value,
    signature: String,
    protected: String,
}

/// Compatibility entry for version 1 manifest interoperability.
#[derive(Debug, Deserialize, Serialize)]
struct V1Compat {
    #[serde(rename = "v1Compatibility")]
    v1_compat: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct S1Layer {
    #[serde(rename = "blobSum")]
    blob_sum: String,
}

impl ManifestSchema1Signed {
    /// List digests of all layers referenced by this manifest.
    ///
    /// The returned layers list is ordered starting with the base image first.
    pub fn get_layers(&self) -> Vec<String> {
        self.fs_layers
            .iter()
            .rev()
            .map(|l| l.blob_sum.clone())
            .collect()
    }
}
