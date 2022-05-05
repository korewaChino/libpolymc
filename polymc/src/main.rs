// The CLI
use clap::{App, ColorChoice};

use polymc::launcher::run;
#[tokio::main]
async fn main() {
    let ret = main_ret().await;
    std::process::exit(ret);
}

async fn main_ret() -> i32 {

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
        .subcommand(polymc::launcher::metadata::app())
        .subcommand(polymc::launcher::login::app());

    let matches = app.get_matches();

    if matches.is_present("debug") {
        std::env::set_var("RUST_LOG", "trace");
    } else {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    let ret = match matches.subcommand() {
        Some(("run", sub_matches)) => run::run(sub_matches).await,
        Some(("metadata", sub_matches)) => polymc::launcher::metadata::run(sub_matches).await,
        Some(("login", sub_matches)) => polymc::launcher::login::run(sub_matches).await,
        Some(("meta", sub_matches)) => polymc::launcher::metadata::run(sub_matches).await,
        _ => unreachable!("{:#?}", matches),
    };

    if let Err(e) = ret {
        eprintln!("Error executing:\n{:?}", e);
        1
    } else {
        ret.unwrap()
    }
}
