use crate::config::*;
use crate::auth::Auth;
use anyhow::{Context, Result};
use anyhow::{bail, Ok};
use clap::{App, Arg, ArgMatches};

pub fn app() -> App<'static> {
    App::new("login")
        .about("Logs in and creates a profile")
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .subcommand(
            App::new("microsoft")
                .about("Logs in with Microsoft")
        )
}

pub async fn run(sub_matches: &ArgMatches) -> Result<i32> {
    match sub_matches.subcommand() {
        Some(("microsoft", _sub_matches)) => msft_login().await,
        _ => bail!("no command given"),
    }
}

pub async fn msft_login() -> Result<i32> {
    let auth = Auth::new_microsoft(None).await;

    //println!("{:#?}", auth);

    let config_profile = AuthProfile {
        name: auth.get_username().to_string(),
        auth,
    };

    config_profile.write_to_file();
    Ok(0)
}