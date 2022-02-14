mod meta;
mod run;
mod run_raw;

use clap::{App, ColorChoice};

#[tokio::main]
async fn main() {
    let ret = main_ret().await;
    std::process::exit(ret);
}

async fn main_ret() -> i32 {
    pretty_env_logger::init();

    let app = App::new("plmc")
        .about("libpolymc cli interface")
        .color(ColorChoice::Auto)
        .subcommand(run_raw::app())
        .subcommand(run::app())
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(meta::app());

    let matches = app.get_matches();

    let ret = match matches.subcommand() {
        Some(("run-raw", sub_matches)) => run_raw::run(sub_matches),
        Some(("run", sub_matches)) => run::run(sub_matches).await,
        Some(("meta", sub_matches)) => meta::run(sub_matches).await,
        _ => unreachable!(),
    };

    if let Err(e) = ret {
        eprintln!("Error executing:\n{:?}", e);
        1
    } else {
        ret.unwrap()
    }
}
