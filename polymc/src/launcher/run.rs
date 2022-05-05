use crate::config::{GlobalConfig, AuthProfile};
use crate::meta::manifest::Package;
// use upper crate
use crate::instance::Instance;
use crate::java_wrapper::Java;
use crate::meta::{DownloadRequest, MetaManager, Wants};
use crate::util::*;
use crate::{auth::Auth, meta::SearchResult};
use anyhow::{Context, Result};
use clap::{App, Arg, ArgMatches};
//use console::style;
//use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use log::*;
use tokio::io::{stderr, stdout};

pub fn app() -> App<'static> {
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
                .takes_value(true),
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
                .help("The username to use for authentication"),
        )
        .arg(
            Arg::new("java_extra_args")
                .long("java-args")
                .takes_value(true)
                .multiple_values(true),
        )
        // TODO: Implement this
        .arg(
            Arg::new("demo_mode")
                .long("demo-mode")
                .help("Run in demo mode")
                .takes_value(false)
                .default_value("false"),
        )
        .arg(
            Arg::new("extra_args")
                .long("extra-args")
                .takes_value(true)
                .help("Extra flags to pass to Minecraft")
                .multiple_values(true),
        )
        .arg(
            Arg::new("extra_packages")
                .long("package")
                .short('p')
                .takes_value(true)
                .help("Extra packages to install"),
        )
        .arg(
            Arg::new("package_version")
                .long("package-version")
                .short('V')
                .takes_value(true)
                .help("The version of the package to install"),
        )
        
}

pub async fn run(sub_matches: &ArgMatches) -> Result<i32> {

    let config = GlobalConfig::load();
    //println!("{:#?}", config);


    // Get default login profile
    let default_profile = config.default_user_profile;
    // Get the username from the command line only if it's set
    let username = if let Some(username) = sub_matches.value_of("username") {
        username.to_string()
    } else {
        // Try reading the default profile to see if it's set or it's blank
        if default_profile.is_empty() {
            warn!("No username given and no default profile set");
            "Player".to_string()
        } else {
            default_profile.clone()
        }
    };

    // I dont even know if the above code works lol

    let auth = AuthProfile::load_profile(&username).auth;

    let meta_url = sub_matches
        .value_of("meta_url")
        .map(ToString::to_string)
        .unwrap_or_else(|| "https://meta.polymc.org/v1/".to_string());
    let meta_dir = sub_matches
        .value_of("meta_dir")
        .map(ToString::to_string)
        .unwrap_or_else(|| get_dir("meta"));
    let lib_dir = sub_matches
        .value_of("lib_dir")
        .map(ToString::to_string)
        .unwrap_or_else(|| get_dir("lib"));

    let natives_dir = sub_matches
        .value_of("natives_dir")
        .map(ToString::to_string)
        .unwrap_or_else(|| get_dir("natives"));

    let mc_dir = sub_matches
        .value_of("mc_dir")
        .map(ToString::to_string)
        .unwrap_or_else(|| get_dir("minecraft"));


    let assets_dir = sub_matches
        .value_of("assets_dir")
        .map(ToString::to_string)
        .unwrap_or_else(|| get_dir("assets"));

    let version = sub_matches.value_of("mc_version").unwrap();
    let uid = sub_matches
        .value_of("uid")
        .map(ToString::to_string)
        .unwrap_or_else(|| "net.minecraft".to_string());

    let java_path = sub_matches.value_of("java").unwrap();

    let meta = MetaManager::new(&lib_dir, &assets_dir, &meta_url);

    let wants = Wants::new(&uid, version);

    let results = download_meta(meta, wants, None, Some(&meta_dir)).await;
    let search = results.unwrap();
    // For authentication profiles

    let mut instance = Instance::new(&uid, &version, &mc_dir, search);

    instance.set_libraries_path(&lib_dir);
    instance.set_assets_path(&assets_dir);
    instance.set_natives_path(&natives_dir);

    run_instance(&instance, &Java::new(java_path), auth)
}

pub fn run_instance(instance: &Instance, java: &Java, auth: Auth) -> Result<i32> {
    let mut child = java.start(&instance, auth)?;
    let c_stdout = child
        .process
        .stdout
        .take()
        .context("Failed to get stdout")?;
    let c_stderr = child
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

#[allow(unused_must_use)]
pub async fn download_meta(
    mut meta: MetaManager,
    wants: Wants,
    extra_packages: Option<&[Package]>,
    meta_cache: Option<&str>,
) -> Result<SearchResult> {
    // Search for things we want
    meta.search(wants);

    let extras = extra_packages.unwrap_or_default();

    // Extra packages
    for pkg in extras {
        meta.search(Wants::new(&pkg.name, &pkg.version));
    }
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();

    let mut client = hyper::Client::builder().build::<_, hyper::Body>(https);

    let search = loop {
        let search = meta.continue_search()?;
        if search.is_ready() {
            break search;
        }

        // For each Download request in search results

        for r in &search.requests {
            debug!("Downloading {}", r.get_url());
            if r.is_file() {
                crate::launcher::metadata::index::download_file(&mut client, r).await?;
                // Spawn a thread and download the file
                //tokio::spawn(crate::launcher::metadata::index::download_file(&mut client, r));
                //let download = tokio::spawn(metaIndex::download_file(&mut client, r));
            } else {
                let (file, f_type) = crate::launcher::metadata::index::download_meta(
                    &mut client,
                    r,
                    meta_cache.unwrap_or("./meta_cache"),
                )
                .await?;
                if let Some(mut file) = file {
                    if let DownloadRequest::AssetIndex { version, uid, .. } = &r {
                        meta.load_asset_index_reader(uid, &version, &mut file)?;
                    } else {
                        meta.load_reader(&mut file, f_type)?;
                    }
                }
            }
        }
    };
    // Return the search result
    Ok(search)
}
