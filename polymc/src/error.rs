#[derive(err_derive::Error, Debug)]
pub enum Error {
    #[error(display = "io: {}", _0)]
    Io(#[source] std::io::Error),

    #[error(display = "json: {}", _0)]
    Json(#[source] serde_json::Error),

    #[error(display = "hex: {}", _0)]
    FromHex(#[source] hex::FromHexError),

    #[error(display = "utf8: {}", _0)]
    FromUtf8(#[source] std::str::Utf8Error),

    #[error(display = "Invalid library name")]
    LibraryInvalidName,

    #[error(display = "Library not supported on the current platform")]
    LibraryNotSupported,

    #[error(display = "Library is missing")]
    LibraryMissing,

    #[error(display = "Library has invalid hash")]
    LibraryInvalidHash,

    #[error(display = "Meta data not found for requested search")]
    MetaNotFound,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;