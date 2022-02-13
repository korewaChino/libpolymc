use anyhow::{anyhow, Context, Result};
use clap::{App, Arg, ArgMatches};
use log::*;
use mktemp::Temp;
use polymc::auth::Auth;
use polymc::instance::Instance;
use polymc::java_wrapper::Java;
use polymc::meta::FileType::AssetIndex;
use polymc::meta::{DownloadRequest, MetaManager, Wants};
use tokio::io::{stderr, stdout};

fn get_dir(sub: &str) -> String {
    let mut dir = dirs::data_dir().unwrap();
    dir.push("plmc");
    dir.push(sub);
    dir.display().to_string()
}

pub(crate) fn app() -> App<'static> {
    App::new("run")
        .about("Run the game")
        .arg(
            Arg::new("java")
                .long("java")
                .short('j')
                .env("PLMC_JAVA")
                .takes_value(true)
                .help("Path to the java executable")
                .required(true),
        )
        .arg(
            Arg::new("mc_version")
                .long("version")
                .short('v')
                .env("PLMC_MC_VERSION")
                .help("The Minecraft version to run")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("uid")
                .long("uid")
                .env("PLMC_MC_UID")
                .help("The manifest UID to run")
                .default_value("net.minecraft"),
        )
        .arg(
            Arg::new("meta_url")
                .long("base-url")
                .env("PLMC_BASE_URL")
                .help("Base url of the meta server to use")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("lib_dir")
                .long("lib-dir")
                .takes_value(true)
                .env("PLMC_LIB_DIR"),
        )
        .arg(
            Arg::new("meta_dir")
                .long("meta-dir")
                .takes_value(true)
                .env("PLMC_META_DIR"),
        )
        .arg(
            Arg::new("mc_dir")
                .long("mc-dir")
                .short('d')
                .env("PLMC_MC_DIR")
                .takes_value(true)
                .help("The Minecraft directory"),
        )
        .arg(
            Arg::new("assets_dir")
                .long("assets-dir")
                .env("PLMC_ASSETS_DIR")
                .takes_value(true),
        )
        .arg(
            Arg::new("natives_dir")
                .long("natives-dir")
                .env("PLMC_NATIVE_DIR")
                .takes_value(true),
        )
        .arg(
            Arg::new("username")
                .long("username")
                .short('u')
                .env("PMLC_USERNAME")
                .takes_value(true)
                .help("The username to use for authentication")
                .default_value("Player"),
        )
        .arg(
            Arg::new("java_extra_args")
                .long("java-args")
                .takes_value(true)
                .multiple_values(true),
        )
}

pub(crate) async fn run(sub_matches: &ArgMatches) -> Result<i32> {
    let meta_url = sub_matches.value_of("meta_url").unwrap();
    let meta_dir = sub_matches
        .value_of("meta_dir")
        .map(ToString::to_string)
        .unwrap_or_else(|| get_dir("meta"));

    let lib_dir = sub_matches
        .value_of("lib_dir")
        .map(ToString::to_string)
        .unwrap_or_else(|| get_dir("lib"));

    let mc_dir = sub_matches
        .value_of("mc_dir")
        .map(ToString::to_string)
        .unwrap_or_else(|| get_dir("game"));

    let assets_dir = sub_matches
        .value_of("assets_dir")
        .map(ToString::to_string)
        .unwrap_or_else(|| get_dir("assets"));

    let version = sub_matches.value_of("mc_version").unwrap();
    let uid = sub_matches.value_of("uid").unwrap();
    let wants = Wants::new(uid, version);

    let mut manager = MetaManager::new(&lib_dir, &assets_dir, &meta_url);
    manager.search(wants);

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();

    let mut client = hyper::Client::builder().build(https);

    let search = loop {
        let search = manager.continue_search()?;
        if search.is_ready() {
            break search;
        }

        for r in &search.requests {
            info!("requested: {:?}", r);
            if r.is_file() {
                crate::meta::index::download_file(&mut client, r).await?;
            } else {
                let (file, f_type) =
                    crate::meta::index::download_meta(&mut client, r, &meta_dir).await?;
                if let Some(mut file) = file {
                    if let DownloadRequest::AssetIndex { version, uid, .. } = &r {
                        manager.load_asset_index_reader(uid, &version, &mut file)?;
                    } else {
                        manager.load_reader(&mut file, f_type)?;
                    }
                }
            }
        }
    };

    let mut instance = Instance::new(uid, &version, &mc_dir, search);
    instance.set_libraries_path(&lib_dir);

    if let Some(dir) = sub_matches.value_of("natives_dir") {
        instance.set_natives_path(dir);
    }

    let java = sub_matches.value_of("java").unwrap();
    let java = Java::new(java);

    let mut child = java.start(&instance, Auth::new_offline("foo"))?;

    let mut c_stdout = child
        .process
        .stdout
        .take()
        .context("Failed to get stdout")?;
    let mut c_stderr = child
        .process
        .stderr
        .take()
        .context("Failed to get stderr")?;

    tokio::spawn(async move {
        let mut c_stdout = tokio::process::ChildStdout::from_std(c_stdout).unwrap();
        loop {
            tokio::io::copy(&mut c_stdout, &mut stdout()).await.unwrap();
        }
    });
    tokio::spawn(async move {
        let mut c_stderr = tokio::process::ChildStderr::from_std(c_stderr).unwrap();
        loop {
            tokio::io::copy(&mut c_stderr, &mut stderr()).await.unwrap();
        }
    });

    let exit = child.process.wait()?;

    Ok(exit.code().context("Failed to get exit code")?)
}
