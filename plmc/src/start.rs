use anyhow::{anyhow, Context, Result};
use clap::{App, Arg, ArgMatches};
use log::*;
use mktemp::Temp;
use polymc::auth::Auth;
use polymc::instance::Instance;
use polymc::java_wrapper::Java;
use polymc::meta::FileType::AssetIndex;
use polymc::meta::{DownloadRequest, MetaManager, Wants};
use tokio::io::{stderr, stdout};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use rand::Rng;
use std::time::{Duration, Instant};
use console::{style};
use serde_json::{json, Value};

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

pub(crate) fn run(matches: &ArgMatches) -> Result<()> {
    let instance = matches.value_of("instance").unwrap();
    // Check if instance is is a path
    if instance.contains("/") {
        let instance = instance.to_string();
    }
    // else the instance is {PLMC_DIR}/instances/{instance}.json
    else {
        let instance = get_dir("instances") + "/" + instance + ".json";
    }
}