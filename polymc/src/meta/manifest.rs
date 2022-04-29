use log::{debug, trace};
use ring::digest::{SHA1_OUTPUT_LEN, SHA256_OUTPUT_LEN};
use serde::{Deserialize, Serialize};

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::meta::AssetIndexInfo;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(rename = "+traits", default)]
    pub traits: Vec<String>,

    #[serde(default)]
    pub asset_index: Option<AssetIndexInfo>,
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
    #[serde(rename = "type", default)]
    pub release_type: String, // TODO: enum
    pub uid: String,
    pub version: String, // FIXME: SemVer type
}

impl Manifest {
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

        // I have spent an hour figuring out why I can't split strings, And finally found this like of code.

        // I did it. I finally did it. I have finally made this code cross-platform.
        // Holy shit. Kill me now.
        #[cfg(windows)]
        {
            ret.join(";")
        }
        #[cfg(not(windows))]
        {
            ret.join(":")
        }
    }

    pub fn assets_path_at(&self, at: &str) -> Option<String> {
        if self.asset_index.is_none() {
            return None;
        }

        let index = self.asset_index.as_ref().unwrap();
        let mut path = Path::new(at).to_path_buf();
        path.push("indexes");
        path.push(format!("{}.json", &index.id));
        Some(path.display().to_string())
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

        Ok(ret)
    }

    /// Verify all data.
    /// # Safety
    /// This uses write without synchronization, so only run one instance on a given dataset.
    pub unsafe fn verify_caching_at<S: AsRef<std::ffi::OsStr> + ?Sized>(
        &self,
        path: &S,
        platform: &OS,
    ) -> Result<Vec<(Library, Error)>> {
        let mut ret = Vec::new();

        for lib in &self.libraries {
            if !unsafe { *lib.verified.get() } && lib.required_for(platform) {
                if let Err(e) = lib.verify_at(path, platform) {
                    match e {
                        Error::LibraryMissing => ret.push((lib.clone(), e)),
                        Error::LibraryInvalidHash => ret.push((lib.clone(), e)),
                        _ => return Err(e),
                    }
                } else {
                    unsafe {
                        let verified = &mut *lib.verified.get();
                        *verified = true;
                    }
                }
            }
        }

        if let Some(jar) = &self.main_jar {
            if !unsafe { *jar.verified.get() } {
                if let Err(e) = jar.verify_at(path, platform) {
                    match e {
                        Error::LibraryMissing => ret.push((jar.clone(), e)),
                        Error::LibraryInvalidHash => ret.push((jar.clone(), e)),
                        _ => return Err(e),
                    }
                } else {
                    unsafe {
                        let verified = &mut *jar.verified.get();
                        *verified = true;
                    }
                }
            }
        }

        Ok(ret)
    }
}

crate::meta::index::from_str_json!(Manifest);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub name: LibraryName,
    pub downloads: LibraryDownloads,
    #[serde(default)]
    pub natives: HashMap<String, String>,

    #[serde(default)]
    pub extract: Option<ExtractOptions>,

    #[serde(default)]
    pub rules: Vec<Rule>,

    #[serde(skip)]
    verified: std::rc::Rc<UnsafeCell<bool>>,
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

        if digest.as_ref() == artifact.sha1.as_ref() {
            trace!("{} is valid", self.name);
            Ok(())
        } else {
            Err(Error::LibraryInvalidHash)
        }
    }

    pub fn required_for(&self, platform: &OS) -> bool {
        let mut allow = false;
        if self.rules.is_empty() {
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
    pub sha1: Sha1Sum,
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
    // TOOD: arch?
}

impl OS {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: None,
        }
    }

    // TODO: add discover function
    #[cfg(target_os = "macos")]
    pub fn get() -> Self {
        Self {
            name: "osx".to_string(),
            version: None, // TODO
        }
    }

    #[cfg(target_os = "linux")]
    pub fn get() -> Self {
        Self {
            name: "linux".to_string(),
            version: None, // TODO
        }
    }

    #[cfg(target_os = "windows")]
    pub fn get() -> Self {
        Self {
            name: "windows".to_string(),
            version: None, // TODO
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    #[serde(default)]
    pub equals: Option<String>,
    pub suggests: Option<String>,
    pub uid: String,
}

#[derive(Debug, Clone, serde_with::SerializeDisplay, serde_with::DeserializeFromStr)]
pub struct Sha1Sum([u8; ring::digest::SHA1_OUTPUT_LEN]);

impl std::fmt::Display for Sha1Sum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl FromStr for Sha1Sum {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = hex::decode(s)?;
        if s.len() != ring::digest::SHA1_OUTPUT_LEN {
            return Err(Error::LibraryInvalidHash);
        }

        let s: Option<[u8; ring::digest::SHA1_OUTPUT_LEN]> = s.try_into().ok();
        if let Some(s) = s {
            Ok(Self(s))
        } else {
            Err(Error::LibraryInvalidHash)
        }
    }
}

