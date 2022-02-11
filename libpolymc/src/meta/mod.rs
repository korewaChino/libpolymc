use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::Error;

pub mod minecraft;

#[derive(Debug, Clone, SerializeDisplay, DeserializeFromStr)]
pub struct LibraryName {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub extra_versions: Vec<String>,
}

impl LibraryName {
    pub fn at_path<S: AsRef<std::ffi::OsStr> + ?Sized>(&self, path: &S) -> PathBuf {
        let mut path = Path::new(path).to_path_buf();

        self.namespace
            .split('.')
            .map(|v| path.push(v))
            .collect::<()>();

        path.push(&self.name);
        path.push(&self.version);

        if self.extra_versions.len() != 0 {
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
}

impl Display for LibraryName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.extra_versions.len() != 0 {
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

impl FromStr for LibraryName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s: Vec<&str> = s.split(':').collect();
        if s.len() < 3 {
            return Err(Error::InvalidLibraryName);
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
            name_parsed.at_path(""),
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
            name_parsed.at_path(""),
            Path::new("com/mojang/minecraft/1.18.1/minecraft-1.18.1-client.jar")
        )
    }
}
