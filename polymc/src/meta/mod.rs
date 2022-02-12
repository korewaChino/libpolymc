use log::*;
use std::ffi::CStr;
use std::io::Read;
use std::os::raw::c_char;

use crate::Result;

mod index;
pub mod manifest;
mod request;

pub use index::*;
pub use request::*;

pub struct MetaManager {
    pub library_path: String,
    pub base_url: String,
    wants: Vec<Wants>,
    pub index: Option<MetaIndex>,
}

impl MetaManager {
    /// Create A new MetaManager.
    pub fn new(library_path: &str, base_url: &str) -> Self {
        Self {
            library_path: library_path.to_string(),
            base_url: base_url.to_string(),
            wants: Vec::new(),
            index: None,
        }
    }

    pub fn search(&mut self, what: Wants) -> Result<SearchResult> {
        self.wants.push(what);

        if self.index.is_none() {
            let index = DownloadRequest::new_meta_index(self.index_url());
            return Ok(SearchResult::new(vec![index]));
        }

        self.continue_search()
    }

    /// continue search
    pub fn continue_search(&mut self) -> Result<SearchResult> {
        let mut req = Vec::new();
        for what in &self.wants {
            let package_index = self.index.as_ref().unwrap().get_uid(&what.uid)?;
            if package_index.index.is_none() {
                let download = DownloadRequest::new_package_index(&self.base_url, package_index);
                req.push(download);
                continue;
            }

            let version = package_index
                .index
                .as_ref()
                .unwrap()
                .find_version(&what.version)?;
            if version.manifest.is_none() {
                let download = DownloadRequest::new_package_manifest(
                    &self.base_url,
                    &package_index.uid,
                    version,
                );
                req.push(download);
                continue;
            }
        }

        Ok(SearchResult::new(req))
    }

    pub fn index_url(&self) -> String {
        format!("{}/index.json", self.base_url)
    }

    pub fn load_reader<R: Read>(&mut self, reader: &mut R, file_type: FileType) -> Result<()> {
        debug!("Loading {:?}", file_type);
        match file_type {
            FileType::MetaIndex => {
                let index = MetaIndex::from_reader(reader)?;
                trace!("loaded meta index");
                self.index = Some(index);

                Ok(())
            }
            FileType::Index => {
                let package = PackageIndex::from_reader(reader)?;
                trace!("loaded index: {}", package.uid);

                let index = self.index.as_mut().unwrap().get_uid_mut(&package.uid)?;
                index.index = Some(package);
                Ok(())
            }
            _ => todo!(),
        }
    }

    /// Create A new MetaManager.
    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "meta_manager_new"]
    pub unsafe extern "C" fn new_c(
        library_path: *const c_char,
        base_url: *const c_char,
    ) -> *mut Self {
        unsafe { Self::new_c_err(library_path, base_url) }
            .map(|c| Box::into_raw(Box::new(c)))
            .unwrap_or(core::ptr::null_mut())
    }

    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "meta_manager_free"]
    pub unsafe extern "C" fn free(v: *mut Self) {
        let _ = unsafe { Box::from_raw(v) };
    }

    #[cfg(feature = "ctypes")]
    unsafe fn new_c_err(library_path: *const c_char, base_url: *const c_char) -> Result<Self> {
        let library_path = unsafe { CStr::from_ptr(library_path) };
        let library_path = library_path.to_str()?;

        let base_url = unsafe { CStr::from_ptr(base_url) }.to_str()?;

        Ok(Self::new(library_path, base_url))
    }
}

pub struct Wants {
    pub uid: String,
    pub version: String,
    pub release_type: Option<String>,
}

impl Wants {
    pub fn new(uid: &str, version: &str) -> Self {
        Self {
            uid: uid.to_string(),
            version: version.to_string(),
            release_type: None,
        }
    }

    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "meta_wants_new"]
    pub unsafe extern "C" fn new_c(uid: *const c_char, version: *const c_char) -> *mut Self {
        unsafe { Self::new_c_err(uid, version) }
            .map(|c| Box::into_raw(Box::new(c)))
            .unwrap_or(core::ptr::null_mut())
    }

    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "meta_wants_free"]
    pub unsafe extern "C" fn free(v: *mut Self) {
        let _ = unsafe { Box::from_raw(v) };
    }

    #[cfg(feature = "ctypes")]
    unsafe fn new_c_err(uid: *const c_char, version: *const c_char) -> Result<Self> {
        let uid = unsafe { CStr::from_ptr(uid) }.to_str()?;
        let version = unsafe { CStr::from_ptr(version) }.to_str()?;

        Ok(Self::new(uid, version))
    }
}

pub struct SearchResult {
    pub requests: Vec<DownloadRequest>,
}

impl SearchResult {
    pub fn new(requests: Vec<DownloadRequest>) -> Self {
        Self { requests }
    }
}
