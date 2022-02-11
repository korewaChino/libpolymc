#[derive(err_derive::Error, Debug)]
pub enum Error {
    #[error(display = "io: {}", _0)]
    Io(#[source] std::io::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
