//use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

#[cfg(target_family = "unix")]
use std::os::unix::io::{AsRawFd, RawFd};

use anyhow::{Ok, Result};
use log::*;

use crate::auth::Auth;
use crate::instance::Instance;
use crate::meta::manifest::OS;
use crate::{Error};

#[derive(Debug)]
#[repr(C)]
pub struct RunningInstance<'a> {
    pub process: Child,
    pub instance: &'a Instance,
}

impl<'a> RunningInstance<'a> {
    /// Return raw fd of stdin of the java process.
    ///
    /// # Safety
    /// The returned fd has to be closed after use.
    #[cfg(target_family = "unix")]
    #[no_mangle]
    pub unsafe extern "C" fn running_instance_get_stdin_fd(&self) -> RawFd {
        self.process
            .stdin
            .as_ref()
            .map(|fd| fd.as_raw_fd())
            .unwrap_or(-libc::ENOENT)
    }

    /// Return raw fd of stdout of the java process.
    ///
    /// # Safety
    /// The returned fd has to be closed after use.
    #[cfg(target_family = "unix")]
    #[no_mangle]
    pub unsafe extern "C" fn running_instance_get_stdout_fd(&self) -> RawFd {
        self.process
            .stdout
            .as_ref()
            .map(|fd| fd.as_raw_fd())
            .unwrap_or(-libc::ENOENT)
    }

    /// Return raw fd of stderr of the java process.
    ///
    /// # Safety
    /// The returned fd has to be closed after use.
    #[cfg(target_family = "unix")]
    #[no_mangle]
    pub unsafe extern "C" fn running_instance_get_stderr_fd(&self) -> RawFd {
        self.process
            .stderr
            .as_ref()
            .map(|fd| fd.as_raw_fd())
            .unwrap_or(-libc::ENOENT)
    }

    /*#[no_mangle]
    pub unsafe extern "C" fn running_instance_kill(mut self) -> c_int {
        if let Err(e) = self.process.kill() {
            -e.raw_os_error().unwrap_or(libc::ENOTRECOVERABLE)
        } else {
            0
        }
    }
    */
}

pub struct Java {
    java: PathBuf,
}

impl Java {
    pub fn new<S: AsRef<std::ffi::OsStr> + ?Sized>(java: &S) -> Self {
        Self {
            java: Path::new(java).to_path_buf(),
        }
    }

    pub fn start<'a>(&self, instance: &'a Instance, auth: Auth) -> Result<RunningInstance<'a>> {
        // TODO: check java version before starting minecraft
        // TODO: propagate OS from here into every leaf functions
        let platform = OS::get();
        // Check if Windows
        /* if platform.name == "windows" {
            // make format as follows:
            // & "{}"
            let mut command = Command::new("&");
            command.args(&self.java);
        } */

        // Set classpath
        std::env::set_var("CLASSPATH", &instance.get_class_paths());
        // Generate Minecraft folder
        std::fs::create_dir_all(&instance.minecraft_path)?;
        let mut command = Command::new(&self.java);
        command
            .args(instance.get_manifest_extra_jvm_args(&platform))
            .args(&instance.java_opts)
            .arg(format!("-Xms{}", instance.config.min))
            .arg(format!("-Xmx{}", instance.config.max))
            .arg(format!("-Djava.library.path={}", instance.build_natives()?))
            .arg(format!(
                "-Dminecraft.launcher.brand={}",
                env!("CARGO_PKG_NAME")
            )) // TODO: read from come config
            .arg(format!(
                "-Dminecraft.launcher.version={}",
                env!("CARGO_PKG_VERSION")
            ))
            .arg("-XX:+UnlockExperimentalVMOptions")
            .arg("-XX:+UseG1GC")
            .arg("-XX:G1NewSizePercent=20")
            .arg("-XX:G1ReservePercent=20")
            .arg("-XX:MaxGCPauseMillis=50")
            .arg("-XX:G1HeapRegionSize=32M")
/*             .arg("-cp")
            .arg(&instance.get_class_paths()) */
            .arg(
                &instance
                    .manifests
                    .get(&instance.uid)
                    .ok_or(Error::MetaNotFound)?
                    .main_class
                    .as_ref()
                    .ok_or(Error::MetaNotFound)?,
            )
            //.arg("net.minecraft.client.main.Main")
            .arg("--gameDir")
            .arg(&instance.minecraft_path)
            .arg("--assetsDir")
            .arg(&instance.get_assets_path())
            .arg("--accessToken")
            .arg(auth.get_token().unwrap_or("0"))
            .arg("--uuid")
            .arg(auth.get_uuid().unwrap_or("0"))
            .arg("--assetIndex")
            .arg(
                &instance
                    .manifests
                    .get(&instance.uid)
                    .ok_or(Error::MetaNotFound)?
                    .asset_index
                    .as_ref()
                    .ok_or(Error::MetaNotFound)?
                    .id,
            )
            .arg("--width")
            .arg(instance.config.width.to_string())
            .arg("--height")
            .arg(instance.config.height.to_string())
            .arg("--username")
            .arg(auth.get_username())
            .arg("--version")
            .arg(env!("CARGO_PKG_NAME"))
            //.arg(&instance.version)
            .arg(&instance.extra_args.join(" "))
            .current_dir(&instance.minecraft_path);

        let command_params = &mut command
            .get_args()
            .map(|s| s.to_str().unwrap_or("error"))
            .collect::<Vec<&str>>();
        // Find the position of the --accessToken argument and then move it by one position
        // So we can have the position of the Access Token
        let access_token_pos = command_params
            .iter()
            .position(|s| s == &"--accessToken")
            .unwrap();
        // censor the access token
        command_params[access_token_pos + 1] = "<REDACTED>";

        debug!("Classpath: {:#?}", std::env::var("CLASSPATH").unwrap());
        debug!(
            "Starting minecraft: {} {}",
            command.get_program().to_str().unwrap_or("error"),
            command_params.join(" ")
        );
        trace!("in workdir: {}", &instance.minecraft_path);

        let process = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(RunningInstance { process, instance })
    }
}
