/// Manifest version 2 schema 2
#[derive(Debug,Default,Deserialize,Serialize)]
pub struct ManifestSchema2 {
    #[serde(rename = "schemaVersion")]
    schema_version: u16,
    #[serde(rename = "mediaType")]
    media_type: String,
    config: Config,
    layers: Vec<Layer>,
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Config {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Layer {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
    urls: Option<Vec<String>>,
}

/// Manifest List
#[derive(Debug,Default,Deserialize,Serialize)]
pub struct ManifestList {
    #[serde(rename = "schemaVersion")]
    schema_version: u16,
    #[serde(rename = "mediaType")]
    media_type: String,
    manifests: Vec<ManifestObj>,
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct ManifestObj {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
    platform: Platform,
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Platform {
    architecture: String,
    os: String,
    #[serde(rename = "os.version")]
    os_version: Option<String>,
    #[serde(rename = "os.features")]
    os_features: Option<Vec<String>>,
    variant: Option<String>,
    features: Option<Vec<String>>,
}

impl ManifestSchema2 {
    pub fn get_layers(&self) -> Vec<String> {
        self.layers
            .iter()
            .map(|l| l.digest.clone())
            .collect()
    }
}
