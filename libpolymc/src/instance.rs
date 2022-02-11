use crate::auth::Auth;
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
    /// Path to the base directory
    pub minecraft_path: PathBuf,
    /// Path to Minecraft's assets
    pub assets_path: Option<PathBuf>,
    /// Path to Minecraft's Java libraries
    pub libraries_path: Option<PathBuf>,
    /// Path to the minecraft.jar
    pub jar_path: Option<PathBuf>,
    /// Java options to pass to Minecraft.
    pub java_opts: Vec<String>,

    pub config: InstanceGameConfig,
}

impl Instance {
    pub fn new<S: AsRef<std::ffi::OsStr> + ?Sized>(
        name: &str,
        version: &str,
        minecraft_path: &S,
    ) -> Self {
        Self {
            name: name.to_owned(),
            version: version.to_owned(),
            minecraft_path: Path::new(minecraft_path).to_path_buf(),
            assets_path: None,
            libraries_path: None,
            jar_path: None,
            java_opts: Vec::new(),
            config: Default::default(),
        }
    }

    /// Set the assets path.
    pub fn set_assets_path<S: AsRef<std::ffi::OsStr> + ?Sized>(&mut self, path: &S) {
        self.assets_path = Some(Path::new(path).to_path_buf());
    }

    /// Get the current asset path.
    /// This will default onto the assets folder inside the minecraft path.
    pub fn get_assets_path(&self) -> PathBuf {
        if let Some(path) = &self.assets_path {
            path.to_owned()
        } else {
            let mut path = self.minecraft_path.clone();
            path.push("assets").to_owned();
            path
        }
    }

    /// Set the libraries path.
    pub fn set_libraries_path<S: AsRef<std::ffi::OsStr> + ?Sized>(&mut self, path: &S) {
        self.libraries_path = Some(Path::new(path).to_path_buf());
    }



    /// Get the current libraries path.
    /// This will default onto the default .minecraft/libraries path.
    pub fn get_libraries_path(&self) -> PathBuf {
        if let Some(path) = &self.libraries_path {
            path.to_owned()
        } else {
            let mut path = self.minecraft_path.clone();
            path.push("libraries");
            path
        }
    }

    /// Get the current minecraft.jar path.
    /// This will default onto the default versions/<version>/<version>.jar path.
    pub fn get_jar_path(&self) -> PathBuf {
        if let Some(path) = &self.jar_path {
            path.to_owned()
        } else {
            let mut path = self.minecraft_path.clone();
            path.push("versions");
            path.push(&self.version);
            path.push(format!("{}.jar", self.version));
            path
        }
    }
}



#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    #[test]
    fn get_assets_path() {
        let instance = Instance::new("test", "0.0.0", "/not/existing");

        assert_eq!(
            instance.get_assets_path(),
            Path::new("/not/existing/assets")
        );
        assert_eq!(
            instance.get_libraries_path(),
            Path::new("/not/existing/bin/0.0.0")
        );

        let mut instance = instance;
        instance.set_assets_path("/assets/path");
        assert_eq!(instance.get_assets_path(), Path::new("/assets/path"));
        assert_eq!(
            instance.get_libraries_path(),
            Path::new("/not/existing/bin/0.0.0")
        );

        instance.set_libraries_path("/libraries/path");
        assert_eq!(instance.get_assets_path(), Path::new("/assets/path"));
        assert_eq!(instance.get_libraries_path(), Path::new("/libraries/path"));
    }
}
