//! Parser for `docker://` URLs.

// The `docker://` schema is not officially documented, but has a reference implementation:
// https://github.com/docker/distribution/blob/v2.6.1/reference/reference.go

use std::{fmt, str};
use std::str::FromStr;

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

#[derive(Clone, Debug, Default)]
pub struct Reference {
    has_schema: bool,
    raw_input: String,
    registry: String,
    repository: String,
    image: String,
    version: Version,
}

impl Reference {
    pub fn new(registry: Option<String>,
               repository: Option<String>,
               image: String,
               version: Option<Version>)
               -> Self {
        let reg = registry.unwrap_or("registry-1.docker.io".to_string());
        let repo = repository.unwrap_or("library".to_string());
        let ver = version.unwrap_or(Version::Tag("latest".to_string()));
        Self {
            has_schema: false,
            raw_input: "".into(),
            registry: reg,
            repository: repo,
            image: image,
            version: ver,
        }
    }

    pub fn registry(&self) -> String {
        self.registry.clone()
    }

    pub fn image(&self) -> String {
        self.repository.clone() + "/" + self.image.as_str()
    }

    pub fn version(&self) -> String {
        self.version.to_string()
    }

    pub fn to_raw_string(&self) -> String {
        self.raw_input.clone()
    }

    //TODO(lucab): move this to a real URL type
    pub fn to_url(&self) -> String {
        format!("docker://{}/{}/{}{:?}",
                self.registry,
                self.repository,
                self.image,
                self.version)
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f,
               "{}/{}/{}{:?}",
               self.registry,
               self.repository,
               self.image,
               self.version)
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
    let mut reg = "registry-1.docker.io";
    let mut repo = "library";
    let rest: Vec<&str> = rest.rsplitn(3, '/').collect();
    let image = match rest.len() {
        1 => rest[0],
        2 => {
            repo = rest[1];
            rest[0]
        }
        _ => {
            reg = rest[2];
            repo = rest[1];
            rest[0]
        }
    };
    if image.len() < 1 {
        bail!("name too short");
    }
    if image.len() > 127 {
        bail!("name too long");
    }
    Ok(Reference {
           has_schema: has_schema,
           raw_input: s.to_string(),
           registry: reg.to_string(),
           repository: repo.to_string(),
           image: image.to_string(),
           version: ver,
       })
}
