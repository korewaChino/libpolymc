use crate::meta::manifest::{Library, Manifest, OS};
use crate::meta::SearchResult;
use crate::{Error, Result};
use log::trace;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct InstanceGameConfig {
    pub min: String, // TODO: create enum type?
    pub max: String,

    pub width: u32,
    pub height: u32,
}

impl Default for InstanceGameConfig {
    fn default() -> Self {
        Self {
            min: "512M".to_owned(),
            max: "1024M".to_owned(),
            width: 854,
            height: 480,
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Instance {
    /// Name of the Minecraft instance given by the user.
    pub name: String,
    /// The version string of the instance.
    pub version: String,
    /// Path to the base directory.
    pub minecraft_path: String,
    /// Path to Minecraft's assets.
    pub assets_path: Option<String>,
    /// Path to Minecraft's Java libraries.
    pub libraries_path: Option<String>,
    /// Path to Minecraft's native libraries.
    pub natives_path: Option<String>,
    /// Java options to pass to the JVM.
    pub java_opts: Vec<String>,
    /// Extra arguments to pass to Minecraft.
    pub extra_args: Vec<String>,

    pub config: InstanceGameConfig,

    pub uid: String,
    pub manifests: HashMap<String, Manifest>,
}

impl Instance {
    pub fn new(
        name: &str,
        version: &str,
        minecraft_path: &str,
        search_result: SearchResult,
    ) -> Self {
        Self {
            name: name.to_owned(),
            version: version.to_owned(),
            minecraft_path: minecraft_path.to_string(),
            assets_path: None,
            libraries_path: None,
            natives_path: None,
            java_opts: Vec::new(),
            extra_args: Vec::new(),
            config: Default::default(),

            uid: search_result.uid,
            manifests: search_result.manifests,
        }
    }

    /// Set the assets path.
    pub fn set_assets_path(&mut self, path: &str) {
        self.assets_path = Some(path.to_string());
    }

    /// Get the current asset path.
    /// This will default onto the assets folder inside the minecraft path.
    pub fn get_assets_path(&self) -> String {
        if let Some(path) = &self.assets_path {
            path.to_string()
        } else {
            let mut path = Path::new(&self.minecraft_path).to_path_buf();
            path.push("assets");
            path.to_str().unwrap().to_string()
        }
    }

    /// Set the libraries path.
    pub fn set_libraries_path(&mut self, path: &str) {
        // Convert to an absolute path
        self.libraries_path = Some(path.to_string())
    }

    /// Get the current libraries path.
    /// This will default onto the default .minecraft/libraries path.
    pub fn get_libraries_path(&self) -> String {
        if let Some(path) = &self.libraries_path {
            path.to_string()
        } else {
            let mut path = Path::new(&self.minecraft_path).to_path_buf();
            path.push("libraries");
            path.to_str().unwrap().to_string()
        }
    }

    pub fn set_extra_args(&mut self, args: Vec<String>) {
        self.extra_args = args.to_vec();
    }

    /// Set the natives path.
    pub fn set_natives_path(&mut self, path: &str) {
        self.natives_path = Some(path.to_string())
    }

    /// Get the current natives path.
    /// This will default onto the default .minecraft/natives path.
    pub fn get_natives_path(&self) -> String {
        if let Some(path) = &self.natives_path {
            path.to_string()
        } else {
            let mut path = Path::new(&self.minecraft_path).to_path_buf();
            path.push("natives");
            path.to_str().unwrap().to_string()
        }
    }

    pub fn get_natives(&self, platform: &OS) -> Vec<&Library> {
        let mut ret = Vec::new();
        for (_k, v) in &self.manifests {
            for lib in &v.libraries {
                if lib.natives.get(&platform.name).is_some() {
                    ret.push(lib);
                }
            }
        }
        ret
    }

    /// Extract natives into the natives path
    pub fn build_natives(&self) -> Result<String> {
        let path = self.get_natives_path();

        std::fs::create_dir_all(&path)?;
        let os = OS::get();

        let libs = self.get_natives(&os);
        for lib in libs {
            let jar = lib.path_at_for(&self.get_libraries_path(), &os);
            trace!("extracting natives {} to: {}", jar.display(), path);

            let file = OpenOptions::new().read(true).open(jar)?;
            let mut archive = zip::ZipArchive::new(file)?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let mut outpath = Path::new(&path).to_path_buf();
                match file.enclosed_name() {
                    Some(path) => {
                        if let Some(extract) = &lib.extract {
                            for x in &extract.exclude {
                                if path == Path::new(x) {
                                    trace!("Skipping: {}", x);
                                    continue;
                                }
                            }
                        }
                        outpath.push(path)
                    }
                    None => continue,
                }

                if (*file.name()).ends_with('/') {
                    std::fs::create_dir_all(&outpath)?;
                } else {
                    trace!("extracting file: {}", file.name());
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            std::fs::create_dir_all(p)?;
                        }
                    }

                    let mut outfile = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .append(false)
                        .open(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = file.unix_mode() {
                        fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                    }
                }
            }
        }

        Ok(path)
    }

    /// Get the current minecraft.jar path.
    /// This will default onto the default versions/<version>/<version>.jar path.
    pub fn get_jar_path(&self) -> Result<String> {
        let manifest = self.manifests.get(&self.uid).ok_or(Error::MetaNotFound)?;
        let os = OS::get();
        Ok(manifest
            .main_jar
            .as_ref()
            .ok_or(Error::MetaNotFound)?
            .path_at_for(&self.get_libraries_path(), &os)
            .display()
            .to_string())
    }

    pub fn get_class_paths(&self) -> String {
        let mut ret = Vec::new();
        for (_k, v) in &self.manifests {
            ret.push(v.build_class_path_at(&self.get_libraries_path(), &OS::get()));
        }
        // The following lines is breaking all Windows builds.
        // WHY IN THE FUCK ARE YOU STILL SEPERATED BY A COLON? I SWEAR TO GOD I TOLD YOU TO USE SEMICOLONS!!!!!!!!!!!!!!
        // RUST WHAT THE FUCK IS WRONG WITH YOU
        // NOTE: Check polymc::meta::manifest::Manifest::build_class_path_at. Skill issue.
        // Check if windows
        #[cfg(windows)]
        {
            ret.join(";")
        }
        #[cfg(not(windows))]
        {
            ret.join(":")
        }
    }

    pub fn get_manifest_extra_jvm_args(&self, platform: &OS) -> Vec<String> {
        let mut ret = Vec::new();

        for (_k, v) in &self.manifests {
            for v in &v.traits {
                if let Some(v) = Self::parse_trait(v, platform) {
                    ret.push(v)
                }
            }
        }

        ret
    }

    fn parse_trait(jvm_trait: &str, platform: &OS) -> Option<String> {
        Some(match jvm_trait {
            "FirstThreadOnMacOS" if platform.name == "osx" => "-XstartOnFirstThread".to_string(),
            _ => {
                log::info!("unknown jvm trait: '{jvm_trait}'");
                return None;
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::meta::DownloadRequest;

    use super::*;
    use std::path::Path;
    /*
    these tests are broken because we also need to make a fake downloader and idk how to do that
    #[test]
    fn get_path() {
        let instance = Instance::new("test", "0.0.0", "/not/existing", SearchResult::new(DownloadRequest::new("test", "0.0.0", "test", "test", "test")));

        assert_eq!(
            instance.get_assets_path(),
            Path::new("/not/existing/assets")
        );
        assert_eq!(
            instance.get_libraries_path(),
            Path::new("/not/existing/libraries")
        );

        let mut instance = instance;
        instance.set_assets_path("/assets/path");
        assert_eq!(instance.get_assets_path(), Path::new("/assets/path"));
        assert_eq!(
            instance.get_libraries_path(),
            Path::new("/not/existing/libraries/")
        );

        instance.set_libraries_path("/libraries/path");
        assert_eq!(instance.get_assets_path(), Path::new("/assets/path"));
        assert_eq!(instance.get_libraries_path(), Path::new("/libraries/path"));
    }*/
}
