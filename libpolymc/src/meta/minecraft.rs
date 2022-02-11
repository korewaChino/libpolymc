use crate::meta::LibraryName;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(rename = "+traits", default)]
    pub traits: Vec<String>,

    #[serde(default)]
    pub asset_index: Option<AssetIndex>,
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub main_jar: Option<Library>,
    pub minecraft_arguments: Option<String>,
    pub name: String,
    pub order: i64,
    pub release_time: String, // FIXME: time type
    #[serde(default)]
    pub requires: Vec<Requirement>,
    #[serde(rename = "type")]
    pub release_type: String, // TODO: enum
    pub uid: String,
    pub version: String, // FIXME: SemVer type
}

impl Manifest {
    pub fn from_reader<R: std::io::Read>(reader: R) -> Result<Self> {
        Ok(serde_json::from_reader(reader)?)
    }

    pub fn parse_str(input: &str) -> Result<Self> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn build_class_path_at<S: AsRef<std::ffi::OsStr> + ?Sized>(
        &self,
        path: &S,
        platform: &OS,
    ) -> String {
        let mut ret = Vec::new();

        for lib in &self.libraries {
            if lib.required_for(platform) {
                ret.push(lib.name.path_at(path).display().to_string());
            }
        }

        if let Some(jar) = &self.main_jar {
            ret.push(jar.name.path_at(path).display().to_string())
        }

        ret.join(":")
    }

    pub fn verify_at<S: AsRef<std::ffi::OsStr> + ?Sized>(
        &self,
        path: &S,
        platform: &OS,
    ) -> Result<Vec<(Library, Error)>> {
        let mut ret = Vec::new();

        for lib in &self.libraries {
            if lib.required_for(platform) {
                if let Err(e) = lib.verify_at(path, platform) {
                    match e {
                        Error::LibraryMissing => ret.push((lib.clone(), e)),
                        Error::LibraryInvalidHash => ret.push((lib.clone(), e)),
                        _ => return Err(e),
                    }
                }
            }
        }

        if let Some(jar) = &self.main_jar {
            if let Err(e) = jar.verify_at(path, platform) {
                match e {
                    Error::LibraryMissing => ret.push((jar.clone(), e)),
                    Error::LibraryInvalidHash => ret.push((jar.clone(), e)),
                    _ => return Err(e),
                }
            }
        }

        return Ok(ret);
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
    #[serde(default)]
    pub natives: HashMap<String, String>,

    #[serde(default)]
    pub extract: HashMap<String, Vec<String>>,

    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl Library {
    pub fn verify_at<S: AsRef<std::ffi::OsStr> + ?Sized>(
        &self,
        at: &S,
        platform: &OS,
    ) -> Result<()> {
        debug!("verifying {}", self.name);
        let artifact = self
            .select_for(platform)
            .ok_or(Error::LibraryNotSupported)?;
        let path = self.path_at_for(at, platform);

        trace!("verifying {}", path.display());
        if !path.is_file() {
            return Err(Error::LibraryMissing);
        }

        let mut file = OpenOptions::new().read(true).open(path)?;

        let mut digest = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);

        loop {
            let mut buf = [0u8; 8192];
            let read = file.read(&mut buf)?;
            digest.update(&buf[..read]);
            if read < buf.len() {
                break;
            }
        }

        let digest = digest.finish();

        let hash = &self
            .select_for(platform)
            .ok_or(Error::LibraryNotSupported)?;
        let hash = hex::decode(&hash.sha1)?;

        if digest.as_ref() == &hash {
            trace!("{} is valid", self.name);
            return Ok(());
        } else {
            Err(Error::LibraryInvalidHash)
        }
    }

    pub fn required_for(&self, platform: &OS) -> bool {
        let mut allow = false;
        if self.rules.len() == 0 {
            allow = true;
        } else {
            for r in &self.rules {
                if r.action == RuleAction::Allow && !allow {
                    allow = r.os.name == platform.name;
                }
            }
        }

        allow
    }

    pub fn select_for(&self, os: &OS) -> Option<&LibraryDownload> {
        if let Some(name) = self.natives.get(&os.name) {
            self.downloads.classifiers.get(name)
        } else {
            Some(&self.downloads.artifact)
        }
    }

    pub fn path_at<S: AsRef<std::ffi::OsStr> + ?Sized>(&self, at: &S) -> PathBuf {
        self.name.path_at(at)
    }

    pub fn path_at_for<S: AsRef<std::ffi::OsStr> + ?Sized>(
        &self,
        at: &S,
        platform: &OS,
    ) -> PathBuf {
        if let Some(name) = self.natives.get(&platform.name) {
            self.name.path_at_natives(at, name)
        } else {
            self.name.path_at(at)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDownloads {
    pub artifact: LibraryDownload,
    #[serde(default)]
    pub classifiers: HashMap<String, LibraryDownload>,
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
    pub action: RuleAction,

    pub os: OS,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OS {
    pub name: String,
    pub version: Option<String>,
}

impl OS {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: None,
        }
    }

    // TODO: add discover function
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub equals: String,
    pub suggests: String,
    pub uid: String,
}
