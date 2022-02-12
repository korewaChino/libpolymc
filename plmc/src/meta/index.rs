use anyhow::{bail, Context, Result};
use clap::{App, Arg, ArgMatches};
use hyper::body::HttpBody;
use hyper::client::connect::Connect;
use hyper::Client;
use log::*;
use mktemp::Temp;
use polymc::meta::{DownloadRequest, FileType, MetaIndex, MetaManager, Wants};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub(crate) fn app() -> App<'static> {
    App::new("index")
        .about("Parse a meta index definition")
        .arg(Arg::new("file").long("file").short('i').takes_value(true))
        .subcommand(
            App::new("search")
                .about("Search in meta index")
                .arg(
                    Arg::new("base_url")
                        .long("base-url")
                        .required(true)
                        .takes_value(true)
                        .env("PLMC_BASE_URL"),
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
                ),
        )
}

pub(crate) async fn run(sub_matches: &ArgMatches) -> Result<i32> {
    match sub_matches.subcommand() {
        None => run_index(sub_matches),
        Some(("search", sub_matches)) => run_search(sub_matches).await,
        _ => bail!("Unknown command"),
    }
}

fn run_index(sub_matches: &ArgMatches) -> Result<i32> {
    let file = sub_matches.value_of("file").unwrap();
    let mut file = OpenOptions::new()
        .read(true)
        .open(file)
        .context("Opening input file")?;

    let index = MetaIndex::from_reader(&mut file)?;

    println!("{}", serde_json::to_string_pretty(&index).unwrap());

    Ok(0)
}

async fn run_search(sub_matches: &ArgMatches) -> Result<i32> {
    let tmp_lib = Temp::new_dir()?;
    let tmp_meta = Temp::new_dir()?;
    let lib_dir = if let Some(dir) = sub_matches.value_of("lib_dir") {
        dir.to_string()
    } else {
        tmp_lib.display().to_string()
    };

    let meta_dir = if let Some(dir) = sub_matches.value_of("meta_dir") {
        dir.to_string()
    } else {
        tmp_meta.display().to_string()
    };

    let base_url = sub_matches.value_of("base_url").unwrap().to_string();

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();

    let mut client = Client::builder().build(https);

    let mut meta_manager = MetaManager::new(&lib_dir, &base_url);
    let wants = Wants::new("net.minecraft", "1.18.1"); // TODO: non hardcoded values

    meta_manager.search(wants)?;

    loop {
        let search = meta_manager.continue_search()?;
        if search.requests.len() == 0 {
            break;
        }

        for r in &search.requests {
            info!("requested: {:?}", r);
            let (mut file, f_type) = download(&mut client, r, &lib_dir, &meta_dir).await?;
            meta_manager.load_reader(&mut file, f_type)?;
        }
    }

    Ok(0)
}

async fn download<C: Connect + Clone + Send + Sync + 'static>(
    client: &mut Client<C>,
    request: &DownloadRequest,
    lib_dir: &str,
    meta_dir: &str,
) -> Result<(File, FileType)> {
    match request {
        DownloadRequest::Library { .. } => bail!("TODO: implement downloading library"),
        _ => download_meta(client, request, meta_dir).await,
    }
}

async fn download_meta<C: Connect + Clone + Send + Sync + 'static>(
    client: &mut Client<C>,
    request: &DownloadRequest,
    meta_dir: &str,
) -> Result<(File, FileType)> {
    // TODO: implement digest based on has_hash
    let filename = match request {
        DownloadRequest::MetaIndex { .. } => format!("{}/index.json", meta_dir),
        DownloadRequest::Index { uid, .. } => format!("{}/{}/index.json", meta_dir, uid),
        DownloadRequest::Manifest { uid, version, .. } => {
            format!("{}/{}/{}.json", meta_dir, uid, version)
        }
        _ => bail!("Could not find location to store meta data in"),
    };

    if Path::new(&filename).is_file() && request.has_hash() {
        let mut file = OpenOptions::new().read(true).open(&filename)?;

        let mut digest = ring::digest::Context::new(request.get_hash_algo().unwrap());

        loop {
            let mut buf = [0u8; 8192];
            let read = file.read(&mut buf)?;
            digest.update(&buf[..read]);
            if read < buf.len() {
                break;
            }
        }

        let digest = digest.finish();

        if digest.as_ref() == request.get_hash() {
            debug!("found {} in cache", request.get_url());
            file.seek(SeekFrom::Start(0))?;
            return Ok((file, request.request_type()));
        }
        info!("Cache mismatch for {}", request.get_url());
    }

    std::fs::create_dir_all(
        Path::new(&filename)
            .parent()
            .context("Filename has no parent")?,
    )?;

    let url = request.get_url().parse()?;

    let mut res = client.get(url).await?;
    if !res.status().is_success() {
        bail!("Failed to download file: {}", res.status());
    }

    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .append(false)
        .open(&filename)?;

    while let Some(chunk) = res.body_mut().data().await {
        file.write_all(&chunk?)?;
    }

    file.seek(SeekFrom::Start(0))?;

    Ok((file, request.request_type()))
}
