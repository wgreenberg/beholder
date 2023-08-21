use thiserror::Error;
use lz4_flex::block::DecompressError as Lz4Error;
use flate2::DecompressError as ZlibError;

pub type UnpackResult<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Deku error: {0}")]
    DekuError(#[from] deku::DekuError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("LZ4 error: {0}")]
    Lz4Error(#[from] Lz4Error),
    #[error("Zlib error: {0}")]
    ZlibError(#[from] ZlibError),
}
