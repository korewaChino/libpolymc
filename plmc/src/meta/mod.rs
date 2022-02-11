mod minecraft;

use anyhow::{bail, Result};
use clap::{App, Arg, ArgMatches};

pub(crate) fn app() -> App<'static> {
    App::new("meta")
        .about("Parse meta files and print the rust representation of them")
        .subcommand(minecraft::app())
}

pub(crate) fn run(sub_matches: &ArgMatches) -> Result<i32> {
    match sub_matches.subcommand() {
        Some(("minecraft", sub_matches)) => minecraft::run(sub_matches),
        _ => bail!("no command given"),
    }
}
