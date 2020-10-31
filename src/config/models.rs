use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use http_types::Url;
use serde::Deserialize;
use structopt::StructOpt;

use crate::common::parse_public_url;
use crate::config::{RtcBuild, RtcClean, RtcServe, RtcWatch};

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, StructOpt)]
pub struct ConfigOptsBuild {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[structopt(parse(from_os_str))]
    pub target: Option<PathBuf>,
    /// Build in release mode [default: false]
    #[structopt(long)]
    #[serde(default)]
    pub release: bool,
    /// The output dir for all final assets [default: dist]
    #[structopt(short, long, parse(from_os_str))]
    pub dist: Option<PathBuf>,
    /// The public URL from which assets are to be served [default: /]
    #[structopt(long, parse(from_str=parse_public_url))]
    pub public_url: Option<String>,
}

/// Config options for the watch system.
#[derive(Clone, Debug, Default, Deserialize, StructOpt)]
pub struct ConfigOptsWatch {
    /// Additional paths to ignore [default: []]
    #[structopt(short, long, parse(from_os_str))]
    pub ignore: Option<Vec<PathBuf>>,
}

/// Config options for the serve system.
#[derive(Clone, Debug, Default, Deserialize, StructOpt)]
pub struct ConfigOptsServe {
    /// The port to serve on [default: 8080]
    #[structopt(long)]
    pub port: Option<u16>,
    /// Open a browser tab once the initial build is complete [default: false]
    #[structopt(long)]
    #[serde(default)]
    pub open: bool,
    /// A URL to which requests will be proxied [default: None]
    #[structopt(long = "proxy-backend")]
    #[serde(default)]
    pub proxy_backend: Option<Url>,
    /// The URI on which to accept requests which are to be rewritten and proxied to backend
    /// [default: None]
    #[structopt(long = "proxy-rewrite")]
    #[serde(default)]
    pub proxy_rewrite: Option<String>,
}

/// Config options for the serve system.
#[derive(Clone, Debug, Default, Deserialize, StructOpt)]
pub struct ConfigOptsClean {
    /// The output dir for all final assets [default: dist]
    #[structopt(short, long, parse(from_os_str))]
    pub dist: Option<PathBuf>,
    /// Optionally perform a cargo clean [default: false]
    #[structopt(long)]
    #[serde(default)]
    pub cargo: bool,
}

/// Config options for building proxies.
///
/// NOTE WELL: this configuration type is different from the others inasmuch as it is only used
/// when parsing the `Trunk.toml` config file. It is not intended to be configured via CLI or env
/// vars.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigOptsProxy {
    /// The URL of the backend to which requests are to be proxied.
    pub backend: Url,
    /// An optional URI prefix which is to be used as the base URI for proxying requests, which
    /// defaults to the URI of the backend.
    ///
    /// When a value is specified, requests received on this URI will have this URI segment replaced
    /// with the URI of the `backend`.
    pub rewrite: Option<String>,
}

/// A model of all potential configuration options for the Trunk CLI system.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ConfigOpts {
    pub build: Option<ConfigOptsBuild>,
    pub watch: Option<ConfigOptsWatch>,
    pub serve: Option<ConfigOptsServe>,
    pub clean: Option<ConfigOptsClean>,
    pub proxy: Option<Vec<ConfigOptsProxy>>,
    #[cfg(feature = "compression")]
    pub compression: Option<Vec<ConfigOptsCompression>>,
}

impl ConfigOpts {
    /// Extract the runtime config for the build system based on all config layers.
    pub async fn rtc_build(cli_build: ConfigOptsBuild, config: Option<PathBuf>) -> Result<Arc<RtcBuild>> {
        let base_layer = Self::file_and_env_layers(config)?;
        let build_layer = Self::cli_opts_layer_build(cli_build, base_layer);
        let build_opts = build_layer.build.unwrap_or_default();
        Ok(Arc::new(RtcBuild::new(build_opts)?))
    }

    /// Extract the runtime config for the watch system based on all config layers.
    pub async fn rtc_watch(cli_build: ConfigOptsBuild, cli_watch: ConfigOptsWatch, config: Option<PathBuf>) -> Result<Arc<RtcWatch>> {
        let base_layer = Self::file_and_env_layers(config)?;
        let build_layer = Self::cli_opts_layer_build(cli_build, base_layer);
        let watch_layer = Self::cli_opts_layer_watch(cli_watch, build_layer);
        let build_opts = watch_layer.build.unwrap_or_default();
        let watch_opts = watch_layer.watch.unwrap_or_default();
        Ok(Arc::new(RtcWatch::new(build_opts, watch_opts)?))
    }

