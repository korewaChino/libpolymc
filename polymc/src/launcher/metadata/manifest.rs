use crate::meta::manifest::{Manifest, OS};
use anyhow::{Context, Result};
use clap::{App, Arg, ArgMatches};
use log::trace;
use std::fs::OpenOptions;

pub(crate) fn app() -> App<'static> {
    App::new("manifest")
        .about("Parse a minecraft meta definition")
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::new("file")
                .long("file")
                .short('i')
                .takes_value(true)
                .required(true),
        )
        .subcommand(
            App::new("lib")
                .about("build/verify library path")
                .arg(
                    Arg::new("dir")
                        .long("dir")
                        .short('d')
                        .takes_value(true)
                        .required(true),
                )
                .arg(Arg::new("os").long("os").takes_value(true).required(true))
                .arg(Arg::new("verify").long("verify")),
        )
}

pub(crate) fn run(sub_matches: &ArgMatches) -> Result<i32> {
    let file = sub_matches.value_of("file").unwrap();
    let mut file = OpenOptions::new()
        .read(true)
        .open(file)
        .context("Opening input file")?;

    let meta = Manifest::from_reader(&mut file)?;

    match sub_matches.subcommand() {
        Some(("lib", sub_matches)) => run_lib(sub_matches, meta),
        _ => {
            println!("{:?}", meta);
            Ok(0)
        }
    }
}

fn run_lib(sub_matches: &ArgMatches, meta: Manifest) -> Result<i32> {
    let dir = sub_matches.value_of("dir").unwrap();
    let os = sub_matches.value_of("os").unwrap(); // TODO: get default somehow
    let os = OS::new(os);

    if sub_matches.is_present("verify") {
        let verify = meta.verify_at(dir, &os)?;
        if !verify.is_empty() {
            println!("Failed to verify libraries:");
            for (lib, e) in verify {
                println!("{}: {}", lib.name, e);
                trace!("{:?}", lib);
            }
            return Ok(1);
        }
    }

    let libraries = meta.build_class_path_at(dir, &os);
    println!("{}", libraries);

    Ok(0)
}
