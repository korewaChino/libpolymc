pub mod index;
mod manifest;

use anyhow::{bail, Result};
use clap::{App, ArgMatches};

pub(crate) fn app() -> App<'static> {
    App::new("meta")
        .about("Parse meta files and print the rust representation of them")
        .subcommand(manifest::app())
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .subcommand(index::app())
}

pub(crate) async fn run(sub_matches: &ArgMatches) -> Result<i32> {
    match sub_matches.subcommand() {
        Some(("manifest", sub_matches)) => manifest::run(sub_matches),
        Some(("index", sub_matches)) => index::run(sub_matches).await,
        _ => bail!("no command given"),
    }
}