    /// Extract the runtime config for the serve system based on all config layers.
    pub async fn rtc_serve(
        cli_build: ConfigOptsBuild, cli_watch: ConfigOptsWatch, cli_serve: ConfigOptsServe, config: Option<PathBuf>,
    ) -> Result<Arc<RtcServe>> {
        let base_layer = Self::file_and_env_layers(config)?;
        let build_layer = Self::cli_opts_layer_build(cli_build, base_layer);
        let watch_layer = Self::cli_opts_layer_watch(cli_watch, build_layer);
        let serve_layer = Self::cli_opts_layer_serve(cli_serve, watch_layer);
        let build_opts = serve_layer.build.unwrap_or_default();
        let watch_opts = serve_layer.watch.unwrap_or_default();
        let serve_opts = serve_layer.serve.unwrap_or_default();
        Ok(Arc::new(RtcServe::new(build_opts, watch_opts, serve_opts, serve_layer.proxy)?))
    }

    /// Extract the runtime config for the clean system based on all config layers.
    pub async fn rtc_clean(cli_clean: ConfigOptsClean, config: Option<PathBuf>) -> Result<Arc<RtcClean>> {
        let base_layer = Self::file_and_env_layers(config)?;
        let clean_layer = Self::cli_opts_layer_clean(cli_clean, base_layer);
        let clean_opts = clean_layer.clean.unwrap_or_default();
        Ok(Arc::new(RtcClean::new(clean_opts)?))
    }

    /// Return the full configuration based on config file & environment variables.
    pub async fn full(config: Option<PathBuf>) -> Result<Self> {
        Self::file_and_env_layers(config)
    }

    fn cli_opts_layer_build(cli: ConfigOptsBuild, cfg_base: Self) -> Self {
        let opts = ConfigOptsBuild {
            target: cli.target,
            release: cli.release,
            dist: cli.dist,
            public_url: cli.public_url,
        };
        let cfg_build = ConfigOpts {
            build: Some(opts),
            watch: None,
            serve: None,
            clean: None,
            proxy: None,
            #[cfg(feature = "compression")]
            compression: None,
        };
        Self::merge(cfg_base, cfg_build)
    }

    fn cli_opts_layer_watch(cli: ConfigOptsWatch, cfg_base: Self) -> Self {
        let opts = ConfigOptsWatch { ignore: cli.ignore };
        let cfg = ConfigOpts {
            build: None,
            watch: Some(opts),
            serve: None,
            clean: None,
            proxy: None,
            #[cfg(feature = "compression")]
            compression: None,
        };
        Self::merge(cfg_base, cfg)
    }

    fn cli_opts_layer_serve(cli: ConfigOptsServe, cfg_base: Self) -> Self {
        let opts = ConfigOptsServe {
            port: cli.port,
            open: cli.open,
            proxy_backend: cli.proxy_backend,
            proxy_rewrite: cli.proxy_rewrite,
        };
        let cfg = ConfigOpts {
            build: None,
            watch: None,
            serve: Some(opts),
            clean: None,
            proxy: None,
            #[cfg(feature = "compression")]
            compression: None,
        };
        Self::merge(cfg_base, cfg)
    }

    fn cli_opts_layer_clean(cli: ConfigOptsClean, cfg_base: Self) -> Self {
        let opts = ConfigOptsClean {
            dist: cli.dist,
            cargo: cli.cargo,
        };
        let cfg = ConfigOpts {
            build: None,
            watch: None,
            serve: None,
            clean: Some(opts),
            proxy: None,
            #[cfg(feature = "compression")]
            compression: None,
        };
        Self::merge(cfg_base, cfg)
    }

    fn file_and_env_layers(path: Option<PathBuf>) -> Result<Self> {
        let toml_cfg = Self::from_file(path)?;
        let env_cfg = Self::from_env().context("error reading trunk env var config")?;
        let cfg = Self::merge(toml_cfg, env_cfg);
        Ok(cfg)
    }

