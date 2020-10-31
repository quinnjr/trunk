//! Entrypoint for the compression module.
//! 
//! *NOTE* Compression is only built into the `trunk` binary with the 'compression' feature enabled.

use serde::Deserialize;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub enum Compressor {
    // @TODO: Should deflate even be considered? Is Gzip just better?
    // #[serde(rename(deserialize = "deflate"))]
    // Deflate, // Hidden until worked on.
    #[serde(rename(deserialize = "gzip"))]
    Gzip,
    #[serde(rename(deserialize = "brotli"))]
    Brotli,
    #[serde(rename(deserialize = "zstd"))]
    Zstd,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompressorOptions {
    #[serde(default)]
    pub level: Option<usize>,
}

#[cfg(feature = "gzip-compression")]
mod gzip;
#[cfg(feature = "gzip-compression")]
pub use gzip::GzipCompressor;