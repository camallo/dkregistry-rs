/// Manifest version 2 schema 2.
#[derive(Debug,Default,Deserialize,Serialize)]
pub struct ManifestSchema2 {
    #[serde(rename = "schemaVersion")]
    schema_version: u16,
    #[serde(rename = "mediaType")]
    media_type: String,
    config: Config,
    layers: Vec<S2Layer>,
}

#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Config {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
}

#[derive(Debug,Default,Deserialize,Serialize)]
struct S2Layer {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
    urls: Option<Vec<String>>,
}

/// Manifest List.
#[derive(Debug,Default,Deserialize,Serialize)]
pub struct ManifestList {
    #[serde(rename = "schemaVersion")]
    schema_version: u16,
    #[serde(rename = "mediaType")]
    media_type: String,
    pub manifests: Vec<ManifestObj>,
}

/// Manifest object.
#[derive(Debug,Default,Deserialize,Serialize)]
pub struct ManifestObj {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    pub digest: String,
    pub platform: Platform,
}

/// Platform-related manifest entries.
#[derive(Debug,Default,Deserialize,Serialize)]
pub struct Platform {
    pub architecture: String,
    pub os: String,
    #[serde(rename = "os.version")]
    pub os_version: Option<String>,
    #[serde(rename = "os.features")]
    pub os_features: Option<Vec<String>>,
    pub variant: Option<String>,
    pub features: Option<Vec<String>>,
}

impl ManifestSchema2 {
    /// List digests of all layer referenced by this manifest.
    pub fn get_layers(&self) -> Vec<String> {
        self.layers
            .iter()
            .map(|l| l.digest.clone())
            .collect()
    }

    /// Get digest of the configuration object referenced by this manifest.
    pub fn config(&self) -> String {
        self.config.digest.clone()
    }
}
