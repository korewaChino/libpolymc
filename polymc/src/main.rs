// The CLI
use clap::{App, ColorChoice};

use polymc::launcher::run;
#[tokio::main]
async fn main() {
    let ret = main_ret().await;
    std::process::exit(ret);
}

async fn main_ret() -> i32 {
    pretty_env_logger::init();

    let app = App::new("polymc-cli")
        .about("libpolymc cli interface")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .color(ColorChoice::Auto)
        .arg(
            clap::Arg::new("debug")
                .long("debug")
                .help("Enable debug logging"),
        )
        .subcommand(run::app())
        .subcommand(polymc::launcher::metadata::app());

    let matches = app.get_matches();

    if matches.is_present("debug") {
        std::env::set_var("RUST_LOG", "debug");
    } else {
        std::env::set_var("RUST_LOG", "info");
    }

    let ret = match matches.subcommand() {
        Some(("run", sub_matches)) => run::run(sub_matches).await,
        _ => unreachable!(),
    };

    if let Err(e) = ret {
        eprintln!("Error executing:\n{:?}", e);
        1
    } else {
        ret.unwrap()
    }
}
