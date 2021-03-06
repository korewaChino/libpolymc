use std::io::{stderr, stdout, Read, Write};

use anyhow::{anyhow, bail, Context, Result};
use clap::{App, Arg, ArgMatches};
use log::*;
use polymc::auth::Auth;
use polymc::instance::Instance;
use polymc::java_wrapper::Java;

pub(crate) fn app() -> App<'static> {
    App::new("run-raw")
        .about("Raw run an instance without any data storage")
        .arg(
            Arg::new("java")
                .long("java")
                .short('j')
                .env("PLMC_JAVA")
                .takes_value(true)
                .help("Path to the java executable")
                .required(true),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .short('v')
                .env("PLMC_VERSION")
                .takes_value(true)
                .help("The Minecraft version to run")
                .required(true),
        )
        .arg(
            Arg::new("mc_dir")
                .long("mc-dir")
                .short('d')
                .env("PLMC_MC_DIR")
                .takes_value(true)
                .help("The Minecraft directory")
                .required(true),
        )
        .arg(
            Arg::new("username")
                .long("username")
                .short('u')
                .env("PLMC_USERNAME")
                .takes_value(true)
                .help("The username to use for authentication")
                .required(true),
        )
        .arg(
            Arg::new("java_args")
                .long("java-argument")
                .short('a')
                .takes_value(true)
                .help("Java arguments to pass to the JVM")
                .multiple_values(true),
        )
        .arg(
            Arg::new("library_path")
                .long("library-path")
                .short('l')
                .env("PLMC_LIB_PATH")
                .help("List of libraries to add to the Minecraft classpath")
                .takes_value(true),
        )
}

pub(crate) fn run(sub_matches: &ArgMatches) -> Result<i32> {
    debug!("Running raw minecraft installation");
    let java = sub_matches.value_of("java").unwrap();
    debug!("using java: {}", java);
    let version = sub_matches.value_of("version").unwrap();
    let dir = sub_matches.value_of("mc_dir").unwrap();

    let auth = sub_matches.value_of("username").unwrap();
    // TODO: more than offline
    let auth = Auth::new_offline(auth);

    /*let mut instance = Instance::new(auth.get_username(), version, dir);
    let java = Java::new(java);

    if let Some(lib) = sub_matches.value_of("library_path") {
        trace!("Setting library path to: {}", lib);
        instance.set_libraries_path(lib);
    }

    let mut running = java.start(&instance, auth)?;

    let mut c_stdout = running
        .process
        .stdout
        .take()
        .context("Failed to get stdout")?;
    let mut c_stderr = running
        .process
        .stderr
        .take()
        .context("Failed to get stderr")?;

    std::thread::spawn(move || loop {
        let mut buf = [0u8; 255];
        match c_stdout.read(&mut buf) {
            Ok(_) => stdout().write(&buf).unwrap(),
            Err(_) => return,
        };
        std::thread::sleep(std::time::Duration::from_micros(100));
    });

    std::thread::spawn(move || loop {
        let mut buf = [0u8; 255];
        match c_stderr.read(&mut buf) {
            Ok(_) => stderr().write(&buf).unwrap(),
            Err(_) => return,
        };
        std::thread::sleep(std::time::Duration::from_micros(100));
    });

    let exit = running.process.wait()?;

    exit.code().ok_or(anyhow!("Failed to get exit code"))
     */
    bail!("Cannot find metadata")
}
