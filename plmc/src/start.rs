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

    Ok(1)
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
