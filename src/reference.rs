//! Parser for `docker://` URLs.
//!
//! This module provides support for parsing image references.
//!
//! ## Example
//!
//! ```rust
//! # extern crate dkregistry;
//! # fn main() {
//! # fn run() -> dkregistry::errors::Result<()> {
//! #
//! use std::str::FromStr;
//! use dkregistry::reference::Reference;
//!
//! // Parse an image reference
//! let dkref = Reference::from_str("docker://busybox")?;
//! assert_eq!(dkref.registry(), "registry-1.docker.io");
//! assert_eq!(dkref.repository(), "library/busybox");
//! assert_eq!(dkref.version(), "latest");
//! #
//! # Ok(())
//! # };
//! # run().unwrap();
//! # }
//! ```
//!
//!

// The `docker://` schema is not officially documented, but has a reference implementation:
// https://github.com/docker/distribution/blob/v2.6.1/reference/reference.go

use std::{fmt, str};
use std::str::FromStr;

/// Image version, either a tag or a digest.
#[derive(Clone)]
pub enum Version {
    Tag(String),
    Digest(String, String),
}

impl str::FromStr for Version {
    type Err = ::errors::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = match s.chars().nth(0) {
            Some(':') => Version::Tag(s.trim_left_matches(':').to_string()),
            Some('@') => {
                let r: Vec<&str> = s.trim_left_matches('@').splitn(2, ':').collect();
                if r.len() != 2 {
                    bail!("wrong digest format");
                };
                Version::Digest(r[0].to_string(), r[1].to_string())
            }
            Some(_) => bail!("unknown prefix"),
            None => bail!("too short"),
        };
        Ok(v)
    }
}

impl Default for Version {
    fn default() -> Self {
        Version::Tag("latest".to_string())
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let v = match self {
            &Version::Tag(ref s) => ":".to_string() + s,
            &Version::Digest(ref t, ref d) => "@".to_string() + t + ":" + d,
        };
        write!(f, "{}", v)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let v = match self {
            &Version::Tag(ref s) => s.to_string(),
            &Version::Digest(ref t, ref d) => t.to_string() + ":" + d,
        };
        write!(f, "{}", v)
    }
}

/// A registry image reference.
#[derive(Clone, Debug, Default)]
pub struct Reference {
    has_schema: bool,
    raw_input: String,
    registry: String,
    repository: String,
    version: Version,
}

impl Reference {
    pub fn new(registry: Option<String>, repository: String, version: Option<Version>) -> Self {
        let reg = registry.unwrap_or("registry-1.docker.io".to_string());
        let ver = version.unwrap_or(Version::Tag("latest".to_string()));
        Self {
            has_schema: false,
            raw_input: "".into(),
            registry: reg,
            repository: repository,
            version: ver,
        }
    }

    pub fn registry(&self) -> String {
        self.registry.clone()
    }

    pub fn repository(&self) -> String {
        self.repository.clone()
    }

    pub fn version(&self) -> String {
        self.version.to_string()
    }

    pub fn to_raw_string(&self) -> String {
        self.raw_input.clone()
    }

    //TODO(lucab): move this to a real URL type
    pub fn to_url(&self) -> String {
        format!(
            "docker://{}/{}{:?}",
            self.registry,
            self.repository,
            self.version
        )
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}/{}{:?}", self.registry, self.repository, self.version)
    }
}

impl str::FromStr for Reference {
    type Err = ::errors::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_url(s)
    }
}

fn parse_url(s: &str) -> Result<Reference, ::errors::Error> {
    // TODO(lucab): move to nom
    let mut rest = s;
    let has_schema = rest.starts_with("docker://");
    if has_schema {
        rest = s.trim_left_matches("docker://");
    };
    let (rest, ver) = match (rest.rfind('@'), rest.rfind(':')) {
        (Some(i), _) | (None, Some(i)) => {
            let s = rest.split_at(i);
            (s.0, Version::from_str(s.1)?)
        }
        (None, None) => (rest, Version::default()),
    };
    if rest.len() < 1 {
        bail!("name too short");
    }
    let mut reg = "registry-1.docker.io";
    let split: Vec<&str> = rest.rsplitn(3, '/').collect();
    let image = match split.len() {
        1 => "library/".to_string() + rest,
        2 => rest.to_string(),
        _ => {
            reg = split[2];
            split[1].to_string() + "/" + split[0]
        }
    };
    if image.len() > 127 {
        bail!("name too long");
    }
    Ok(Reference {
        has_schema: has_schema,
        raw_input: s.to_string(),
        registry: reg.to_string(),
        repository: image,
        version: ver,
    })
}
