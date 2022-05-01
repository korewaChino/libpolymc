use serde::{Deserialize, Serialize};

use crate::meta::manifest::{Manifest, Requirement, Sha256Sum};
use crate::{Error, Result};

macro_rules! from_str_json {
    ($type:ident) => {
        impl std::str::FromStr for $type {
            type Err = $crate::Error;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                Ok(serde_json::from_str(s)?)
            }
        }

        impl $type {
            /// Parse data from reader into [`Self`].
            pub fn from_reader<R: std::io::Read>(reader: &mut R) -> $crate::Result<Self> {
                Ok(serde_json::from_reader(reader)?)
            }

            /// Parse slice of data into [`Self`].
            pub fn from_data(data: &[u8]) -> $crate::Result<Self> {
                Ok(serde_json::from_slice(data)?)
            }
        }
    };
}
pub(crate) use from_str_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaIndex {
    pub format_version: u64,
    pub packages: Vec<MetaIndexPackage>,
}

impl MetaIndex {
    pub fn get_uid_mut(&mut self, uid: &str) -> Result<&mut MetaIndexPackage> {
        for package in &mut self.packages {
            if package.uid == uid {
                return Ok(package);
            }
        }

        Err(Error::MetaNotFound)
    }

    pub fn get_uid(&self, uid: &str) -> Result<&MetaIndexPackage> {
        for package in &self.packages {
            if package.uid == uid {
                return Ok(package);
            }
        }

        Err(Error::MetaNotFound)
    }
}

from_str_json!(MetaIndex);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaIndexPackage {
    pub name: String,
    pub sha256: Sha256Sum,
    pub uid: String,

    /// Resolved package index
    #[serde(skip)]
    pub index: Option<PackageIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageIndex {
    pub format_version: u64,
    pub name: String,
    pub uid: String,
    pub versions: Vec<PackageVersion>,
}

impl PackageIndex {
    pub fn find_version_mut(&mut self, version: &str) -> Result<&mut PackageVersion> {
        for package in &mut self.versions {
            if package.version == version {
                return Ok(package);
            }
        }

        Err(Error::MetaNotFound)
    }

    pub fn find_version(&self, version: &str) -> Result<&PackageVersion> {
        for package in &self.versions {
            if package.version == version {
                return Ok(package);
            }
        }

        Err(Error::MetaNotFound)
    }
}

from_str_json!(PackageIndex);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageVersion {
    pub release_time: String, // TODO: proper type
    #[serde(default)]
    pub requires: Vec<Requirement>,
    pub sha256: Sha256Sum,
    #[serde(rename = "type", default)]
    pub release_type: Option<String>, // TODO: enum type?
    pub version: String,

    /// Resolved package manifest
    #[serde(skip)]
    pub manifest: Option<Manifest>,
}
