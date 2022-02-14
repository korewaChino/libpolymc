use crate::meta::manifest::{Sha1Sum, Sha256Sum};
use crate::meta::{Asset, AssetIndexInfo, MetaIndexPackage, PackageVersion};
use std::ffi::{CStr, CString};
use std::fmt::{Display, Formatter};
use std::os::raw::c_char;
use std::path::PathBuf;

use super::manifest::LibraryDownload;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum FileType {
    /// Index of Indexes in the meta directory
    MetaIndex,
    /// Index of versions
    Index,
    /// Version manifest
    Manifest,
    /// Library File (usually a jar file)
    Library,
    /// Asset index
    AssetIndex,
    /// Asset file (images, etc).
    Asset,
}

impl FileType {
    #[export_name = "download_type_hash_size"]
    pub extern "C" fn hash_size(&self) -> usize {
        match self {
            Self::MetaIndex => 0,
            Self::Library | Self::AssetIndex | Self::Asset => ring::digest::SHA1_OUTPUT_LEN,
            _ => ring::digest::SHA256_OUTPUT_LEN,
        }
    }

    #[export_name = "download_type_is_library"]
    pub extern "C" fn is_library(&self) -> bool {
        matches!(self, Self::Library)
    }

    #[export_name = "download_type_is_asset"]
    pub extern "C" fn is_asset(&self) -> bool {
        matches!(self, Self::Asset)
    }

    /// True if either the type is an asset or a library.
    #[export_name = "download_type_is_file"]
    pub extern "C" fn is_file(&self) -> bool {
        self.is_library() || self.is_asset()
    }

    pub fn get_hash_algo(&self) -> Option<&'static ring::digest::Algorithm> {
        use ring::digest;
        Some(match self {
            Self::Index => &digest::SHA256,
            Self::Manifest => &digest::SHA256,
            Self::Library | Self::AssetIndex | Self::Asset => &digest::SHA1_FOR_LEGACY_USE_ONLY,
            _ => return None,
        })
    }
}

impl Display for FileType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MetaIndex => "meta_index",
            Self::Index => "index",
            Self::Manifest => "manifest",
            Self::Library => "library",
            Self::AssetIndex => "asset_index",
            Self::Asset => "asset",
        })
    }
}

#[derive(Debug, Clone)]
pub enum DownloadRequest {
    MetaIndex {
        url: String,
    },
    Index {
        url: String,
        uid: String,
        hash: Sha256Sum,
    },
    Manifest {
        url: String,
        version: String,
        uid: String,
        hash: Sha256Sum,
    },
    Library {
        path: String,
        download: LibraryDownload,
    },
    AssetIndex {
        uid: String,
        version: String,
        info: AssetIndexInfo,
    },
    Asset {
        asset: Asset,
        uid: String,
        url: String,
        path: String,
    },
}

impl DownloadRequest {
    pub fn new_meta_index(url: String) -> Self {
        Self::MetaIndex { url }
    }

    pub fn new_package_index(base_url: &str, package: &MetaIndexPackage) -> Self {
        Self::Index {
            url: format!("{}/{}/index.json", base_url, package.uid),
            uid: package.uid.to_string(),
            hash: package.sha256.clone(),
        }
    }

    pub fn new_package_manifest(base_url: &str, uid: &str, package: &PackageVersion) -> Self {
        Self::Manifest {
            url: format!("{}/{}/{}.json", base_url, uid, package.version),
            version: package.version.to_string(),
            hash: package.sha256.clone(),
            uid: uid.to_string(),
        }
    }

    pub fn new_library(download: LibraryDownload, path: PathBuf) -> Self {
        Self::Library {
            download,
            path: path.display().to_string(),
        }
    }

    #[export_name = "download_request_type"]
    pub extern "C" fn request_type(&self) -> FileType {
        match self {
            Self::MetaIndex { .. } => FileType::MetaIndex,
            Self::Index { .. } => FileType::Index,
            Self::Manifest { .. } => FileType::Manifest,
            Self::Library { .. } => FileType::Library,
            Self::AssetIndex { .. } => FileType::AssetIndex,
            Self::Asset { .. } => FileType::Asset,
        }
    }

    #[export_name = "download_request_hash_size"]
    pub extern "C" fn hash_size(&self) -> usize {
        self.request_type().hash_size()
    }

    pub fn get_hash(&self) -> &[u8] {
        match self {
            Self::MetaIndex { .. } => &[],
            Self::Index { hash, .. } => hash.as_ref(),
            Self::Manifest { hash, .. } => hash.as_ref(),
            Self::Library { download, .. } => download.sha1.as_ref(),
            Self::AssetIndex { info, .. } => info.sha1.as_ref(),
            Self::Asset { asset, .. } => asset.hash.as_ref(),
        }
    }

    #[export_name = "download_request_has_hash"]
    pub extern "C" fn has_hash(&self) -> bool {
        self.hash_size() != 0
    }

    #[export_name = "download_request_is_library"]
    pub extern "C" fn is_library(&self) -> bool {
        self.request_type().is_library()
    }

    #[export_name = "download_request_is_asset"]
    pub extern "C" fn is_asset(&self) -> bool {
        self.request_type().is_asset()
    }

    #[export_name = "download_request_is_file"]
    pub extern "C" fn is_file(&self) -> bool {
        self.request_type().is_file()
    }

    pub fn get_hash_algo(&self) -> Option<&'static ring::digest::Algorithm> {
        self.request_type().get_hash_algo()
    }

    /// Get the hash of the file to download.
    /// If the type is MetaIndex `NULL` will be returned
    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "download_request_get_hash"]
    pub extern "C" fn get_hash_c(&self) -> *const u8 {
        match self {
            Self::MetaIndex { .. } => core::ptr::null(),
            _ => self.get_hash().as_ptr(),
        }
    }

    pub fn get_url(&self) -> &str {
        match self {
            Self::MetaIndex { url, .. } => url.as_str(),
            Self::Index { url, .. } => url.as_str(),
            Self::Manifest { url, .. } => url.as_str(),
            Self::Library { download, .. } => download.url.as_str(),
            Self::AssetIndex { info, .. } => info.url.as_str(),
            Self::Asset { url, .. } => url.as_str(),
        }
    }

    /// Get the url of request.
    /// The returned pointer has to be freed with [`free_str`] and not with free.
    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "download_request_get_url"]
    pub extern "C" fn get_url_c(&self) -> *mut c_char {
        let url = self.get_url();
        let url = CString::new(url);

        url.map(|u| u.into_raw())
            .unwrap_or(core::ptr::null_mut() as *mut _)
    }

    /// If the type is Library, this returns the expected path to save the file under.
    pub fn get_path(&self) -> Option<&str> {
        match self {
            Self::Library { path, .. } => Some(path),
            Self::Asset { path, .. } => Some(path),
            _ => None,
        }
    }

    /// If the type is Library, this returns the expected path to save the file under.
    /// The returned pointer has to be freed with [`free_str`] and not with free.
    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "download_request_get_path"]
    pub extern "C" fn get_path_c(&self) -> *mut c_char {
        match self.get_path() {
            Some(p) => CString::new(p)
                .map(|u| u.into_raw())
                .unwrap_or(core::ptr::null_mut()),
            None => core::ptr::null_mut(),
        }
    }
}
