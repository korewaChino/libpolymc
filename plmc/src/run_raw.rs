use anyhow::Result;
use clap::{App, Arg, ArgMatches};
use libpolymc::auth::Auth;
use libpolymc::instance::Instance;
use libpolymc::java_wrapper::Java;
use log::{debug, info};

pub(crate) fn app() -> App<'static> {
    App::new("run-raw")
        .about("Raw run an instance without any data storage")
        .arg(
            Arg::new("java")
                .long("java")
                .env("PLMC_JAVA")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .short('v')
                .env("PLMC_VERSION")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("mc_dir")
                .long("mc-dir")
                .env("PLMC_MC_DIR")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("username")
                .long("username")
                .env("PLMC_USERNAME")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("java_args")
                .long("java-argument")
                .takes_value(true)
                .multiple_values(true),
        )
}

pub(crate) fn run(sub_matches: &ArgMatches) -> Result<()> {
    debug!("Running raw minecraft installation");
    let java = sub_matches.value_of("java").unwrap();
    debug!("using java: {}", java);
    let version = sub_matches.value_of("version").unwrap();
    let dir = sub_matches.value_of("mc_dir").unwrap();

    let auth = sub_matches.value_of("username").unwrap();
    // TODO: more than offline
    let auth = Auth::new_offline(auth);

    let instance = Instance::new(auth.get_username(), version, dir);
    let java = Java::new(java);

    let mut running = java.start(&instance, auth)?;

    let output = running.process.wait_with_output()?;
    info!("stdout: {}", String::from_utf8_lossy(&output.stderr));

    Ok(())
}
