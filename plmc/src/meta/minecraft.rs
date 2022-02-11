use anyhow::{Context, Result};
use clap::{App, Arg, ArgMatches};
use libpolymc::meta::minecraft::Minecraft;
use std::fs::OpenOptions;
use std::io::Read;

pub(crate) fn app() -> App<'static> {
    App::new("minecraft")
        .about("Parse a minecraft meta definition")
        .arg(
            Arg::new("file")
                .long("file")
                .short('i')
                .takes_value(true)
                .required(true),
        )
}

pub(crate) fn run(sub_matches: &ArgMatches) -> Result<i32> {
    let file = sub_matches.value_of("file").unwrap();
    let mut file = OpenOptions::new()
        .read(true)
        .open(file)
        .context("Opening input file")?;

    let meta = libpolymc::meta::minecraft::Minecraft::from_reader(file)?;

    //println!("{:?}", meta);

    let foo = &meta.libraries[0].name;
    println!("{}", foo.at_path("/").display());

    Ok(0)
}
