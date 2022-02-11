mod run_raw;

use clap::{App, Arg, ColorChoice};

fn main() {
    pretty_env_logger::init();

    let app = App::new("plmc")
        .about("libpolymc cli interface")
        .color(ColorChoice::Auto)
        .subcommand(run_raw::app());

    let matches = app.get_matches();

    let ret = match matches.subcommand() {
        Some(("run-raw", sub_matches)) => run_raw::run(sub_matches),
        _ => unreachable!(),
    };

    if let Err(e) = ret {
        eprintln!("Error executing:\n{:?}", e);
    }
}
