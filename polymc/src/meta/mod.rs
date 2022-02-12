pub mod manifest;

#[repr(C)]
pub enum FileType {
    Index,
    Manifest,
    Library,
}
