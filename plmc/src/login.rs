use anyhow::{anyhow, Context, Result};
use clap::{App, Arg, ArgMatches};
use console::style;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use log::*;
use polymc::auth::Auth;
use rand::seq::SliceRandom;
use rand::Rng;
use std::{
    string,
    time::{Duration, Instant},
};

pub(crate) fn app() -> App<'static> {
    App::new("login").about("Login to the minecraft server")
}

pub(crate) async fn run(sub_matches: &ArgMatches) -> Result<i32> {
    //TODO: Implement this shit. CC: @kloenk
    todo!();
}
