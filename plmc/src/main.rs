mod meta;
mod run_raw;

use clap::{App, ColorChoice};

fn main() {
    let ret = main_ret();
    std::process::exit(ret);
}

fn main_ret() -> i32 {
    pretty_env_logger::init();

    let app = App::new("plmc")
        .about("libpolymc cli interface")
        .color(ColorChoice::Auto)
        .subcommand(run_raw::app())
        .subcommand(meta::app());

    let matches = app.get_matches();

    let ret = match matches.subcommand() {
        Some(("run-raw", sub_matches)) => run_raw::run(sub_matches),
        Some(("meta", sub_matches)) => meta::run(sub_matches),
        _ => unreachable!(),
    };

    if let Err(e) = ret {
        eprintln!("Error executing:\n{:?}", e);
        1
    } else {
        ret.unwrap()
    }
}
