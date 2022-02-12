use log::*;
use std::ffi::CStr;
use std::io::Read;
use std::os::raw::c_char;

use crate::{Error, Result};

mod index;
pub mod manifest;
mod request;

use crate::meta::manifest::{Manifest, Requirement};
pub use index::*;
pub use request::*;

pub struct MetaManager {
    pub library_path: String,
    pub base_url: String,
    wants: Vec<Wants>,
    extra_wants: Vec<Wants>,
    pub index: Option<MetaIndex>,
}

impl MetaManager {
    /// Create A new MetaManager.
    pub fn new(library_path: &str, base_url: &str) -> Self {
        Self {
            library_path: library_path.to_string(),
            base_url: base_url.to_string(),
            wants: Vec::new(),
            extra_wants: Vec::new(),
            index: None,
        }
    }

    pub fn search(&mut self, what: Wants) -> Result<()> {
        self.wants.push(what);

        Ok(())
    }

    /// continue search
    pub fn continue_search(&mut self) -> Result<SearchResult> {
        if self.wants.len() == 0 {
            return Err(Error::MetaNotFound);
        }

        if self.index.is_none() {
            let index = DownloadRequest::new_meta_index(self.index_url());
            return Ok(SearchResult::new(vec![index]));
        }

        let mut ret = Vec::new();

        for what in self.wants.clone() {
            let mut requires = self.search_for(&what)?;
            ret.append(&mut requires);
        }

        for what in self.extra_wants.clone() {
            let mut requires = self.search_for(&what)?;
            ret.append(&mut requires);
        }

        Ok(SearchResult::new(ret))
    }

    fn search_for(&mut self, what: &Wants) -> Result<Vec<DownloadRequest>> {
        let mut ret = Vec::new();

        let package_index = self.index.as_ref().unwrap().get_uid(&what.uid)?;
        if package_index.index.is_none() {
            let download = DownloadRequest::new_package_index(&self.base_url, package_index);
            ret.push(download);
            return Ok(ret);
        }

        let version = package_index
            .index
            .as_ref()
            .unwrap()
            .find_version(&what.version)?;

        self.extra_wants
            .append(&mut self.check_requirements(&version.requires));

        if version.manifest.is_none() {
            let download =
                DownloadRequest::new_package_manifest(&self.base_url, &package_index.uid, version);
            ret.push(download);
            return Ok(ret);
        }

        let manifest = version.manifest.as_ref().unwrap();

        self.extra_wants
            .append(&mut self.check_requirements(&manifest.requires));

        Ok(ret)
    }

    pub fn check_requirements(&self, reqs: &Vec<Requirement>) -> Vec<Wants> {
        let mut ret = Vec::new();

        for req in reqs {
            for wants in &self.wants {
                if wants.uid == req.uid {
                    break;
                }
            }
            for wants in &self.extra_wants {
                if wants.uid == req.uid {
                    break;
                }
            }
            trace!("adding {:?} to extra_wants", req);
            ret.push(req.clone().into())
        }

        ret
    }

    pub fn index_url(&self) -> String {
        format!("{}/index.json", self.base_url)
    }

    /// The user has to ensure the hash does match
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
            FileType::Manifest => {
                let manifest = Manifest::from_reader(reader)?;
                trace!("loaded manifest: {}", manifest.name);
                let index = self.index.as_mut().unwrap().get_uid_mut(&manifest.uid)?;
                let package = index
                    .index
                    .as_mut()
                    .unwrap()
                    .find_version_mut(&manifest.version)?;
                package.manifest = Some(manifest);

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

#[derive(Debug, Clone)]
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

impl From<Requirement> for Wants {
    fn from(req: Requirement) -> Self {
        Self {
            uid: req.uid,
            version: req.suggests,
            release_type: None,
        }
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
