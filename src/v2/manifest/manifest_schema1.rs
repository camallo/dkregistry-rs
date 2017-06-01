use v2::*;

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

#[derive(Debug,Deserialize,Serialize)]
pub struct Layer {
    #[serde(rename = "blobSum")]
    blob_sum: String,
}

impl ManifestSchema1Signed {
    pub fn get_layers(&self) -> Vec<String> {
        self.fs_layers
            .iter()
            .rev()
            .map(|l| l.blob_sum.clone())
            .collect()
    }
}
