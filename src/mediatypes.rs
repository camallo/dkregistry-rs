//! Media-types for API objects.

// schema1 types, see https://docs.docker.com/registry/spec/manifest-v2-1/

/// Manifest, version 2 schema 1.
pub static MANIFEST_V2S1: &'static str = "application/vnd.docker.distribution.manifest.v1+json";
/// Signed manifest, version 2 schema 1.
pub static MANIFEST_V2S1_SIGNED: &'static str = "application/vnd.docker.distribution.manifest.v1+prettyjws";

// schema2 types, see https://docs.docker.com/registry/spec/manifest-v2-2/

/// Manifest, version 2 schema 1.
pub static MANIFEST_V2S2: &'static str = "application/vnd.docker.distribution.manifest.v2+json";
/// Manifest List (aka "fat manifest").
pub static MANIFEST_LIST: &'static str = "application/vnd.docker.distribution.manifest.list.v2+json";
/// Image layer, as a gzip-compressed tar.
pub static IMAGE_LAYER: &'static str = "application/vnd.docker.image.rootfs.diff.tar.gzip";
/// Configuration object for a container.
pub static CONTAINER_CONFIG_V1: &'static str = "application/vnd.docker.container.image.v1+json";
