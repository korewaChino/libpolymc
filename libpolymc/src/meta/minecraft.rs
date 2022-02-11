use crate::meta::LibraryName;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Minecraft {
    #[serde(rename = "+traits")]
    pub traits: Vec<String>,

    pub asset_index: AssetIndex,
    pub libraries: Vec<Library>,
    pub mainClass: String,
    pub mainJar: Library,
    pub minecraftArguments: String,
    pub name: String,
    pub order: i64,
    pub releaseTime: String, // FIXME: time type
    pub requires: Vec<Requirement>,
    #[serde(rename = "type")]
    pub release_type: String, // TODO: enum
    pub uid: String,
    pub version: String, // FIXME: SemVer type
}

impl Minecraft {
    pub fn from_reader<R: std::io::Read>(reader: R) -> Result<Self> {
        Ok(serde_json::from_reader(reader)?)
    }

    pub fn parse_str(input: &str) -> Result<Self> {
        Ok(serde_json::from_str(input)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: i64,
    pub total_size: i64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub name: LibraryName,
    pub downloads: LibraryDownloads,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDownloads {
    pub artifact: LibraryDownload,
    #[serde(default)]
    pub classifiers: HashMap<String, LibraryDownload>,
    #[serde(default)]
    pub extract: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub natives: HashMap<String, String>,

    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDownload {
    pub sha1: String,
    pub size: i64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub action: String, // TODO: Enum type?

    #[serde(default)]
    pub os: Option<OS>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OS {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub equals: String,
    pub suggests: String,
    pub uid: String,
}