impl AsRef<[u8; ring::digest::SHA1_OUTPUT_LEN]> for Sha1Sum {
    fn as_ref(&self) -> &[u8; SHA1_OUTPUT_LEN] {
        &self.0
    }
}

#[derive(Debug, Clone, serde_with::SerializeDisplay, serde_with::DeserializeFromStr)]
pub struct Sha256Sum([u8; ring::digest::SHA256_OUTPUT_LEN]);

impl std::fmt::Display for Sha256Sum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl FromStr for Sha256Sum {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = hex::decode(s)?;
        if s.len() != ring::digest::SHA256_OUTPUT_LEN {
            return Err(Error::LibraryInvalidHash);
        }

        let s: Option<[u8; ring::digest::SHA256_OUTPUT_LEN]> = s.try_into().ok();
        if let Some(s) = s {
            Ok(Self(s))
        } else {
            Err(Error::LibraryInvalidHash)
        }
    }
}

impl AsRef<[u8; ring::digest::SHA256_OUTPUT_LEN]> for Sha256Sum {
    fn as_ref(&self) -> &[u8; SHA256_OUTPUT_LEN] {
        &self.0
    }
}

#[derive(Debug, Clone, serde_with::SerializeDisplay, serde_with::DeserializeFromStr)]
pub struct LibraryName {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub extra_versions: Vec<String>,
}

impl LibraryName {
    pub fn base_path_at<S: AsRef<std::ffi::OsStr> + ?Sized>(&self, path: &S) -> PathBuf {
        let mut path = Path::new(path).to_path_buf();
        self.namespace
            .split('.')
            .map(|v| path.push(v))
            .for_each(drop);

        path.push(&self.name);
        path.push(&self.version);

        path
    }

    pub fn path_at<S: AsRef<std::ffi::OsStr> + ?Sized>(&self, path: &S) -> PathBuf {
        let mut path = self.base_path_at(path);

        if !self.extra_versions.is_empty() {
            path.push(format!(
                "{}-{}-{}.jar",
                self.name,
                self.version,
                self.extra_versions.join("-")
            ));
        } else {
            path.push(format!("{}-{}.jar", self.name, self.version));
        }

        path
    }

    pub fn path_at_natives<S: AsRef<std::ffi::OsStr> + ?Sized>(
        &self,
        path: &S,
        natives: &str,
    ) -> PathBuf {
        let mut path = self.base_path_at(path);

        if !self.extra_versions.is_empty() {
            path.push(format!(
                "{}-{}-{}-{}.jar",
                self.name,
                self.version,
                self.extra_versions.join("-"),
                natives
            ));
        } else {
            path.push(format!("{}-{}-{}.jar", self.name, self.version, natives));
        }

        path
    }
}

//TODO: Just use LibraryName for this?
#[derive(Debug, Clone)]
pub struct Package{
    pub name: String,
    pub version: String
}

impl Package {
    fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string()
        }
    }
}

impl std::fmt::Display for LibraryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.extra_versions.is_empty() {
            write!(
                f,
                "{}:{}:{}:{}",
                self.namespace,
                self.name,
                self.version,
                self.extra_versions.join(":")
            )
        } else {
            write!(f, "{}:{}:{}", self.namespace, self.name, self.version)
        }
    }
}

impl std::str::FromStr for LibraryName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s: Vec<&str> = s.split(':').collect();
        if s.len() < 3 {
            return Err(Error::LibraryInvalidName);
        }

        let mut extra_versions = Vec::new();
        for s in &s[3..] {
            extra_versions.push(s.to_string());
        }

        Ok(Self {
            namespace: s[0].to_owned(),
            name: s[1].to_owned(),
            version: s[2].to_owned(),
            extra_versions,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct ExtractOptions {
    pub exclude: Vec<String>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn libraryname() {
        let name = "ca.weblite:java-objc-bridge:1.0.0";

        let name_parsed: LibraryName = name.parse().unwrap();
        assert_eq!(name_parsed.namespace, "ca.weblite");
        assert_eq!(name_parsed.name, "java-objc-bridge");
        assert_eq!(name_parsed.version, "1.0.0");

        assert_eq!(name_parsed.to_string(), name);

        assert_eq!(
            name_parsed.path_at(""),
            Path::new("ca/weblite/java-objc-bridge/1.0.0/java-objc-bridge-1.0.0.jar")
        );

        let name = "com.mojang:minecraft:1.18.1:client";
        let name_parsed: LibraryName = name.parse().unwrap();
        assert_eq!(name_parsed.namespace, "com.mojang");
        assert_eq!(name_parsed.name, "minecraft");
        assert_eq!(name_parsed.version, "1.18.1");
        assert_eq!(name_parsed.extra_versions, vec!["client"]);

        assert_eq!(name_parsed.to_string(), name);

        assert_eq!(
            name_parsed.path_at(""),
            Path::new("com/mojang/minecraft/1.18.1/minecraft-1.18.1-client.jar")
        )
    }
}
