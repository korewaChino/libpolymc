use anyhow::{anyhow, Context, Ok, Result};
use clap::{App, Arg, ArgMatches};
use console::style;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use log::*;
use mktemp::Temp;
use polymc::auth::Auth;
use polymc::instance::Instance;
use polymc::java_wrapper::Java;
use polymc::meta::FileType::AssetIndex;
use polymc::meta::{DownloadRequest, MetaManager, Wants};
use rand::seq::SliceRandom;
use rand::Rng;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::io::{stderr, stdout};

fn get_dir(sub: &str) -> String {
    let mut dir = dirs::data_dir().unwrap();
    dir.push("plmc");
    dir.push(sub);
    dir.display().to_string()
}

pub(crate) fn app() -> App<'static> {
    App::new("start")
        .about("Starts the game from an instance profile")
        .arg(
            // default argument
            Arg::new("instance")
                .long("instance")
                .short('i')
                .env("PLMC_INSTANCE")
                .help("The instance profile to use")
                .takes_value(true)
                .required(true),
        )
}
pub(crate) async fn run(sub_matches: &ArgMatches) -> Result<i32> {
    let instance = sub_matches.value_of("instance").unwrap();
    // check if instance is a path
    let instance = if instance.contains("/") {
        instance.to_string()
    } else {
        // get instance profile from {plmc_dir}/instances/{instance}
        get_dir("instances") + "/" + format!("{}.json", instance).as_str()
    };
    let contents = std::fs::read_to_string(instance).context("Could not read instance file")?;
    // println!("{}", contents);
    let config: serde_json::Value =
        serde_json::from_str(&contents).expect("Invalid JSON in instance file");
    println!("{:#?}", config);
    // get name from config
    let name = config["name"].as_str().unwrap_or("Unnamed instance");
    println!("Starting instance {}", name);

    let meta_url = config["base_url"].as_str().unwrap_or("https://meta.polymc.org/v1");
    let meta_dir = get_dir("meta");
    let lib_dir = get_dir("lib");
    let default_mc_dir = get_dir("game");
    // this is messy, send help
    let mc_dir = config["game_path"].as_str().unwrap_or(&default_mc_dir);
    let username = config["username"].as_str().unwrap_or("Player");
    let assets_dir = get_dir("assets");
    let version = config["version"].as_str().unwrap(); // Required
    let uid = config["uid"].as_str().unwrap_or("net.minecraft");
    let java = config["java_path"].as_str().unwrap();


    let wants = Wants::new(uid, version);

    let mut manager = MetaManager::new(&lib_dir, &assets_dir, &meta_url);
    #[warn(unused_must_use)]
    manager.search(wants);

    // check if extra packages are specified
    //let extra_packages = sub_matches.values_of("extra_packages").unwrap_or_default();
    //let package_version = sub_matches.value_of("package_version").unwrap_or_default();

/*     for package in extra_packages {
        #[warn(unused_must_use)]
        manager.search(Wants::new(package, package_version));
    } */

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();

    let mut client = hyper::Client::builder().build(https);

    // Let's use indicatif to show the progress!
    let spinner_style = ProgressStyle::default_bar()
        .tick_chars("|\\-/")
        .progress_chars("=> ")
        .template("{prefix:.bold.dim} {spinner} [{bar}] {msg}");
    println!("Downloading Assets...");

    let search = loop {
        let search = manager.continue_search()?;
        if search.is_ready() {
            break search;
        }
        // get the total amount of files to download
        // total is search.requests's length, but we have to return the variable because rust
        let mut total = search.requests.len();
        let pb = ProgressBar::new(total as u64);
        pb.set_style(spinner_style.clone());
        pb.set_message("Loading...");
        // draw the progress bar
        for r in &search.requests {
            info!("requested: {:?}", r);
            if r.is_file() {
                // print download progress
                // set the progress bar to the current file
                pb.set_message(format!(
                    "[{}/{}] Downloading {}",
                    pb.position(),
                    total,
                    r.get_url()
                ));
                //println!("Downloading {}", r.get_url());
                crate::meta::index::download_file(&mut client, r).await?;
                pb.inc(1);
            } else {
                // print download progress
                pb.set_message(format!("Loading Metadata from {}", r.get_url()));
                let (file, f_type) =
                    crate::meta::index::download_meta(&mut client, r, &meta_dir).await?;
                if let Some(mut file) = file {
                    if let DownloadRequest::AssetIndex { version, uid, .. } = &r {
                        manager.load_asset_index_reader(uid, &version, &mut file)?;
                    } else {
                        manager.load_reader(&mut file, f_type)?;
                    }
                }
                pb.inc(1);
            }
        }
        pb.finish();
    };
    let mut instance = Instance::new(uid, &version, &mc_dir, search);
    instance.set_libraries_path(&lib_dir);
    let mut extras = Vec::new();

    /* if let Some(extra_args) = sub_matches.values_of("extra_args") {
        extras.extend(extra_args.map(ToString::to_string));
    } */
    // TODO Add support for extra flags

    // if demo_mode is true add --demo to the extra args
/*     if sub_matches.is_present("demo_mode") {
        if sub_matches.value_of("demo_mode").unwrap() == "true" {
            extras.push("--demo".to_string());
        }
    }
 */
    instance.set_extra_args(extras);

/*     if let Some(dir) = sub_matches.value_of("natives_dir") {
        instance.set_natives_path(dir);
    }
 */
    instance.set_assets_path(&assets_dir);

    let java = Java::new(java);

    let mut child = java.start(&instance, Auth::new_offline(username))?;

    let mut c_stdout = child
        .process
        .stdout
        .take()
        .context("Failed to get stdout")?;
    let mut c_stderr = child
        .process
        .stderr
        .take()
        .context("Failed to get stderr")?;

    tokio::spawn(async move {
        let mut c_stdout = tokio::process::ChildStdout::from_std(c_stdout).unwrap();
        loop {
            tokio::io::copy(&mut c_stdout, &mut stdout()).await.unwrap();
        }
    });
    tokio::spawn(async move {
        let mut c_stderr = tokio::process::ChildStderr::from_std(c_stderr).unwrap();
        loop {
            tokio::io::copy(&mut c_stderr, &mut stderr()).await.unwrap();
        }
    });

    let exit = child.process.wait()?;

    Ok(exit.code().context("Failed to get exit code")?)


}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    use std::io::prelude::*;
    fn test_run() {
        let mut file = File::create("test.json").unwrap();
        file.write_all(b"{\"test\":\"test\"}").unwrap();
        file.sync_all().unwrap();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        println!("{}", contents);
    }
}
