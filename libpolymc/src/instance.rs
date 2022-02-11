use crate::auth::Auth;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Instance {
    /// Name of the Minecraft instance given by the user.
    pub name: String,
    /// The version string of the instance.
    pub version: String,
    /// Path to the minecraft.jar
    pub minecraft_path: PathBuf,
    /// Path to Minecraft's assets
    pub assets_path: Option<PathBuf>,
    /// Path to Minecraft's Java libraries
    pub libraries_path: Option<PathBuf>,
    /// Java options to pass to Minecraft.
    pub java_opts: Vec<String>,
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
            java_opts: Vec::new(),
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
    /// This will default onto the default bin/version folder inside the minecraft path.
    pub fn get_libraries_path(&self) -> PathBuf {
        if let Some(path) = &self.libraries_path {
            path.to_owned()
        } else {
            let mut path = self.minecraft_path.clone();
            path.push("bin");
            path.push(&self.version);
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
