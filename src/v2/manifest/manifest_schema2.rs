use crate::errors::{Error, Result};
use reqwest::Method;
use serde::ser::SerializeMap;
use std::collections::{HashMap, HashSet};

/// Manifest version 2 schema 2.
///
/// Specification is at https://docs.docker.com/registry/spec/manifest-v2-2/.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ManifestSchema2Spec {
    #[serde(rename = "schemaVersion")]
    schema_version: u16,
    #[serde(rename = "mediaType")]
    media_type: String,
    config: Config,
    layers: Vec<S2Layer>,
}

/// Super-type for combining a ManifestSchema2 with a ConfigBlob.
#[derive(Debug, Default)]
pub struct ManifestSchema2 {
    pub manifest_spec: ManifestSchema2Spec,
    pub config_blob: ConfigBlob,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}

/// Partial representation of a container image (application/vnd.docker.container.image.v1+json).
///
/// The remaining fields according to [the image spec v1][image-spec-v1] are not covered.
///
/// [image-spec-v1]: https://github.com/moby/moby/blob/a30990b3c8d0d42280fa501287859e1d2393a951/image/spec/v1.md#image-json-description
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConfigBlob {
    /// CPU Architecture supported by this image.
    /// Common values: "amd64", "arm", "386".
    pub architecture: String,
    /// Operating system, supported as a host.
    /// Common values: "linux", "freebsd", "darwin".
    pub os: String,
    /// Runtime configuration
    #[serde(rename = "config", default, skip_serializing_if = "Option::is_none")]
    pub runtime_config: Option<RuntimeConfig>,
}

/// RunConfig, as defined by [OCI image specification][image-spec-v1].
/// See specification for detailed explanation.
///
/// [image-spec-v1]: https://github.com/moby/moby/blob/a30990b3c8d0d42280fa501287859e1d2393a951/image/spec/v1.md#container-runconfig-field-descriptions
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RuntimeConfig {
    /// Defines user and optionally group to use when running this image.
    #[serde(rename = "User", default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Default memory limit in bytes.
    #[serde(rename = "Memory", default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<u64>,
    /// Default memory+swap limit in bytes; value of -1 disables swap.
    #[serde(
        rename = "MemorySwap",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub memory_swap: Option<i64>,
    /// Default CPU shares.
    #[serde(rename = "CpuShares", default, skip_serializing_if = "Option::is_none")]
    pub cpu_shares: Option<u32>,
    /// Ports that should be exposed
    #[serde(
        rename = "ExposedPorts",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exposed_ports: Option<OciHashSet<String>>,
    /// Environment variables that should be set
    #[serde(rename = "Env", default, skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
    /// Environment variables that should be set
    #[serde(
        rename = "Entrypoint",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub entrypoint: Option<Vec<String>>,
    /// Environment variables that should be set
    #[serde(rename = "Cmd", default, skip_serializing_if = "Option::is_none")]
    pub cmd: Option<Vec<String>>,
    /// Volumes that should be exposed
    #[serde(rename = "Volumes", default, skip_serializing_if = "Option::is_none")]
    pub volumes: Option<OciHashSet<std::path::PathBuf>>,
    /// Default working directory
    #[serde(
        rename = "WorkingDir",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub working_dir: Option<std::path::PathBuf>,
    /// User-defined metadata
    #[serde(rename = "Labels")]
    pub labels: Option<HashMap<String, String>>,
}

/// OCI specification uses strange JSON encoding for sets.
/// This struct wraps HashSet, implementing this encoding.
#[derive(Debug, Default)]
pub struct OciHashSet<T>(pub HashSet<T>);

impl<T: serde::Serialize> serde::Serialize for OciHashSet<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map_serializer = serializer.serialize_map(Some(self.0.len()))?;
        for item in &self.0 {
            map_serializer.serialize_entry(item, &EmptyStruct {})?;
        }
        map_serializer.end()
    }
}

impl<'de, T: serde::Deserialize<'de> + Eq + std::hash::Hash> serde::Deserialize<'de>
    for OciHashSet<T>
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let map = std::collections::HashMap::<T, EmptyStruct>::deserialize(deserializer)?;
        Ok(OciHashSet(map.into_iter().map(|(k, _v)| k).collect()))
    }
}

// will be encoded as `{}` in JSON, while () would encode as `null`
#[derive(Serialize, Deserialize)]
struct EmptyStruct {}

#[derive(Debug, Default, Deserialize, Serialize)]
struct S2Layer {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    digest: String,
    urls: Option<Vec<String>>,
}

/// Manifest List.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ManifestList {
    #[serde(rename = "schemaVersion")]
    schema_version: u16,
    #[serde(rename = "mediaType")]
    media_type: String,
    pub manifests: Vec<ManifestObj>,
}

/// Manifest object.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ManifestObj {
    #[serde(rename = "mediaType")]
    media_type: String,
    size: u64,
    pub digest: String,
    pub platform: Platform,
}

/// Platform-related manifest entries.
#[derive(Debug, Default, Deserialize, Serialize)]
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

impl ManifestSchema2Spec {
    /// Get `Config` object referenced by this manifest.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Fetch the config blob for this manifest
    pub(crate) async fn fetch_config_blob(
        self,
        client: crate::v2::Client,
        repo: String,
    ) -> Result<ManifestSchema2> {
        let url = {
            let ep = format!(
                "{}/v2/{}/blobs/{}",
                client.base_url.clone(),
                repo,
                self.config.digest
            );
            reqwest::Url::parse(&ep)?
        };

        let r = client
            .build_reqwest(Method::GET, url.clone())
            .send()
            .await?;

        let status = r.status();
        trace!("GET {:?}: {}", url, &status);

        if !status.is_success() {
            return Err(Error::UnexpectedHttpStatus(status));
        }

        let config_blob = r.json::<ConfigBlob>().await?;

        Ok(ManifestSchema2 {
            manifest_spec: self,
            config_blob,
        })
    }
}

impl ManifestSchema2 {
    /// List digests of all layers referenced by this manifest.
    ///
    /// The returned layers list is ordered starting with the base image first.
    pub fn get_layers(&self) -> Vec<String> {
        self.manifest_spec
            .layers
            .iter()
            .map(|l| l.digest.clone())
            .collect()
    }

    /// Get the architecture from the config
    pub fn architecture(&self) -> String {
        self.config_blob.architecture.to_owned()
    }
}
