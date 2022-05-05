use crate::meta::{DownloadRequest, FileType, MetaIndex, MetaManager, Wants};
use anyhow::{bail, Context, Result};
use clap::{App, Arg, ArgMatches};
use hyper::body::HttpBody;
use hyper::client::connect::Connect;
use hyper::Client;
use log::*;
use mktemp::Temp;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub(crate) fn app() -> App<'static> {
    App::new("index")
        .about("Parse a meta index definition")
        .arg(Arg::new("file").long("file").short('i').takes_value(true))
        .setting(clap::AppSettings::ArgRequiredElseHelp)
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
                    Arg::new("assets_dir")
                        .long("assets-dir")
                        .env("PLMC_ASSETS_DIR")
                        .takes_value(true),
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
    let tmp_assets = Temp::new_dir()?;
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

    let assets_dir = if let Some(dir) = sub_matches.value_of("assets_dir") {
        dir.to_string()
    } else {
        tmp_assets.display().to_string()
    };

    let base_url = sub_matches.value_of("base_url").unwrap().to_string();

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();

    let mut client = Client::builder().build(https);

    let mut meta_manager = MetaManager::new(&lib_dir, &assets_dir, &base_url);
    let wants = Wants::new("net.minecraft", "1.18.1"); // TODO: non hardcoded values

    meta_manager.search(wants)?;

    loop {
        let search = meta_manager.continue_search()?;
        if search.requests.is_empty() {
            break;
        }

        for r in &search.requests {
            info!("requested: {:?}", r);
            if r.is_file() {
                download_file(&mut client, r).await?;
            } else {
                let (file, f_type) = download_meta(&mut client, r, &meta_dir).await?;
                if file.is_some() {
                    if matches!(f_type, FileType::AssetIndex) {
                    } else {
                        meta_manager.load_reader(&mut file.unwrap(), f_type)?;
                    }
                }
            }
        }
    }

    Ok(0)
}

pub async fn download_file<C: Connect + Clone + Send + Sync + 'static>(
    client: &mut Client<C>,
    request: &DownloadRequest,
) -> Result<()> {
    let filename = request.get_path().unwrap();

    if verify_hash(&filename, request).is_ok() {
        return Ok(());
    }

    std::fs::create_dir_all(
        Path::new(filename)
            .parent()
            .context("Filename has no parent")?,
    )?;

    let url = request.get_url().parse()?;

    // Follow redirects
    let mut res = client.get(url).await?;

    if !res.status().is_success() {
        // check status code
        trace!("{:?}", res.status());
        if res.status().is_redirection() {
            // follow redirect and try again
            let url = res.headers().get("location").unwrap().to_str()?;
            let url = url.parse()?;
            debug!("redirected to: {}", url);
            res = client.get(url).await?;
        } else {
            bail!(
                "Failed to download file: {} ({})",
                request.get_url(),
                res.status()
            );
        }
    }

    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .append(false)
        .open(&filename)?;

    let mut digest = ring::digest::Context::new(request.get_hash_algo().unwrap());

    while let Some(chunk) = res.body_mut().data().await {
        let chunk = chunk?;
        digest.update(&chunk);
        file.write_all(&chunk)?;
    }

    let digest = digest.finish();
    if digest.as_ref() != request.get_hash() {
        bail!("Failed to download file, got invalid hash");
    }

    Ok(())
}

pub async fn download_meta<C: Connect + Clone + Send + Sync + 'static>(
    client: &mut Client<C>,
    request: &DownloadRequest,
    meta_dir: &str,
) -> Result<(Option<File>, FileType)> {
    // TODO: implement digest based on has_hash
    let filename = match request {
        DownloadRequest::MetaIndex { .. } => format!("{}/index.json", meta_dir),
        DownloadRequest::Index { uid, .. } => format!("{}/{}/index.json", meta_dir, uid),
        DownloadRequest::Manifest { uid, version, .. } => {
            format!("{}/{}/{}.json", meta_dir, uid, version)
        }
        DownloadRequest::AssetIndex { path, .. } => path.to_string(),
        _ => bail!("Could not find location to store meta data in"),
    };

    if let Ok(file) = verify_hash(&filename, request) {
        return Ok((Some(file), request.request_type()));
    } else {
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

    let mut digest = if request.has_hash() {
        Some(ring::digest::Context::new(request.get_hash_algo().unwrap()))
    } else {
        None
    };

    while let Some(chunk) = res.body_mut().data().await {
        let chunk = chunk?;
        if let Some(digest) = digest.as_mut() {
            digest.update(&chunk);
        }
        file.write_all(&chunk)?;
    }

    // TODO: check hash
    /*if let Some(digest) = digest {
        let digest = digest.finish();
        if digest.as_ref() != request.get_hash() {
            warn!("Hash mismatch after downloading file");
            return Ok((None, request.request_type()));
        }
    }*/

    file.seek(SeekFrom::Start(0))?;

    Ok((Some(file), request.request_type()))
}

fn verify_hash(filename: &str, request: &DownloadRequest) -> Result<File> {
    if !request.has_hash() {
        bail!("Request has no hash");
    }
    debug!("Verifying hash for {}", filename);
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
        return Ok(file);
    }

    bail!("Invalid Hash");
}
