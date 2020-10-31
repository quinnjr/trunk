//! Gzip asset compression pipeline.
//! 
//! Requires that the `gzip-compress` feature be enabled.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_std::fs;
use futures::channel::mpsc::Sender;
use indicatif::ProgressBar;
use regex::Regex;

use crate::common::{BUILDING, ERROR, SUCCESS};
use crate::config::RtcBuild;
use crate::pipelines::AssetFile;


/// The Gzip asset compressor.
pub struct GzipCompressor {
    /// Regex Test to perform on the assets.
    pub regex: Regex,
    /// Files to process.
    pub file: Vec<AssetFile>,

}

impl GzipCompressor {
    async new() -> Self {
        
    }

    async filter_assets(&self) {
        
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
}