    /// Read runtime config from a `Trunk.toml` file at the target path.
    ///
    /// NOTE WELL: any paths specified in a Trunk.toml file must be interpreted as being relative
    /// to the file itself.
    fn from_file(path: Option<PathBuf>) -> Result<Self> {
        let mut path = path.unwrap_or_else(|| "Trunk.toml".into());
        if !path.exists() {
            return Ok(Default::default());
        }
        if !path.is_absolute() {
            path = path
                .canonicalize()
                .with_context(|| format!("error getting canonical path to Trunk config file {:?}", &path))?;
        }
        let cfg_bytes = std::fs::read(&path).context("error reading config file")?;
        let mut cfg: Self = toml::from_slice(&cfg_bytes).context("error reading config file contents as TOML data")?;
        if let Some(parent) = path.parent() {
            cfg.build.iter_mut().for_each(|build| {
                build.target.iter_mut().for_each(|target| {
                    if !target.is_absolute() {
                        *target = parent.join(&target);
                    }
                });
                build.dist.iter_mut().for_each(|dist| {
                    if !dist.is_absolute() {
                        *dist = parent.join(&dist);
                    }
                });
            });
            cfg.watch.iter_mut().for_each(|watch| {
                watch.ignore.iter_mut().for_each(|ignores_vec| {
                    ignores_vec.iter_mut().for_each(|ignore_path| {
                        if !ignore_path.is_absolute() {
                            *ignore_path = parent.join(&ignore_path);
                        }
                    });
                });
            });
            cfg.clean.iter_mut().for_each(|clean| {
                clean.dist.iter_mut().for_each(|dist| {
                    if !dist.is_absolute() {
                        *dist = parent.join(&dist);
                    }
                });
            });
        }
        Ok(cfg)
    }

    fn from_env() -> Result<Self> {
        let build: ConfigOptsBuild = envy::prefixed("TRUNK_BUILD_").from_env()?;
        let watch: ConfigOptsWatch = envy::prefixed("TRUNK_WATCH_").from_env()?;
        let serve: ConfigOptsServe = envy::prefixed("TRUNK_SERVE_").from_env()?;
        let clean: ConfigOptsClean = envy::prefixed("TRUNK_CLEAN_").from_env()?;
        Ok(ConfigOpts {
            build: Some(build),
            watch: Some(watch),
            serve: Some(serve),
            clean: Some(clean),
            proxy: None,
            #[cfg(feature = "compression")]
            compression: None, //@TODO: add environment options?
        })
    }

    /// Merge the given layers, where the `greater` layer takes precedence.
    #[cfg(not(feature = "compression"))]
    fn merge(mut lesser: Self, mut greater: Self) -> Self {
        greater.build = match (lesser.build.take(), greater.build.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(l), Some(mut g)) => {
                g.target = g.target.or(l.target);
                g.dist = g.dist.or(l.dist);
                g.public_url = g.public_url.or(l.public_url);
                // NOTE: this can not be disabled in the cascade.
                if l.release {
                    g.release = true
                }
                Some(g)
            }
        };
        greater.watch = match (lesser.watch.take(), greater.watch.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(l), Some(mut g)) => {
                g.ignore = g.ignore.or(l.ignore);
                Some(g)
            }
        };
        greater.serve = match (lesser.serve.take(), greater.serve.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(l), Some(mut g)) => {
                g.proxy_backend = g.proxy_backend.or(l.proxy_backend);
                g.proxy_rewrite = g.proxy_rewrite.or(l.proxy_rewrite);
                g.port = g.port.or(l.port);
                // NOTE: this can not be disabled in the cascade.
                if l.open {
                    g.open = true
                }
                Some(g)
            }
        };
        greater.clean = match (lesser.clean.take(), greater.clean.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(l), Some(mut g)) => {
                g.dist = g.dist.or(l.dist);
                // NOTE: this can not be disabled in the cascade.
                if l.cargo {
                    g.cargo = true
                }
                Some(g)
            }
        };
        greater.proxy = match (lesser.proxy.take(), greater.proxy.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(_), Some(g)) => Some(g), // No meshing/merging. Only take the greater value.
        };
        greater
    }

    /// Merge the given layers, where the `greater` layer takes precedence.
    #[cfg(feature = "compression")]
    fn merge(mut lesser: Self, mut greater: Self) -> Self {
        greater.build = match (lesser.build.take(), greater.build.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(l), Some(mut g)) => {
                g.target = g.target.or(l.target);
                g.dist = g.dist.or(l.dist);
                g.public_url = g.public_url.or(l.public_url);
                // NOTE: this can not be disabled in the cascade.
                if l.release {
                    g.release = true
                }
                Some(g)
            }
        };
        greater.watch = match (lesser.watch.take(), greater.watch.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(l), Some(mut g)) => {
                g.ignore = g.ignore.or(l.ignore);
                Some(g)
            }
        };
        greater.serve = match (lesser.serve.take(), greater.serve.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(l), Some(mut g)) => {
                g.proxy_backend = g.proxy_backend.or(l.proxy_backend);
                g.proxy_rewrite = g.proxy_rewrite.or(l.proxy_rewrite);
                g.port = g.port.or(l.port);
                // NOTE: this can not be disabled in the cascade.
                if l.open {
                    g.open = true
                }
                Some(g)
            }
        };
        greater.clean = match (lesser.clean.take(), greater.clean.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(l), Some(mut g)) => {
                g.dist = g.dist.or(l.dist);
                // NOTE: this can not be disabled in the cascade.
                if l.cargo {
                    g.cargo = true
                }
                Some(g)
            }
        };
        greater.proxy = match (lesser.proxy.take(), greater.proxy.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(_), Some(g)) => Some(g), // No meshing/merging. Only take the greater value.
        };
        greater.compression = match(lesser.compression.take(), greater.compression.take()) {
            (None, None) => None,
            (Some(val), None) | (None, Some(val)) => Some(val),
            (Some(_), Some(g)) => Some(g),
        };
        greater
    }
}

