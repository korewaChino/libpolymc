#[derive(err_derive::Error, Debug)]
pub enum Error {
    #[error(display = "io: {}", _0)]
    Io(#[source] std::io::Error),

    #[error(display = "json: {}", _0)]
    Json(#[source] serde_json::Error),

    #[error(display = "Invalid library name")]
    InvalidLibraryName,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
