use log::*;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::meta::manifest::Sha1Sum;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexInfo {
    pub id: String,
    pub sha1: Sha1Sum,
    pub size: i64,
    pub total_size: i64,
    pub url: String,

    #[serde(skip)]
    pub cache: Option<AssetIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub objects: HashMap<String, Asset>,
}

impl AssetIndex {
    pub fn verify_at(&self, at: &str) -> Result<Vec<(Asset, Error)>> {
        let mut ret = Vec::new();
        for (_name, asset) in &self.objects {
            if let Err(e) = asset.verify_at(at) {
                match e {
                    Error::LibraryMissing => ret.push((asset.clone(), e)),
                    Error::LibraryInvalidHash => ret.push((asset.clone(), e)),
                    _ => return Err(e),
                }
            }
        }

        Ok(ret)
    }

    /// Verify all data.
    /// # Safety
    /// This uses write without synchronization, so only run one instance on a given dataset.
    pub unsafe fn verify_caching_at(&self, at: &str) -> Result<Vec<(Asset, Error)>> {
        info!("Verifying asset index cache at {}", at);
        let mut ret = Vec::new();
        for (_name, asset) in &self.objects {
            if let Err(e) = unsafe { asset.verify_caching_at(at) } {
                match e {
                    Error::LibraryMissing => ret.push((asset.clone(), e)),
                    Error::LibraryInvalidHash => ret.push((asset.clone(), e)),
                    _ => return Err(e),
                }
            }
        }

        Ok(ret)
    }
}

crate::meta::index::from_str_json!(AssetIndex);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub hash: Sha1Sum,
    pub size: i64,

    #[serde(skip)]
    verified: std::rc::Rc<UnsafeCell<bool>>,
}

impl Asset {
    pub fn path_at(&self, at: &str) -> String {
        let mut path = Path::new(at).to_path_buf();
        path.push("objects");
        path.push(hex::encode(&self.hash.as_ref()[0..1]));
        path.push(hex::encode(&self.hash.as_ref()));

        path.display().to_string()
    }

    pub fn verify_at(&self, at: &str) -> Result<()> {
        #[cfg(debug_assertions)]
        trace!("verifying asset: {}", hex::encode(self.hash.as_ref()));

        let path = self.path_at(at);

        if !Path::new(&path).is_file() {
            return Err(Error::LibraryMissing);
        }

        let mut file = OpenOptions::new().read(true).open(path)?;

        let mut digest = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);

        loop {
            let mut buf = [0u8; 8192];
            let read = file.read(&mut buf)?;
            digest.update(&buf[..read]);
            if read < buf.len() {
                break;
            }
        }

        let digest = digest.finish();

        if digest.as_ref() == self.hash.as_ref() {
            trace!("{} is valid", hex::encode(self.hash.as_ref()));
            Ok(())
        } else {
            Err(Error::LibraryInvalidHash)
        }
    }

    /// Verify all data.
    /// # Safety
    /// This uses write without synchronization, so only run one instance on a given dataset.
    pub unsafe fn verify_caching_at(&self, at: &str) -> Result<()> {
        if unsafe { *self.verified.get() } {
            Ok(())
        } else {
            if let Err(e) = self.verify_at(at) {
                Err(e)
            } else {
                unsafe {
                    let verified = &mut *self.verified.get();
                    *verified = true;
                }
                Ok(())
            }
        }
    }
}
