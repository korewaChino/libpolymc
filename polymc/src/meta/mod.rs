use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::raw::c_char;

#[cfg(all(feature = "ctypes", target_family = "unix"))]
use std::os::unix::io::{FromRawFd, RawFd};

#[cfg(all(feature = "ctypes", target_family = "windows"))]
use std::os::windows::io::{FromRawHandle, RawHandle};

use libc::c_int;
use log::*;

use crate::{Error, Result};

mod index;
pub mod manifest;
mod request;

use crate::meta::manifest::{Manifest, Requirement, OS};
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
        if self.wants.is_empty() {
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

        let os = OS::get();
        let verify_result = unsafe { manifest.verify_caching_at(&self.library_path, &os)? };
        for (lib, _error) in &verify_result {
            let at = lib.path_at_for(&self.library_path, &os);
            ret.push(DownloadRequest::from_library(
                lib.select_for(&os).ok_or(Error::MetaNotFound)?.clone(),
                at,
            ))
        }

        Ok(ret)
    }

    pub fn check_requirements(&self, reqs: &[Requirement]) -> Vec<Wants> {
        let mut ret = Vec::new();

        for req in reqs {
            for wants in &self.wants {
                if wants.uid == req.uid {
                    return ret;
                }
            }
            for wants in &self.extra_wants {
                if wants.uid == req.uid {
                    return ret;
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

    pub fn load_meta_index(&mut self, index: MetaIndex) -> Result<()> {
        trace!("loaded meta index");
        self.index = Some(index);
        Ok(())
    }

    pub fn load_index(&mut self, package: PackageIndex) -> Result<()> {
        trace!("loaded index: {}", package.uid);

        let index = self
            .index
            .as_mut()
            .ok_or(Error::MetaNotFound)?
            .get_uid_mut(&package.uid)?;

        index.index = Some(package);

        Ok(())
    }

    pub fn load_manifest(&mut self, manifest: Manifest) -> Result<()> {
        trace!("loaded manifest: {}", manifest.name);
        let index = self
            .index
            .as_mut()
            .ok_or(Error::MetaNotFound)?
            .get_uid_mut(&manifest.uid)?;
        let package = index
            .index
            .as_mut()
            .ok_or(Error::MetaNotFound)?
            .find_version_mut(&manifest.version)?;

        package.manifest = Some(manifest);

        Ok(())
    }

    pub fn load(&mut self, data: &str, file_type: FileType) -> Result<()> {
        debug!("Loading(str) {:?}", file_type);
        match file_type {
            FileType::MetaIndex => {
                let index = data.parse()?;
                self.load_meta_index(index)
            }
            FileType::Index => {
                let package = data.parse()?;
                self.load_index(package)
            }
            FileType::Manifest => {
                let manifest = data.parse()?;
                self.load_manifest(manifest)
            }
            _ => Err(Error::MetaNotFound),
        }
    }

    /// The user has to ensure the hash does match
    pub fn load_reader<R: Read>(&mut self, reader: &mut R, file_type: FileType) -> Result<()> {
        debug!("Loading {:?}", file_type);
        match file_type {
            FileType::MetaIndex => {
                let index = MetaIndex::from_reader(reader)?;
                self.load_meta_index(index)
            }
            FileType::Index => {
                let package = PackageIndex::from_reader(reader)?;
                self.load_index(package)
            }
            FileType::Manifest => {
                let manifest = Manifest::from_reader(reader)?;
                self.load_manifest(manifest)
            }
            _ => Err(Error::MetaNotFound),
        }
    }

    pub fn load_data(&mut self, data: &[u8], file_type: FileType) -> Result<()> {
        debug!("Loading(data) {:?}", file_type);
        match file_type {
            FileType::MetaIndex => {
                let index = MetaIndex::from_data(data)?;
                self.load_meta_index(index)
            }
            FileType::Index => {
                let package = PackageIndex::from_data(data)?;
                self.load_index(package)
            }
            FileType::Manifest => {
                let manifest = Manifest::from_data(data)?;
                self.load_manifest(manifest)
            }
            _ => Err(Error::MetaNotFound),
        }
    }

    pub fn load_file(&mut self, file: &str, file_type: FileType) -> Result<()> {
        debug!("Loading file {file} for type {file_type}");
        let mut file = OpenOptions::new().read(true).open(file)?;

        self.load_reader(&mut file, file_type)
    }

    /// Load file into MetaManager.
    ///
    /// # Safety
    /// file has to be a valid CStr pointing to a file.
    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "meta_manager_load_file"]
    pub unsafe extern "C" fn load_file_c(
        &mut self,
        file: *const c_char,
        file_type: FileType,
    ) -> c_int {
        let file = unsafe { CStr::from_ptr(file) }.to_str();
        if file.is_err() {
            return -libc::EINVAL;
        }

        if let Err(e) = self.load_file(file.unwrap(), file_type) {
            -e.as_c_error()
        } else {
            0
        }
    }

    /// Load file into MetaManager.
    ///
    /// # Safety
    /// fd has to be a valid fd.
    #[cfg(all(feature = "ctypes", target_family = "unix"))]
    #[doc(hidden)]
    #[export_name = "meta_manager_load_fd"]
    pub unsafe extern "C" fn load_fd(&mut self, fd: RawFd, file_type: FileType) -> c_int {
        let mut file = unsafe { File::from_raw_fd(fd) };

        if let Err(e) = self.load_reader(&mut file, file_type) {
            -e.as_c_error()
        } else {
            0
        }
    }

    /// Load file into MetaManager.
    ///
    /// # Safety
    /// Handle has to be a valid file handle.
    #[cfg(all(feature = "ctypes", target_family = "windows"))]
    #[export_name = "meta_manager_load_handle"]
    pub unsafe extern "C" fn load_handle(
        &mut self,
        handle: RawHandle,
        file_type: FileType,
    ) -> c_int {
        let mut file = unsafe { File::from_raw_handle(handle) };

        if let Err(e) = self.load_reader(&mut file, file_type) {
            -e.as_c_error()
        } else {
            0
        }
    }

    /// Load string into MetaManager
    ///
    /// # Safety
    /// Data has to be a valid pointer to a string holding the json of the type *file_type*.
    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "meta_manager_load"]
    pub unsafe extern "C" fn load_c(&mut self, data: *const c_char, file_type: FileType) -> c_int {
        let data = unsafe { CStr::from_ptr(data) }.to_str();
        if data.is_err() {
            return -libc::EINVAL;
        }

        if let Err(e) = self.load(data.unwrap(), file_type) {
            -e.as_c_error()
        } else {
            0
        }
    }

    /// Load string into MetaManager
    ///
    /// # Safety
    /// Data has to be a valid pointer valid for *len* holding the json of the type *file_type*.
    #[cfg(feature = "ctypes")]
    #[doc(hidden)]
    #[export_name = "meta_manager_load_data"]
    pub unsafe extern "C" fn load_data_c(
        &mut self,
        data: *const c_char,
        len: usize,
        file_type: FileType,
    ) -> c_int {
        let data = unsafe { std::slice::from_raw_parts(data as *const u8, len) };

        if let Err(e) = self.load_data(data, file_type) {
            -e.as_c_error()
        } else {
            0
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