#[cfg(feature = "compression")]
use crate::compression::{Compressor, CompressorOptions};

/// Struct for deserialized compression configuration.
/// 
/// **NOTE**
/// This struct is gated by the "compression" feature. `trunk` must be compiled with the feature enabled for this config to be used.
/// 
/// Ex: For enabling gzip compression.
///     ```sh
///     cargo install trunk --features gzip-compression
///     ```
#[cfg(feature = "compression")]
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigOptsCompression {
    /// Specifies the compression algorithm. A valid algorithm _must_ be specified.
    pub algorithm: Compressor,
    /// Specifies options to be passed to the compression algorithm. Optional.
    /// @TODO: Ensure that multiple compression algorithms can use the same `options` field.
    #[serde(default)]
    pub options: Option<CompressorOptions>,
    /// A RegExp test used to include/exclude assets for compression. Optional.
    #[serde(default)]
    pub test: Option<String>,
    /// Allow for inclusion of certain assets. Optional.
    /// @TODO: Figure out how to actually do this with minimal overhead.
    #[serde(default)]
    pub include: Option<Vec<String>>,
    /// Allow for exclusion of certain assets. Optional.
    /// @TODO: Figure out how to actually do this with minimal overhead.
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
    /// Size of assets (in bytes) that should be compressed. Optional.
    #[serde(default)]
    threshold: Option<usize>,
    /// Minimum compression ratio to original file size to emit.
    /// Asset compression below the desired ratio will not emit a resultant compressed file.
    /// Optional.
    #[serde(default)]
    ratio: Option<f32>,
}

#[cfg(test)]
mod tests {

    use super::ConfigOpts;
    use std::error::Error;
    #[cfg(feature = "compression")]
    use crate::compression::{Compressor, CompressorOptions};

    #[test]
    #[cfg_attr(not(feature = "gzip-compression"), ignore)]
    fn deserialize_config_opts_compression_gzip() -> Result<(), Box<dyn Error>> {
        let input = r#"
            [[compression]]
            algorithm = "gzip"
            options = { level = 9 }
        "#;

        let config: ConfigOpts = toml::from_str(&input)?;

        assert!(config.compression.is_some());

        if let Some(compressors) = config.compression {
            assert_eq!(compressors.len(), 1);
            for compressor in compressors {
                assert_eq!(compressor.algorithm, Compressor::Gzip);
                assert!(compressor.options.is_some());
                let options: CompressorOptions = compressor.options.unwrap();
                assert!(options.level.is_some());
                assert_eq!(options.level.unwrap(), 9);
            }
            Ok(())
        } else {
            Err(Box::from("Should have been a valid compression configuration"))
        }
    }

    #[test]
    #[cfg_attr(not(feature = "compression"), ignore)]
    fn deserialize_config_opts_compression_none() -> Result<(), Box<dyn Error>> {
        let input = r#"
        [build]
        target = "src/index.html"
        dist = "dist"
        public_url = "/assets/"

        [serve]
        port = 9000
        "#;

        let config: ConfigOpts = toml::from_str(&input)?;

        assert!(config.compression.is_none());

        Ok(())
    }

    #[test]
    #[cfg_attr(not(feature = "compression"), ignore)]
    #[should_panic]
    fn invalid_config_opts_compression() {
        let input = r#"
            [[compression]]
            ratio = 0.8
        "#;

        let _config: ConfigOpts = toml::from_str(&input)
            .expect("Should not have constructed a valid compression config");
    }
}